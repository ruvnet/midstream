# 0011 — QUIC TLS verification: secure-by-default, no `SkipServerVerification`

- **Status:** Accepted (implemented in #8)
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** security, transport, tls

## Context and Problem Statement

`crates/quic-multistream/src/native.rs:39-43` unconditionally installs
a `SkipServerVerification` certificate verifier (defined at `:227-280`)
on **every** `QuicConnection::connect`:

```rust
let mut crypto = rustls::ClientConfig::builder()
    .with_safe_defaults()
    .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
    .with_no_client_auth();
```

The accompanying comment says "for demo purposes" but:

- There is no `#[cfg(test)]`.
- There is no feature gate (`insecure-dev-only` or otherwise).
- There is no env-var override.
- There is no runtime warning emitted at connect time.

Any caller — including AIMDS, when it's wired through the workspace
(see [ADR-0004](0004-aimds-workspace-member.md)) — gets a QUIC client
that accepts any certificate from any peer. This is the most serious
finding in the repo: a published crate
(`midstreamer-quic-multistream`) ships with TLS verification disabled
by default. Anyone shipping a downstream binary inherits a
man-in-the-middle vulnerability.

The bench file `benches/quic_bench.rs:36-49` self-signs certs
server-side, which is fine for benches, but the production `connect()`
path inherits the skip-verify behaviour.

## Decision Drivers

- **Secure-by-default.** The library must refuse to accept invalid
  certificates *by default*. Insecure modes are opt-in, never opt-out.
- **No hidden footguns.** A reviewer skimming `Cargo.toml` features
  should be able to tell whether a binary trusts the world.
- **Bench/test compatibility.** Local benches with self-signed certs
  must keep working — but only when the consumer explicitly says so.

## Considered Options

1. **Status quo.** Skip verification by default. Document it harder.
2. **Default to platform CA roots via `rustls-native-certs`**, with an
   optional cargo feature `insecure-dev-only-skip-verification` that
   restores the current behaviour. Feature is mutually exclusive with
   `--release`-style consumer builds (enforced via `#[cfg]` gating).
3. **Default to platform CA roots via `webpki-roots`** (Mozilla CA
   bundle baked in). Same opt-in insecure feature. Webpki-roots is
   simpler but doesn't pick up enterprise CAs.
4. **Default to `rustls-platform-verifier`** (uses OS-native trust
   store APIs, picks up enterprise CAs, matches OS behaviour). Newest;
   adds a tokio-rustls dependency.

## Decision Outcome

**Chosen option: Option 4 — `rustls-platform-verifier` as the default,
plus an `insecure-dev-only` cargo feature for bench/test consumers.**
`rustls-platform-verifier` honours OS trust policies (Mac Keychain,
Windows cert store, system `ca-certificates`) so enterprise
deployments don't need a custom CA bundle baked in.

The insecure-mode feature exists, but:

- It is named `insecure-dev-only-skip-server-verification` (verbose on
  purpose).
- Enabling it emits a `tracing::warn!` at every connect.
- It triggers a `#[deprecated(note = "TLS verification disabled.
  Never enable for production.")]` warning on the feature shim.
- CI explicitly tests the negative path: a default-build `connect`
  against a self-signed cert must **fail**.

The standalone `SkipServerVerification` impl is moved into
`#[cfg(feature = "insecure-dev-only-skip-server-verification")]`-gated
module so it does not exist in default builds.

### Positive consequences

- Default `cargo build` of any consumer of `midstreamer-quic-multistream`
  produces a TLS-verifying client.
- The "insecure" mode is opt-in at the dependency level, surfaces in
  `cargo tree --features`, and is auditable via `cargo deny`.
- Enterprise CAs work without bundle updates because we defer to the OS.

### Negative consequences

- Self-signed-cert benches (`benches/quic_bench.rs`) must either
  (a) enable the insecure feature explicitly, or (b) install the test
  CA into a temporary OS trust store, which is fiddly on CI.
  We pick (a): benches set
  `[dependencies.midstreamer-quic-multistream]
  features = ["insecure-dev-only-skip-server-verification"]`.
- One new dep (`rustls-platform-verifier`). Already audited.

## Implementation notes

- `crates/quic-multistream/Cargo.toml` adds:

  ```toml
  [features]
  default = []
  insecure-dev-only-skip-server-verification = []

  [dependencies]
  rustls = "0.23"
  rustls-platform-verifier = "0.5"
  ```

- `crates/quic-multistream/src/native.rs:39-43` rewritten: default
  builds use `rustls_platform_verifier::tls_config()` (or the
  equivalent Rustls builder call with the platform verifier); the
  insecure path is `#[cfg]`-gated, emits `tracing::warn!`, and lives in
  a separate module `insecure.rs` whose mere existence in the source
  tree carries the warning.
- The QUIC bench updates its feature flags; CI also runs a
  `cargo build` with **no** insecure feature and a separate
  `cargo test --features insecure-…` only for the bench/test crate.
- `docs/SECURITY_*` reports are updated to delete the "TLS verification
  configured" claim.

## Links

- Related: [ADR-0004](0004-aimds-workspace-member.md),
  [ADR-0014](0014-supply-chain-pinning.md).
- `rustls-platform-verifier`:
  https://docs.rs/rustls-platform-verifier/
