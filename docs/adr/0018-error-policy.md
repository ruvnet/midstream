# 0018 — Error-handling policy: `thiserror` for libs, `anyhow` for binaries

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** error-handling, api, libs

## Context and Problem Statement

Error handling across the repo is inconsistent:

- Each workspace crate defines its own `thiserror`-derived enum
  (`crates/temporal-compare/src/lib.rs:18 TemporalError`,
  `crates/nanosecond-scheduler/src/lib.rs:22 SchedulerError`,
  `crates/temporal-attractor-studio/src/lib.rs:18 AttractorError`,
  `crates/temporal-neural-solver/src/lib.rs:18 TemporalError` —
  note the duplicate name across two crates — ,
  `crates/strange-loop/src/lib.rs:23 StrangeLoopError`,
  `crates/quic-multistream/src/lib.rs:71 QuicError`).
- The top-level binary uses `Box<dyn std::error::Error>` in some
  signatures and `BoxError` aliases in others
  (`src/hypr_service.rs:14`).
- Examples use `expect("Failed to send request")` (`examples/openrouter.rs:60`)
  and `unwrap()` on serialization (`npm-wasm/src/lib.rs:391, 484-487`).
- `thiserror` version skew: root and most crates pin `2.0`, but
  `AIMDS/Cargo.toml:42` pins `1.0`. Two `thiserror` versions compiled
  in the same workspace once AIMDS is folded in.

There is no documented policy for:

- when to use `thiserror` vs `anyhow`,
- whether public APIs may expose `Box<dyn Error>`,
- whether `unwrap`/`expect` are ever acceptable in non-test code,
- how to name error types to avoid the
  `temporal_compare::TemporalError` vs `temporal_neural_solver::TemporalError`
  collision.

## Decision Drivers

- **Public-API stability.** Library error types are part of the
  semver surface; they must be intentional, named distinctly, and
  match the crate's concern.
- **Caller ergonomics.** Application code (binaries, examples) wants
  the contextual richness of `anyhow`; libraries should expose typed
  variants so downstream `match`-on-error works.
- **No `Box<dyn Error>` in public APIs.** It hides the error type and
  forces every consumer to downcast.

## Considered Options

1. **Status quo.** Continue with mixed style.
2. **Mandate `thiserror` in every crate, `anyhow` everywhere else.**
   The "Burntsushi style"; widely adopted in the Rust ecosystem.
3. **Single shared `MidstreamError` enum in a `midstream-error`
   crate.** Each sub-crate's error becomes a variant. Tight; risks
   forcing unrelated crates to depend on each other.
4. **Variant 2 + nominal `Error` type per crate, distinct names.**
   `temporal_compare::CompareError`, `temporal_neural_solver::SolverError`
   to remove the duplicate-`TemporalError`-name collision.

## Decision Outcome

**Chosen option: Option 4.**

Rules:

| Code lives in… | Uses `thiserror` | Uses `anyhow` | Allowed `unwrap`/`expect` |
|----------------|------------------|----------------|---------------------------|
| `crates/midstreamer-*` (libs) | **Yes** — one error enum per crate, named `<Concern>Error` (never just `Error`) | No | Only in tests / `unreachable!` proofs |
| `src/` (binary + library) | Yes for re-exportable types | Yes for binary glue | Tests only |
| `examples/`   | No | Yes — `Result<(), anyhow::Error>` is the example boilerplate | Allowed; examples document intent |
| WASM crates   | Yes; convert to `JsError` at the boundary | No | Never — `unwrap` panics the WASM module |

Renames:

- `crates/temporal-neural-solver/src/lib.rs::TemporalError` →
  `SolverError`.
- `crates/temporal-compare/src/lib.rs::TemporalError` → `CompareError`.
- `crates/temporal-attractor-studio/src/lib.rs::AttractorError` keeps
  its name; collides with nothing.
- The top-level `BoxError = Box<dyn std::error::Error + Send + Sync>`
  alias in `src/hypr_service.rs:14` is replaced with a typed
  `HyprServiceError` (thiserror) plus `anyhow::Result` only in the
  binary entry point.

Version pinning:

- `thiserror = "2"` at `[workspace.dependencies]`; every crate (including
  `AIMDS/*` after [ADR-0004](0004-aimds-workspace-member.md)) inherits.
- `anyhow = "1"` likewise.

### Positive consequences

- Every public API returns a named error type; downstream code can
  match on variants without downcasting.
- `unwrap`/`expect` in production code becomes a clippy lint
  (`clippy::unwrap_used`, `clippy::expect_used`) at `deny` level for
  `crates/` and `src/`, `warn` for examples.
- No more cross-crate name collision (`temporal::TemporalError`).
- One `thiserror` version compiles, not two.

### Negative consequences

- One-time rename across the four colliding crate error types.
  Downstream consumers must adjust imports (semver-major bump).
- Some hot-path code that today does `unwrap()` because the result is
  statically impossible will need explicit `unreachable!()` or
  documented invariants.

## Implementation notes

- Add to root `Cargo.toml`:
  ```toml
  [workspace.lints.clippy]
  unwrap_used = "deny"
  expect_used = "deny"
  panic = "deny"
  ```
  with overrides for `tests/`, `examples/`, and `benches/` set to `warn`.
- Rename the colliding error types in their crates with deprecation
  re-exports (`pub use SolverError as TemporalError;` with
  `#[deprecated]`) for one version.
- Replace `BoxError` and `Box<dyn Error>` returns across `src/` with
  `anyhow::Result` (binary) or named `thiserror` (library).
- Add `tests/no_unwrap_test.rs` that grep-asserts `unwrap()` does not
  appear under `crates/*/src/` or `src/` (ratchet test, not a strict
  lint).

## Links

- Related: [ADR-0011](0011-quic-tls-verification.md),
  [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0016](0016-llm-provider-trait-redesign.md).
- "Error Handling in Rust", Andrew Gallant:
  https://blog.burntsushi.net/rust-error-handling/
