# 0022 — Persistence layer: delete `ruvector.db`, adopt `redb` when persistence is needed

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** persistence, storage, hygiene

## Context and Problem Statement

There is a **1.5 MB binary file `ruvector.db`** committed at the repo
root (`/home/ruvultra/projects/midstream/ruvector.db`,
`file(1)` reports it as generic `data`). Nothing in the workspace
references it:

- `grep -rn "ruvector" Cargo.toml crates/*/Cargo.toml hyprstream-main/Cargo.toml` → no hits.
- `grep -rn "ruvector.db" src/ crates/*/src/ examples/` → no hits in
  first-party code; only stray references inside `.claude*/` metadata.
- It is **not** an embedded asset (`include_bytes!`/`include_str!`
  return nothing).

So it is an **orphan binary blob** in the repo. Most likely it's the
state file of an unrelated tool (ruflo / ruvector) that was run inside
this directory and accidentally committed; the `.gitignore` should
have excluded it.

Separately, the only persistence dependency in the dependency graph is
**`duckdb = "1.1.1"` with `features = ["bundled"]`** in
`hyprstream-main/Cargo.toml:23` — a 200+MB compile cost driven by a
crate ([ADR-0002](0002-unvendor-hyprstream.md)) that nobody calls
(grep for `hyprstream_core`/`use hyprstream` returns zero hits in
first-party code).

So midstream today has:

- one orphan committed `.db` file (1.5 MB),
- one massive transitively-included database engine (`duckdb
  bundled`) that nothing imports,
- no chosen first-party persistence layer.

Several upcoming ADRs *do* need persistence:

- [ADR-0008](0008-lock-free-scheduler-cache.md) `moka` cache is
  in-memory; nice to have a warm-restart cache file.
- [ADR-0012](0012-streaming-input-bounds.md) `MetricRing` could
  optionally flush to disk on rotation.
- ReasoningBank-style pattern persistence (not in this repo yet but
  referenced from `CLAUDE.md`).

## Decision Drivers

- **No orphaned blobs.** Committed binary files must have a documented
  purpose, or they leave.
- **Don't pay for what you don't use.** `duckdb bundled` adds
  significant compile time for code that isn't invoked.
- **Pick once, use everywhere.** When persistence is needed, the
  pick should be uniform — not a free-for-all of per-crate choices.
- **Embedded, single-file, async-friendly, ACID.** The streaming use
  case is small, local, write-mostly. We don't want a SQL engine; we
  want a fast embedded KV/typed-tree.

## Considered Options

1. **Status quo.** Keep `ruvector.db` and the unused `duckdb`
   dependency.
2. **Delete `ruvector.db`, drop `duckdb bundled`, declare "no
   persistence" until a use-case forces a decision.** Cheapest;
   correct for today.
3. **Adopt `sled`** when persistence is needed. Battle-tested in
   production, but the project's lead has stated unmaintenance risk
   (no 1.0 release; last activity is intermittent).
4. **Adopt `redb`** when persistence is needed. Pure-Rust ACID
   embedded KV with typed tables, single-file, MIT, actively
   maintained, MSRV stable, MMAP-friendly.
5. **Adopt `fjall`** when persistence is needed. LSM-tree, newer
   pure-Rust, async-aware, designed for write-heavy workloads.
6. **Adopt `duckdb`** (already in deps via hyprstream-main).
   Heavy-weight SQL engine; mismatched to a KV use-case.

## Decision Outcome

**Chosen option: Option 2 immediately, plus a forward-binding
preference for Option 4 (`redb`) when persistence is needed.**

Concretely now:

- Delete `ruvector.db` from the working tree. Add `ruvector.db` and
  `*.db` to `.gitignore` so similar accidents are caught.
- Drop the `duckdb` dependency by un-vendoring `hyprstream-main`
  (already covered by [ADR-0002](0002-unvendor-hyprstream.md)). If
  that ADR cannot land immediately, gate `duckdb` behind a feature
  flag so default builds skip it.

Forward direction:

- When persistence is needed (cache warm-restart, metric ring spill,
  pattern store), the implementation crate adds `redb = "2"` and
  exposes a typed table via the workspace's persistence facade
  trait (TBD in a follow-up ADR when the first concrete use-case
  lands).
- `sled` and `fjall` are evaluated for the *first* real write-heavy
  use-case; `redb` is the default until evidence overturns it.

### Positive consequences

- Stops shipping an orphan 1.5 MB binary file in the repo.
- Drops a massive transitively-included database engine that nothing
  imports.
- Establishes the persistence story without prematurely committing to
  a heavy choice.

### Negative consequences

- Deleting `ruvector.db` may surface a missing dependency for any
  external tool that *was* using it from this directory. Mitigated by
  documenting the deletion in the same commit and asking the user
  before deleting (this ADR proposes; the actual `git rm` is a
  follow-up PR).
- The "no persistence today" position means features that would
  benefit from a warm cache (e.g. `moka`'s on-disk persistence add-on)
  stay in-memory-only for now.

## Implementation notes

- Verify the orphan claim once more in a follow-up PR:
  `rg -uu --no-ignore --binary 'ruvector\\.db' .` from the repo root.
- Add `*.db` and `ruvector.db` to root `.gitignore`. Remove the file
  via `git rm ruvector.db` in the same PR; commit message explains
  why and links to this ADR.
- Bisect the `duckdb` cost: confirm `cargo tree --workspace` shows
  `duckdb` only via `hyprstream-main`. Once
  [ADR-0002](0002-unvendor-hyprstream.md) lands, this cost is gone.
- Open a follow-up ADR for the *first* concrete persistence use-case
  (e.g. "ADR-NNNN — Pattern store on disk via `redb`").

## Links

- Related: [ADR-0002](0002-unvendor-hyprstream.md),
  [ADR-0008](0008-lock-free-scheduler-cache.md),
  [ADR-0012](0012-streaming-input-bounds.md).
- `redb`: https://docs.rs/redb/
- `sled`: https://docs.rs/sled/
- `fjall`: https://docs.rs/fjall/
