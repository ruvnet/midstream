# 0004 — AIMDS becomes a first-class workspace member, not a sibling project

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** workspace, security, integration

## Context and Problem Statement

AIMDS (AI Manipulation Defense System) was merged into this repo via
PR #2. It now lives at `AIMDS/` with its **own** Cargo workspace:

```toml
# AIMDS/Cargo.toml
[workspace]
members = [
  "crates/aimds-core",
  "crates/aimds-detection",
  "crates/aimds-analysis",
  "crates/aimds-response",
]

[workspace.dependencies]
midstreamer-temporal-compare = { version = "0.1", path = "../crates/temporal-compare" }
midstreamer-scheduler        = { version = "0.1", path = "../crates/nanosecond-scheduler" }
```

That is:

- AIMDS path-depends on midstream crates *across a workspace boundary*,
- but no midstream crate path-depends on AIMDS,
- and AIMDS is invisible to `cargo check --workspace` run from the repo
  root.

This combination has three concrete effects:

1. **Asymmetric coupling.** AIMDS ships breakage in midstream (because
   it pulls from `../crates/*`) but midstream never builds AIMDS in CI,
   so the inverse breakage is detected by AIMDS releases only.
2. **No invocation site.** As of `30fe5eb`, AIMDS modules are not
   imported from the midstream binary, library, or any workspace crate.
   AIMDS is currently a sibling crate family next to midstream rather
   than a *defence layer for* midstream.
3. **Duplicate `tokio`/`serde`/etc.** Each workspace resolves
   independently, doubling the dependency graph at link time when both
   are built in the same toolchain run.

## Decision Drivers

- **AIMDS is part of the midstream security story** (CVE-1/2/3 in
  `docs/SECURITY_*.md` reference it as the AI-side defence layer).
  It cannot be optional for any deployment that accepts untrusted LLM
  output.
- **Single workspace** (cf. [ADR-0001](0001-single-cargo-workspace.md))
  gives one CI gate and one `Cargo.lock`.
- **Bidirectional, not unidirectional, coupling.** midstream must be
  able to call into AIMDS at the stream-input boundary; AIMDS must be
  able to consume midstream's temporal primitives. Today only the latter
  works.

## Considered Options

1. **Status quo: AIMDS as a sibling workspace.** Easy. Coupling stays
   one-way and silent.
2. **AIMDS becomes part of the root workspace.** `AIMDS/crates/aimds-*`
   are listed in `[workspace.members]` from the root `Cargo.toml`;
   `AIMDS/Cargo.toml` is deleted. Bidirectional path-deps become legal
   and visible to `cargo check --workspace`. Non-Rust AIMDS assets
   (`docker/`, `k8s/`, `scripts/`, `examples/`) stay under `AIMDS/`.
3. **Move AIMDS to a separate repo.** Depend on its published crates
   from midstream. Clean boundary; loses lockstep development.
4. **Flatten AIMDS into midstream's existing `crates/` directory** —
   `crates/aimds-core` etc. — and delete the `AIMDS/` umbrella entirely.
   Most uniform; most disruptive to anyone with bookmarks into `AIMDS/`.

## Decision Outcome

**Chosen option: Option 2 — AIMDS becomes a first-class workspace
member**, with its Rust source moved to `AIMDS/crates/aimds-*` (kept
under the AIMDS umbrella so the docker/k8s/scripts/examples stay
co-located with the crates they protect) and the AIMDS workspace file
deleted in favour of root membership.

Concretely:

- Root `Cargo.toml` adds: `members = [..., "AIMDS/crates/aimds-core",
  "AIMDS/crates/aimds-detection", "AIMDS/crates/aimds-analysis",
  "AIMDS/crates/aimds-response"]`.
- `AIMDS/Cargo.toml` is deleted.
- midstream's input boundary (the place where LLM tokens arrive from
  the wire and before they reach `crates/temporal-compare`) calls into
  `aimds-detection` to check each chunk; PRs implementing this wiring
  cite this ADR.

### Positive consequences

- `cargo check --workspace` covers AIMDS. No more silent breakage on
  refactors of `crates/temporal-compare` or `crates/nanosecond-scheduler`.
- One `Cargo.lock`, one set of resolved versions across the entire repo.
- A real invocation site exists: AIMDS is in the stream-input hot path,
  not in a sibling crate family that nobody calls.
- Symmetric path-deps are now possible: midstream's binary can call
  `aimds_detection::scan`; `aimds_response::block` can call
  `midstreamer_temporal_compare` if needed.

### Negative consequences

- `cargo build` on a clean target builds AIMDS too. Default build time
  goes up; mitigate by leaving AIMDS crates out of `default-members` if
  benchmarks show this is painful.
- Anyone with existing tooling that `cd AIMDS && cargo …` will need to
  switch to `cargo … -p aimds-core` from the root.
- Hot-path coupling introduces a defence-overhead trade-off that must
  be benchmarked (see future perf ADR for the SLO).

## Implementation notes

- Step 1: delete `AIMDS/Cargo.toml`, add the four AIMDS crates to root
  `[workspace.members]`.
- Step 2: convert `AIMDS/crates/aimds-*/Cargo.toml` to use the root
  `[workspace.dependencies]` table for `tokio`, `serde`, etc.
- Step 3: wire `aimds_detection` into the midstream input boundary.
  The first call site is `src/midstream.rs` (the
  `StreamProcessor::process_stream` entry).
- Step 4: add a CI bench (`benches/aimds_overhead_bench.rs`) measuring
  per-token defence overhead; fail the PR if it exceeds the
  to-be-decided SLO.

## Links

- Related: [ADR-0001](0001-single-cargo-workspace.md),
  [ADR-0005](0005-deduplicate-lean-agentic.md).
- AIMDS PR: https://github.com/ruvnet/midstream/pull/2
