# 0040 — Web dashboard: deferred behind a stable event stream

- **Status:** Proposed (defers concrete tech choice)
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** dashboard, web, roadmap, deferred

## Context and Problem Statement

[ADR-0031](0031-dashboard-architecture.md) makes the canonical
console dashboard Rust+`ratatui`, with the TypeScript dashboard
demoted to a thin MCP-subscriber renderer. That ADR explicitly
mentions a "browser dashboard becomes a future option" but does not
pin down the technology.

A real web dashboard is plausible:

- The events ADR-0031 introduces (`DashboardEvent`) are
  serialisable JSON.
- The MCP layer (post-[ADR-0027](0027-rust-js-boundary.md),
  [ADR-0032](0032-mcp-tool-surface.md)) already exposes a stream
  that a browser can subscribe to via WebSocket or SSE.
- The current `npm/` TS surface has the patterns to build it.

But premature commitment to a frontend stack (React vs Svelte vs Vue
vs Leptos vs Yew) would force a choice with no concrete user
demand to ground it, and risks producing yet another half-finished
artefact like the multi-modal claims ([ADR-0028](0028-multimodal-scope.md))
or the unimplemented HTTP RPC server in `lean-agentic-js`
([ADR-0027](0027-rust-js-boundary.md)).

This ADR formally **defers** the web-dashboard tech choice while
naming the interface that any future implementation must conform
to, so that downstream work doesn't accidentally close that door.

## Decision Drivers

- **Don't promise what we won't build.** README and roadmap claims
  must be honest (cf. [ADR-0028](0028-multimodal-scope.md)).
- **Don't pre-commit to a stack** that may not fit when a real user
  requirement arrives.
- **Keep the interface clean** so the future implementation has
  exactly one source of truth to consume.
- **Allow third parties to ship a frontend** against our event
  stream without coordinating with us.

## Considered Options

1. **Ignore the web dashboard.** Pretend the option doesn't exist;
   risk that someone hand-rolls a one-off using an unstable
   internal API.
2. **Build a Rust-WASM dashboard now** (Yew or Leptos). Real risk
   of premature commitment; no current user has asked for it.
3. **Build a React/Svelte dashboard now.** Same as above.
4. **Defer the *implementation*; lock down the *interface***.
   `DashboardEvent` JSON Schema is stable; any consumer (Rust-WASM,
   React, CLI, Grafana plugin, …) can subscribe.
5. **Build a tiny "reference frontend"** (vanilla TS + the MCP SDK)
   as a proof-of-life — to show the event stream is usable from a
   browser — without committing to a framework.

## Decision Outcome

**Chosen option: Option 4 + 5 in combination.**

- The **interface** is committed now: `DashboardEvent`'s JSON Schema
  (auto-derived per [ADR-0027](0027-rust-js-boundary.md)) is part of
  the published `crates/midstreamer-dashboard` crate's documented
  surface. It is versioned per [ADR-0032](0032-mcp-tool-surface.md)'s
  `midstream/v<N>/...` rule.
- The **reference frontend** is a single page (~200 LOC vanilla
  TypeScript + `@modelcontextprotocol/sdk`, no framework) that:
  - subscribes to `midstream://events`,
  - renders a stripped-down version of the `ratatui` console
    dashboard's panels into plain HTML,
  - has no styling beyond minimal CSS.
  It lives at `packages/cli/examples/web-dashboard/index.html`
  (under [ADR-0026](0026-typescript-monorepo.md)'s pnpm workspace)
  and is **explicitly labelled as a reference**, not a product.
- The **framework choice for a "real" dashboard** is deferred until
  a real user requirement arrives, captured in a follow-up ADR.

### What this rules out

- Sneaking a framework dep into the workspace before the choice is
  made. Any PR that adds React, Vue, Svelte, Leptos, Yew, Solid,
  Astro, or similar without a superseding ADR is auto-blocked.
- Building a dashboard against ad-hoc internal APIs instead of the
  published `DashboardEvent` stream.

### Positive consequences

- The README can honestly claim "subscribe to live events from a
  browser" once the reference frontend lands.
- Third parties can ship dashboards (vendor-neutral Grafana plugin,
  enterprise observability dashboards) by consuming the same JSON
  Schema we use.
- We avoid framework-of-the-month churn.

### Negative consequences

- Anyone expecting a polished web UI today won't find one; we ship
  a reference page, not a product. Mitigated by labelling explicitly.
- The deferred ADR adds bureaucracy: the framework choice has to be
  argued when the time comes. That's the point.

## Implementation notes

- Land [ADR-0031](0031-dashboard-architecture.md) first so the
  event stream exists.
- Land [ADR-0027](0027-rust-js-boundary.md) for `ts-rs`/`schemars`
  so the schema is generated.
- Write the reference frontend in a single HTML file with inline TS
  compiled via `esbuild` or served raw via a Vite dev server in
  `pnpm dev`.
- Add a note in `docs/ROADMAP.md` under "Web dashboard (framework
  TBD)" listing this ADR as the gate.
- Add `[lints]` rule (per [ADR-0034](0034-workspace-lints.md)) or
  a `cargo deny` ban that flags any of {react, vue, svelte, leptos,
  yew, solid, astro} appearing in workspace dependencies; PR fails
  with a pointer to this ADR.

## Links

- Related: [ADR-0027](0027-rust-js-boundary.md),
  [ADR-0028](0028-multimodal-scope.md),
  [ADR-0031](0031-dashboard-architecture.md),
  [ADR-0032](0032-mcp-tool-surface.md),
  [ADR-0034](0034-workspace-lints.md).
