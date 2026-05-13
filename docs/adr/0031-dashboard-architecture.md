# 0031 — Console dashboard: split rendering from state, use `ratatui` on the Rust side

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** dashboard, ui, architecture

## Context and Problem Statement

The dashboard lives entirely in TypeScript at
`npm/src/dashboard.ts` (442 LOC) plus a 350-LOC CLI shim at
`npm/src/cli.ts` plus per-modality demos in
`npm/examples/dashboard-demo.ts`. It is a hand-rolled `chalk` +
`readline` renderer with several structural problems:

- **State, rendering, and stream-input mixed in one class.** The
  exported `MidStreamDashboard` (`:49`) holds `DashboardState`,
  drives the redraw loop (`:91 start(refreshRate = 100)`), and
  processes streaming input (`:116 processMessage`, `:142 processStream`).
- **Multi-modal fields baked into the state shape** (`:31-32
  audioStreaming`, `videoStreaming`; `:43 fps`/`latency`). Per
  [ADR-0028](0028-multimodal-scope.md), audio/video are not actually
  implemented; the dashboard renders fields for which no producer
  exists.
- **No event log.** Pattern detections, attractor switches, and
  reward changes appear and disappear as the redraw fires; nothing
  is logged or scrollable.
- **No persistence.** Restarting the dashboard loses all state;
  the rolling window over recent messages (`:33 recentMessages`)
  re-initializes empty.
- **One process.** The dashboard cannot attach to a *running*
  midstream service; it can only host its own analysis inline.
- **No accessibility.** Pure-ANSI output. No JSON / NDJSON mode for
  pipes; no screen-reader-friendly text mode.

This is fine as a demo. It is not fine as the canonical dashboard for
a system whose pitch is "real-time intelligence".

## Decision Drivers

- **One source of truth for state.** The dashboard should *observe*,
  not *generate*, the state it renders.
- **Attach-detach.** A long-running midstream service must be
  observable by zero or many dashboards, simultaneously, without
  affecting its own pipeline.
- **Multiple frontends.** A console TUI is the default; a web UI and
  a JSON-stream-for-scripts mode should be cheap to add.
- **No "ghost" state.** The dashboard renders what the system
  actually produces; it does not invent capabilities (cf. ADR-0028).

## Considered Options

1. **Keep the TS dashboard as-is.** Status quo. Continued drift.
2. **Rewrite the dashboard in Rust using `ratatui` + `crossterm`.**
   Pure Rust, no JS runtime needed to run the console UI, can attach
   directly to the in-process streaming events without a serialize/
   deserialize hop.
3. **Restructure the TS dashboard** into a pure View that subscribes
   to a `DashboardEvents` stream coming over MCP (cf.
   [ADR-0027](0027-rust-js-boundary.md)). State stays in Rust; TS
   only renders.
4. **Hybrid: Rust `ratatui` for the default `midstream tui`
   command; the TS dashboard becomes a thin MCP-client renderer for
   browser/Electron contexts.**

## Decision Outcome

**Chosen option: Option 4 — `ratatui` for the canonical console UI;
the TS dashboard becomes a thin MCP-subscriber renderer.**

Architecture:

```
┌──────────────────┐
│  midstream core  │  emits DashboardEvent<Bytes> on
│  (Rust service)  │  an mpsc channel + via MCP subscribe
└─────────┬────────┘
          │
   ┌──────┼─────────┐
   │      │         │
┌──▼──┐ ┌─▼──┐ ┌────▼────────────┐
│ tui │ │ JS │ │ JSON pipe       │
│ Rs  │ │ MCP│ │ (NDJSON stdout) │
│rata │ │view│ │ for scripts     │
└─────┘ └────┘ └─────────────────┘
```

Concretely:

- New crate `crates/midstreamer-dashboard` exposing:
  - `DashboardEvent` enum (typed; `MessageProcessed`, `PatternDetected`,
    `AttractorTransition`, `RewardUpdate`, `BackpressureSignal`,
    `LimitsViolation`).
  - `DashboardState` (the *observable* state shape; no internal
    counters that belong to the streaming pipeline itself).
  - A `tui` feature that pulls in `ratatui` and `crossterm` and
    builds a console renderer with: a top status bar, an event log
    pane (newest at bottom, scrollable), a metric mini-chart pane,
    a state pane.
- The Rust streaming pipeline emits `DashboardEvent` on every
  significant transition (one event ≠ one message; gated by an
  `event_filter: EnumSet<DashboardEventKind>`).
- The MCP server (per [ADR-0027](0027-rust-js-boundary.md)) exposes
  a `dashboard.subscribe()` tool that streams these events.
- The TS dashboard at `packages/cli/src/dashboard.ts` becomes a thin
  MCP client that subscribes and renders; it shrinks from 442 LOC to
  ~150 because it no longer manages state.
- A `--json` flag on `midstream tui` emits NDJSON to stdout for
  scripted use; one JSON object per `DashboardEvent`.

Removed:

- All `audio`/`video` state fields are removed from `DashboardState`
  until audio/video producers exist (cf. [ADR-0028](0028-multimodal-scope.md)).
- The `start(refreshRate=100)` polling loop is replaced by event-
  driven redraw triggered by incoming events (debounced to
  ≤60 fps).

### Positive consequences

- Dashboard state is generated by the system, not the dashboard.
  Attaching/detaching N dashboards has zero effect on the pipeline.
- `ratatui` gives a real TUI (scrollable log, mini charts, panels)
  instead of full-screen redraws every 100 ms.
- JSON / NDJSON mode falls out of the same `DashboardEvent` stream.
- Browser dashboard becomes a future option (same event stream,
  different renderer).

### Negative consequences

- One-time rewrite. The existing TS dashboard demos
  (`npm/examples/dashboard-demo.ts --mode audio` / `video`) break or
  are replaced by stubs that show the gap honestly.
- Rust `ratatui` is yet another dep tree (small, well-maintained).
- `crates/midstreamer-dashboard` adds a workspace member; CI
  matrix grows by one.

## Implementation notes

- New crate `crates/midstreamer-dashboard` with features `tui`
  (ratatui+crossterm) and `events` (always-on, defines the enum +
  channel).
- `DashboardEvent` derives `Serialize`, `JsonSchema`, `TS` (per
  [ADR-0027](0027-rust-js-boundary.md)) so the TS types regenerate.
- Add `midstream tui` subcommand to `src/bin/main.rs`. It spawns the
  streaming pipeline if not yet running, connects via MCP otherwise.
- `--json` and `--ndjson` flags switch the renderer.
- Update `packages/cli/src/dashboard.ts` to be a pure renderer
  subscribed via `@modelcontextprotocol/sdk`. Delete the
  `DashboardState` interface; import the regenerated TS types.

## Links

- Related: [ADR-0007](0007-bounded-backpressure.md),
  [ADR-0027](0027-rust-js-boundary.md),
  [ADR-0028](0028-multimodal-scope.md).
- `ratatui`: https://ratatui.rs/
- `crossterm`: https://docs.rs/crossterm/
