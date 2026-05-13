# 0025 — Feature-flag policy: additive, off-by-default, documented

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** policy, features, build

## Context and Problem Statement

Cargo feature use is extremely uneven across the workspace:

- **Six of seven Rust crates have no `[features]` section.** Default
  builds are all-in; there is no way to e.g. depend on
  `midstreamer-temporal-compare` without dragging the full LRU cache
  + DTW + LCS + edit-distance code (`crates/temporal-compare`).
- **`npm-wasm/Cargo.toml:13-14` declares features but they are
  default-everything:**

  ```toml
  [features]
  default = ["temporal", "scheduler", "strange-loop", "quic"]
  ```

  …so the feature flags exist syntactically but no consumer
  benefits — turning a feature off requires
  `default-features = false`.
- Several upcoming ADRs add feature flags:
  - [ADR-0011](0011-quic-tls-verification.md):
    `insecure-dev-only-skip-server-verification`.
  - [ADR-0016](0016-llm-provider-trait-redesign.md):
    `provider-genai`, `provider-openai-realtime`.
  - [ADR-0021](0021-quic-implementation-quinn.md):
    `backend-quinn`, `backend-s2n-quic`.

Without a policy, those feature names drift into inconsistent
conventions, and we accumulate the classic feature-flag mess (e.g.
`tls` vs `with-tls` vs `feature-tls`; default-on flags that downstream
consumers can't escape; flags that imply incompatible deps).

## Decision Drivers

- **Additive.** A feature flag turns code *on*; it never changes
  semantics in a way that breaks other features.
- **Off-by-default for opinionated choices.** The default build is
  the smallest, most-secure surface. Opt in to risk, opt in to
  vendor-specific code.
- **Lowercase-kebab, prefix by domain.** Easier to grep, easier to
  reason about.
- **Mutually-exclusive features documented and detected at
  compile-time.** Cargo cannot enforce mutex; we use `compile_error!`.

## Considered Options

1. **No policy.** Free-for-all. Names drift.
2. **Adopt the policy below.** Every crate's features conform.
3. **Workspace-shared `[features]` table.** Cargo doesn't support
   this directly; a build-script hack could approximate it.

## Decision Outcome

**Chosen option: Option 2 — a documented policy applied to every
crate.**

### Naming

- `kebab-case`, lowercase only.
- Prefix by domain:
  - `backend-…` — alternative transport/storage backends (e.g.
    `backend-quinn`, `backend-s2n-quic`).
  - `provider-…` — external service adapters (e.g.
    `provider-genai`, `provider-openai-realtime`).
  - `wasm-…` — WASM-specific knobs.
  - `tokio-…` — runtime knobs (e.g. `tokio-console`).
  - `insecure-…` — anything that lowers a default security posture
    (e.g. `insecure-dev-only-skip-server-verification`). These
    always include a noisy substring (`insecure`, `dev-only`,
    `unsound`) so they show up in any audit grep.

### Defaults

- A crate's `default` feature set contains only what is required for
  the crate to be useful in the most common configuration **without
  enabling any optional dependency, any unsafe code path, or any
  opinionated backend choice**.
- For top-level binaries (`midstream`), `default` may include the
  primary backend (e.g. `backend-quinn`).
- For libraries (`crates/*`), `default` is **empty** unless the
  crate would be inert without something; in that case the smallest
  viable set.

### Additivity

- Features must be additive. Turning on `feature-A` must never
  disable a previously-enabled feature or change a function's
  semantics from "X" to "Y".
- Mutual exclusion is enforced at compile time, not at feature
  resolution time:

  ```rust
  #[cfg(all(feature = "backend-quinn", feature = "backend-only-one"))]
  compile_error!("…");
  ```

### Documentation

- Every feature has a `## ` heading entry in the crate's
  `README.md` or doc-comment with: what it enables, what cost it
  adds (deps, compile time, runtime), and whether it lowers a
  security default.
- `cargo doc` builds with `--all-features` for docs.rs; non-default
  feature items carry `#[cfg(feature = "…")]` doc comments via
  `#[doc(cfg(feature = "…"))]` (requires `#![feature(doc_cfg)]` only
  on nightly docs.rs builds — gate with `cfg_attr(docsrs, …)`).

### Forbidden patterns

- **Default-everything-then-opt-out.** `npm-wasm`'s current
  pattern is a violation; the next bump of that crate sets
  `default = []`.
- **`std` vs `no_std` via the *absence* of a feature.** Crates that
  want `no_std` support add a `std` feature that is `default = ["std"]`,
  not the other way around (positive sense).
- **Cross-crate feature aliasing without `dep:`.** Use the explicit
  `dep:foo` syntax (Cargo namespaced features) so an enabled
  feature does not silently activate `foo`'s default features.

### Positive consequences

- Predictable feature names across crates; greppable.
- Default builds get smaller and safer; risky modes are opt-in and
  visible in `cargo tree --features`.
- WASM crate stops shipping everything by default; consumers can
  ship a slim build.

### Negative consequences

- One-time churn: `npm-wasm`'s `default` set must change, and any
  downstream that relied on it gets a one-line migration
  (`default-features = false` → explicit feature list).
- More features = more CI matrix entries (`cargo hack --feature-powerset`
  is the standard mitigation; cf. [ADR-0014](0014-supply-chain-pinning.md)).

## Implementation notes

- Add a `feature-flags` section to each crate's `README.md`.
- Add `cargo-hack` to CI for `--feature-powerset --no-dev-deps
  check`. Catches features that don't compile together.
- Change `npm-wasm/Cargo.toml`:

  ```toml
  [features]
  default = []
  temporal = ["dep:midstreamer-temporal-compare"]
  scheduler = ["dep:midstreamer-scheduler"]
  strange-loop = ["dep:midstreamer-strange-loop"]
  quic = ["dep:midstreamer-quic-multistream"]
  ```

  …and document a "full" meta-feature that enables all four for
  consumers who want today's behaviour.
- Update each future feature-introducing ADR to cite this policy.

## Links

- Related: [ADR-0011](0011-quic-tls-verification.md),
  [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0016](0016-llm-provider-trait-redesign.md),
  [ADR-0021](0021-quic-implementation-quinn.md),
  [ADR-0024](0024-semver-and-api-stability.md).
- `cargo-hack`: https://github.com/taiki-e/cargo-hack
- Cargo namespaced features:
  https://doc.rust-lang.org/cargo/reference/features.html#dependency-features
