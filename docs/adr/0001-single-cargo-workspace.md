# 0001 — Single Cargo workspace for all Rust crates

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** workspace, build, releases

## Context and Problem Statement

The repository currently contains **three independent Cargo workspaces
plus several stand-alone crates** that all share code by `path = "..."`:

| Workspace / crate                     | Members                                                   |
|---------------------------------------|-----------------------------------------------------------|
| `Cargo.toml` (root)                   | `crates/quic-multistream`, `crates/temporal-compare`, `crates/nanosecond-scheduler`, `crates/temporal-attractor-studio`, `crates/temporal-neural-solver`, `crates/strange-loop` |
| `AIMDS/Cargo.toml`                    | `aimds-core`, `aimds-detection`, `aimds-analysis`, `aimds-response` — and depends on `../crates/temporal-compare` and `../crates/nanosecond-scheduler` by path |
| `hyprstream-main/Cargo.toml`          | vendored copy of the upstream `hyprstream` crate, depended on from root by `path = "hyprstream-main"` |
| `wasm/Cargo.toml` (`lean-agentic-wasm`)         | independent crate, no parent workspace                    |
| `wasm-bindings/Cargo.toml` (`midstream-wasm`)   | independent crate, depends on root by `path = ".."`       |
| `npm-wasm/Cargo.toml`                 | yet another independent WASM crate                        |

Effects:

- **`cargo check --workspace` from the root checks ~6 crates** but
  silently ignores AIMDS, the three WASM crates, and `hyprstream-main`.
  CI therefore can't catch breaking changes across module boundaries in
  one step.
- **`Cargo.lock` files multiply.** Each sub-workspace resolves
  versions independently, which has already produced version skew on
  shared deps (e.g. `tokio`, `serde`, `wasm-bindgen`).
- **Releasing requires N publish scripts.** `publish_aimds.sh`,
  `publish_aimds_crates.sh`, `publish_midstream_crates.sh`,
  `publish_midstreamer_crates.sh` — four ordered scripts whose ordering
  is captured nowhere except shell history.
- **`exclude = ["npm-wasm"]` in the root manifest** is a workaround, not
  a design.

## Decision Drivers

- **Single source of truth for dependency versions.** No version skew
  between sibling crates that ship together.
- **CI in one command.** `cargo check --workspace` and
  `cargo test --workspace` should cover everything that gets shipped
  from this repo.
- **Release ordering preserved.** crates.io publishes need a single,
  reproducible topological order — easily derived from a unified
  workspace graph.
- **No regression in WASM build matrix.** WASM crates compile with
  `--target wasm32-unknown-unknown` (or `wasm32-wasip1`), which is
  incompatible with crates that pull in tokio's `net` feature. The
  workspace must keep these separable.

## Considered Options

1. **Status quo.** Multiple parallel workspaces, path-deps across
   workspace boundaries, four publish scripts. Zero refactor cost,
   ongoing tax forever.
2. **Single workspace, all crates as members.** Root `Cargo.toml`
   members = `crates/*`, `AIMDS/crates/*`, `wasm-bindings`. Stop using
   `path = ".."` to cross workspace boundaries. Use `default-members` to
   keep the WASM crates out of the default `cargo check` set so they
   don't break native-only builds.
3. **Single workspace per published-crate family**, plus a top-level
   "manifest workspace" that lists them all via `[workspace.members]`
   with explicit dedup of `Cargo.lock`. Hybrid; more moving parts.
4. **Split repo.** Keep midstream and AIMDS in separate Git repos and
   depend across by version. Cleanest theoretical answer, but the two
   were merged on purpose (PR #2) and share active development.

## Decision Outcome

**Chosen option: Option 2 — single workspace with all Rust crates as
members**, because it eliminates version skew, gives CI a single entry
point, and removes the need for the four publish scripts. WASM crates
stay buildable by moving them out of `default-members` rather than out
of the workspace.

### Positive consequences

- One `Cargo.lock` for the whole repo. No more skew on `tokio`,
  `serde`, `wasm-bindgen`, `arrow`, etc.
- `cargo check --workspace` becomes the single CI gate.
- `cargo publish --workspace` (nightly feature, or `cargo-release`'s
  workspace mode) replaces the four hand-written publish scripts.
- AIMDS, midstream, and WASM crates can `#[cfg]`-share types without
  the current path-dep tunneling.

### Negative consequences

- One-time refactor: `AIMDS/Cargo.toml` ceases to be a workspace; its
  four crates move under `crates/aimds-*/` (or `AIMDS/crates/aimds-*/`
  with the root workspace listing them). All path-deps update.
- WASM build matrix becomes a bit more fiddly: builds that target
  `wasm32-unknown-unknown` need explicit `-p` selection or
  `--exclude` lists for crates that pull in tokio-net.
- The `hyprstream-main/` vendored copy is also a workspace member
  *only if* ADR-0002 (un-vendor) does **not** land first; otherwise it
  goes away.

## Implementation notes

- Step 1: move `AIMDS/crates/aimds-*` to `crates/aimds-*`, delete
  `AIMDS/Cargo.toml`. Keep the rest of `AIMDS/` (docker, k8s, scripts)
  in place.
- Step 2: add `wasm-bindings` to `[workspace.members]`. Decide WASM
  consolidation in [ADR-0003](0003-wasm-consolidation.md) before adding
  `wasm/` or `npm-wasm/`.
- Step 3: add `[workspace.default-members]` listing only crates that
  build on the native host target.
- Step 4: replace `publish_*.sh` with a single `cargo-release` config
  that respects the workspace's dependency DAG.

## Links

- Related: [ADR-0002](0002-unvendor-hyprstream.md),
  [ADR-0003](0003-wasm-consolidation.md),
  [ADR-0004](0004-aimds-workspace-member.md).
- `cargo-release` workspace mode:
  https://github.com/crate-ci/cargo-release
