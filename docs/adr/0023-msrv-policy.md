# 0023 — Minimum Supported Rust Version (MSRV) policy

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** policy, releases, dependencies

## Context and Problem Statement

**No crate in the workspace declares a `rust-version`.** A grep for
`rust-version` across `Cargo.toml`, `crates/*/Cargo.toml`,
`AIMDS/Cargo.toml`, `wasm-bindings/Cargo.toml`, `wasm/Cargo.toml`,
`npm-wasm/Cargo.toml`, and `hyprstream-main/Cargo.toml` returns zero
hits. All crates declare `edition = "2021"` but no minimum compiler.

Effects:

- **`cargo publish` sets no MSRV constraint** — downstream consumers
  on older toolchains hit `rustc` errors deep in our code with no
  pre-emptive `error: package requires Rust X.Y` message.
- **CI tests `stable` and `nightly`** (`.github/workflows/rust-ci.yml:57`),
  which only proves the *latest* toolchain works; an older toolchain
  that someone is actually pinned to may break silently.
- **The published crates' `docs.rs` build** uses whatever rustc is
  current at publish time, which floats.
- **Dependency upgrades silently raise our MSRV.** A `cargo update`
  that pulls a new `tokio` minor can break consumers on an older
  toolchain with no signal.

Modern Rust ecosystem norms (as of mid-2026):

- `tokio`, `serde`, `clap` typically declare MSRV around N-6 minor
  releases (≈6 months behind stable).
- `arrow`, `quinn`, modern async-trait code often require MSRV
  much closer to stable.
- Several features we want use *recent* Rust (e.g. async fn in trait
  for [ADR-0016](0016-llm-provider-trait-redesign.md), GAT-based
  patterns for `moka`).

## Decision Drivers

- **A stated MSRV** so downstream consumers can plan toolchain pins.
- **A stated MSRV bump policy** so consumers know what to expect.
- **CI enforcement** so the MSRV claim doesn't quietly drift.
- **Realistic ceiling.** We want async fn in trait without
  `async_trait` (Rust 1.75+); we don't want to chase nightly.

## Considered Options

1. **No declared MSRV (status quo).** Free hand for us; opaque for
   consumers.
2. **MSRV = stable.** Always the latest. Bumps freely on every
   release. Cheapest to maintain; least friendly to enterprise
   consumers.
3. **MSRV = N-3 (≈3 months behind stable).** Loose ceiling; allows
   us to use most modern features (async fn in trait, GATs) without
   chasing edge.
4. **MSRV = N-6 or fixed (e.g. 1.75).** Conservative; matches
   `tokio`/`serde` policy; locks us out of some recent improvements
   for ~6 months.

## Decision Outcome

**Chosen option: Option 3 — MSRV declared as a specific version,
floating at roughly N-3 (3 minor releases behind current stable).**

For 2026-05, that means **MSRV = 1.81** (current stable as of this
ADR is 1.84; N-3 = 1.81). Each crate declares
`rust-version = "1.81"`.

MSRV bump policy:

- **Bumping MSRV is a minor-version bump on the affected crate**
  (e.g. 0.2.0 → 0.3.0 in 0.x; 1.4.0 → 1.5.0 in 1.x).
- **MSRV bumps require an ADR** that names the reason (a specific
  language feature, a specific dep MSRV).
- **Bump cadence: at most quarterly.** Even if no specific reason,
  we re-evaluate quarterly; if we don't bump, we document why in the
  Q's release notes.

CI enforcement:

- Add a CI matrix entry running on the declared MSRV via
  `dtolnay/rust-toolchain@1.81` (or the currently-declared version).
  All `cargo check --workspace`, `cargo test --workspace --lib`, and
  `cargo build --workspace` jobs run on both `stable` and `MSRV`.
- A separate `cargo +nightly` job continues to exercise nightly to
  catch future regressions, but it is non-blocking.

### Positive consequences

- Consumers know the MSRV. `cargo publish` enforces it.
- MSRV drift is detected by CI, not by a downstream consumer's bug
  report.
- The policy makes "we just bumped tokio to a version that needs
  newer rustc" a visible, discussable change.

### Negative consequences

- One CI job per crate matrix entry on the MSRV adds wall-clock to
  PRs. Mitigated by reusing the `Swatinem/rust-cache` cache.
- Some attractive deps may have already-higher MSRVs and force a
  bump sooner than quarterly. Each such bump is an ADR.

## Implementation notes

- Add `rust-version = "1.81"` to:
  - root `Cargo.toml`,
  - each `crates/*/Cargo.toml` (or a `[workspace.package]` shared
    inheritance once [ADR-0001](0001-single-cargo-workspace.md) lands).
- Add an MSRV CI job in `.github/workflows/rust-ci.yml`:

  ```yaml
  msrv:
    name: MSRV (1.81)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.81
      - uses: Swatinem/rust-cache@v2
        with: { shared-key: "msrv" }
      - run: cargo check --workspace --locked
      - run: cargo test --workspace --lib --locked
  ```

- `--locked` is mandatory on the MSRV job: an unlocked update could
  pull a dep that needs newer rustc, masking the MSRV check.
- Add `cargo install cargo-msrv` to a maintenance script for the
  quarterly review.

## Links

- Related: [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0016](0016-llm-provider-trait-redesign.md),
  [ADR-0024](0024-semver-and-api-stability.md).
- `cargo-msrv`: https://github.com/foresterre/cargo-msrv
- `rustc` release schedule: https://forge.rust-lang.org/
