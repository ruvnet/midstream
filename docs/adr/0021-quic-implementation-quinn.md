# 0021 — QUIC implementation: `quinn` with optional `s2n-quic` feature

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** transport, quic, rationale

## Context and Problem Statement

`crates/quic-multistream/Cargo.toml:18` pulls in `quinn = "0.11"`.
There is no documented rationale for this choice, no comparison to the
two other production-grade Rust QUIC implementations, and no exit
ramp if quinn becomes unsuitable.

The Rust QUIC landscape today:

| Implementation | Maintainer        | TLS backend                    | Async runtime | Notes |
|----------------|-------------------|--------------------------------|---------------|-------|
| `quinn`        | quinn-rs org      | `rustls` (0.23 via quinn 0.11) | tokio-native  | Most-used in the ecosystem; the QUIC bench file already uses it. |
| `s2n-quic`     | AWS               | `s2n-tls` (or `rustls` via feature) | tokio + runtime-agnostic option | Strong AWS production track record; rich connection-level metrics. |
| `quiche`       | Cloudflare        | BoringSSL (C dep)              | runtime-agnostic | C linkage; not idiomatic Rust; used by Cloudflare's edge. |

We have no record of why quinn was chosen over the others, and the
crate already exposes a `wasm` shim (`crates/quic-multistream/src/wasm.rs`)
that further constrains the choice (s2n-quic is harder to feature-gate
for `wasm32-unknown-unknown` than quinn is).

Additionally, the version of `rustls` quinn pulls in (`0.22.4`) does
**not** match the `rustls 0.23.40` that the rest of the workspace
pulls in (see [ADR-0014](0014-supply-chain-pinning.md) for the
duplicate-version finding) — exactly the kind of friction that an
"optional backend" feature flag tends to make worse.

## Decision Drivers

- **One default; one backup.** Default to one implementation so that
  consumers get a working build with no thought; allow one alternative
  behind a feature flag for users with operational constraints.
- **`rustls`-everywhere.** The rest of the workspace standardises on
  `rustls` (see [ADR-0011](0011-quic-tls-verification.md),
  [ADR-0014](0014-supply-chain-pinning.md)). The QUIC TLS backend must
  match — no native-tls, no BoringSSL, no `s2n-tls`-only variants.
- **WASM compatibility.** A subset of the QUIC crate must build for
  `wasm32-unknown-unknown` to support the WASM bindings
  ([ADR-0003](0003-wasm-consolidation.md)). The chosen primary
  backend must have a `cfg(not(target_arch = "wasm32"))` story; the
  alternative may not.
- **Ecosystem mass.** Bug fixes, examples, and integration with
  tokio's `tokio-tls`/`hyper-quic`/`reqwest`'s emerging HTTP/3
  support matter more than micro-bench wins.

## Considered Options

1. **Status quo: `quinn` only, no documented rationale.** What we have.
2. **`quinn` default + documented `s2n-quic` feature flag.** Adds an
   AWS-friendly backend without changing the default consumer
   experience.
3. **`s2n-quic` default + `quinn` feature flag.** Better for AWS-
   centric deployments; loses WASM and the larger ecosystem tail.
4. **Both backends always linked, runtime-selectable.** Pays the
   compile-time cost of both forever.
5. **Replace `quinn` with `quiche`.** Cloudflare-grade; brings BoringSSL
   C dep and doesn't fit our rustls posture.

## Decision Outcome

**Chosen option: Option 2 — `quinn` as the default; `s2n-quic`
available behind a feature flag `backend-s2n-quic`.**

`quinn` stays the default because:

- It is rustls-native, matching [ADR-0011](0011-quic-tls-verification.md)
  and [ADR-0014](0014-supply-chain-pinning.md).
- The existing `wasm.rs` shim assumes a quinn-style API.
- Ecosystem integrations (`hyper-quic`, `reqwest`'s HTTP/3 preview,
  examples, IDE tooling) target quinn first.

`s2n-quic` behind a feature flag because:

- It has stronger built-in metrics (per-connection RTT histograms,
  loss counters) that an AWS-shaped deployment may prefer.
- It interoperates better with the AWS connection-tracing tools.
- The feature flag forces the cost (duplicate compile, extra deps)
  onto users who actually opt in.

Both backends must use rustls. `s2n-quic` is enabled with its
`provider-tls-rustls` feature; native `s2n-tls` is explicitly
forbidden by our policy.

### Positive consequences

- The QUIC backend choice is documented; reviewers can revisit it.
- Default builds stay rustls-native, quinn-only, tiny on compile.
- Operational alternatives exist for users who need them, opt-in.

### Negative consequences

- The `crates/quic-multistream` API must abstract over both backends
  if `backend-s2n-quic` ships. Concretely: a `Connection` trait
  internal to the crate, with `quinn::Connection` and
  `s2n_quic::Connection` impls. Public API stays backend-agnostic.
  This is non-trivial; some perf-sensitive APIs (zero-copy
  `bytes::Bytes` paths) differ between backends.
- We must run CI matrix entries against both backends on every PR;
  CI minutes increase modestly.

## Implementation notes

- Document the rationale at the top of
  `crates/quic-multistream/src/lib.rs` (doc-comment, not a separate
  file).
- Add to `crates/quic-multistream/Cargo.toml`:

  ```toml
  [features]
  default = ["backend-quinn"]
  backend-quinn = ["dep:quinn"]
  backend-s2n-quic = ["dep:s2n-quic", "s2n-quic/provider-tls-rustls"]
  ```

- The crate must error at compile time if neither backend is enabled
  (`compile_error!("enable one of: backend-quinn, backend-s2n-quic")`).
- Multiple backends enabled simultaneously is supported; runtime
  selects via a `QuicBackend` enum at endpoint construction.
- The `wasm.rs` shim stays under `#[cfg(target_arch = "wasm32")]` and
  is backend-agnostic (it talks to the browser's `fetch`/WebTransport
  surface, not to quinn or s2n-quic directly).

## Links

- Related: [ADR-0011](0011-quic-tls-verification.md),
  [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0003](0003-wasm-consolidation.md).
- `quinn`: https://docs.rs/quinn/
- `s2n-quic`: https://github.com/aws/s2n-quic
