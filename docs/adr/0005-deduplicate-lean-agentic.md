# 0005 — Remove `src/lean_agentic/` duplication of workspace crates

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** workspace, refactor, dedup

## Context and Problem Statement

The top-level crate (`midstream`, `src/lib.rs`) defines a module tree
under `src/lean_agentic/` that **shadows** four workspace crates by
both name and intent:

| Workspace crate                       | Shadowing module                       |
|---------------------------------------|----------------------------------------|
| `crates/nanosecond-scheduler`         | `src/lean_agentic/scheduler.rs`        |
| `crates/temporal-compare`             | `src/lean_agentic/temporal.rs`         |
| `crates/temporal-attractor-studio`    | `src/lean_agentic/attractor.rs`        |
| `crates/temporal-neural-solver`       | `src/lean_agentic/temporal_neural.rs`  |
| `crates/strange-loop`                 | `src/lean_agentic/strange_loop.rs`     |

`src/lean_agentic/mod.rs` *also* re-exports symbols from
`midstreamer_strange_loop` (and presumably the others), so for several
concepts the *same name* resolves to two implementations depending on
the import path — one from the in-tree module, one from the workspace
crate. Example: scheduling policy is defined as
`src/lean_agentic/scheduler.rs::SchedulingPolicy` and *also* as
`midstreamer_scheduler::SchedulingPolicy`. They are not guaranteed to
agree.

The published crates (`midstreamer-temporal-compare`,
`midstreamer-scheduler`, `midstreamer-attractor`,
`midstreamer-neural-solver`, `midstreamer-strange-loop`) are the public
contract. The in-tree `src/lean_agentic/*` modules are an older
implementation that predates the crate split and was never removed.

## Decision Drivers

- **Single source of truth.** A concept (scheduler policy, attractor
  type, theorem store) must have exactly one definition reachable from
  the public API.
- **Public-API integrity.** Consumers of `midstream` end up with a
  different `SchedulingPolicy` than consumers of `midstreamer-scheduler`,
  which makes the cross-crate integration story incoherent.
- **No regression in the binary.** `src/bin/main.rs` and
  `examples/lean_agentic_streaming.rs` use `midstream::lean_agentic::*`.
  Their behaviour must be unchanged.

## Considered Options

1. **Status quo.** Two implementations live in parallel and slowly
   drift apart. Worst long-term option.
2. **Delete `src/lean_agentic/{scheduler,temporal,attractor,
   temporal_neural,strange_loop}.rs` and re-export from the published
   crates.** `src/lean_agentic/mod.rs` becomes a thin facade that
   `pub use`s the symbols from `midstreamer_*` crates so that
   `midstream::lean_agentic::Action` keeps working.
3. **Delete the workspace crates and keep everything in-tree.**
   Inverts the published-crate strategy; breaks five published crates'
   downstream consumers.
4. **Rename the in-tree modules to non-shadowing names.** E.g.
   `src/lean_agentic/scheduler.rs` → `src/lean_agentic/local_scheduler.rs`.
   Resolves the name collision but keeps two implementations.

## Decision Outcome

**Chosen option: Option 2 — delete the in-tree duplicates, keep the
public re-exports.** The `src/lean_agentic/` module becomes a facade
that adapts the published crates into the shapes the
`midstream::lean_agentic::*` consumers expect. Concepts that exist only
in-tree (e.g. `AgenticLoop`, `KnowledgeGraph` — none of these have a
workspace-crate equivalent today) stay in `src/lean_agentic/{agent,
knowledge,learning,reasoning,types}.rs`.

### Positive consequences

- One definition per concept. `SchedulingPolicy` (etc.) is the workspace
  crate's type; everyone uses it.
- The public crate `midstreamer-scheduler` (and friends) is the public
  contract; downstream users get the same types regardless of whether
  they go through `midstream` or directly.
- `src/lean_agentic/` shrinks substantially. Maintenance burden drops.

### Negative consequences

- Any subtle, undocumented divergence between the in-tree and
  workspace implementations becomes a behaviour change. PRs
  implementing this must include a feature-by-feature diff test.
- A few public re-exports may need adapter shims if the workspace
  crate's signature is not identical to the in-tree version.
- This decision is coupled to [ADR-0001](0001-single-cargo-workspace.md):
  if the workspace consolidation lands first, this is a 1-day cleanup;
  if not, the deletion has to coordinate across two workspaces.

## Implementation notes

- Tooling check: `cargo expand --bin midstream` before and after the
  cut, and diff the symbol set. Anything that disappears is a missing
  re-export.
- The five files listed in the table above are deleted; `src/lean_agentic/mod.rs`
  replaces their `pub mod …` declarations with `pub use midstreamer_*::*`.
- Run `cargo test --workspace` and `cargo bench --workspace --no-run`
  to confirm no symbol resolves to a now-deleted path.
- Update `src/lib.rs` re-exports to point at the workspace crates'
  symbols (the names should already match; if not, add shim aliases
  with deprecation warnings).

## Links

- Related: [ADR-0001](0001-single-cargo-workspace.md),
  [ADR-0004](0004-aimds-workspace-member.md).
