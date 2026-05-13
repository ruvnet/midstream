# 0032 — MCP tool surface: namespaced verbs, schema-versioned, lifecycle-separated

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** mcp, api, naming

## Context and Problem Statement

`npm/src/mcp-server.ts` registers 8 MCP tools (`npm/src/mcp-server.ts:42,
116, 131, 156, 176, 191, 209, 217, 230`):

| Current name           | Verb shape | Concern   | Issue                                                  |
|------------------------|------------|-----------|--------------------------------------------------------|
| `analyze_conversation` | verb_noun  | analysis  | OK                                                     |
| `compare_sequences`    | verb_noun  | analysis  | OK                                                     |
| `detect_patterns`      | verb_noun  | analysis  | OK                                                     |
| `analyze_behavior`     | verb_noun  | analysis  | overloads `analyze_`; pattern-name collision risk      |
| `meta_learn`           | verb_verb  | learning  | underscore-as-space; reads oddly                       |
| `get_status`           | verb_noun  | meta      | should be a *resource* (read-only), not a *tool*       |
| `stream_websocket`     | verb_noun  | lifecycle | starts a long-running server — different concern entirely |
| `stream_sse`           | verb_noun  | lifecycle | same                                                   |

Structural issues:

1. **Lifecycle tools mixed with analysis tools.** `stream_websocket`
   and `stream_sse` *start* network servers as a side effect of a
   tool call. That's a server-lifecycle operation, not an analysis
   tool. Mixing them encourages misuse.
2. **No namespace.** When this server runs alongside other MCP servers
   (claude-flow, ruflo, etc.), tool names collide. The MCP spec lets
   us namespace; we don't.
3. **No version in the tool surface.** Bumping the schema of
   `compare_sequences` from "2 sequences" to "N sequences" silently
   breaks every consumer pinned to today's shape.
4. **`get_status` is a tool, not a resource.** The MCP spec
   distinguishes `tools` (effectful or expensive) from `resources`
   (cheap, read-only, addressable). Listing a `get_status` tool wastes
   a tool slot in the LLM's tool-call budget.
5. **Argument schemas are hand-rolled** (e.g.
   `npm/src/mcp-server.ts:124-127` describes a JSON-Schema array of
   strings inline). The Rust side will generate these from
   `schemars`-derived types post-[ADR-0027](0027-rust-js-boundary.md);
   the hand-rolled schemas should not survive.
6. **No idempotency contract.** Repeated calls to `meta_learn` with
   identical input mutate state; there's no `idempotency_key` or
   `dry_run` hint.

## Decision Drivers

- **Namespace + verb-noun + version.** `midstream/v1/analysis.compare_sequences`
  beats `compare_sequences` on every dimension.
- **Tools vs resources.** Read-only addressable data goes in
  resources; effectful operations stay as tools.
- **Schema-first.** Argument shapes derive from Rust types
  (`schemars`), not hand-rolled JSON in TS.
- **Versioning surface.** Adding a v2 of a tool keeps v1 working.

## Considered Options

1. **Keep current names.** Ongoing tax.
2. **Rename only.** Add the `midstream/v1/<group>.<verb>_<noun>`
   convention; keep the lifecycle tools as tools.
3. **Rename + split lifecycle into resources + admin namespace.**
   `midstream/v1/analysis.*` for analysis tools,
   `midstream/v1/admin.*` for lifecycle, and *resources* for
   `status`, `metrics`, `streams`.
4. **Replace MCP entirely.** Out of scope; rejected by
   [ADR-0027](0027-rust-js-boundary.md).

## Decision Outcome

**Chosen option: Option 3.**

Naming convention:

```
midstream/v<N>/<group>.<verb>_<noun>
```

- `v<N>`: integer schema version per group. Independent of the crate
  version. Bumped when an argument or return type changes shape.
- `<group>`: one of `analysis`, `learning`, `admin`. Future groups
  add as needed.
- `<verb>_<noun>`: snake_case, imperative verb + noun. Idempotency
  must be expressible without changing the name.

Initial mapping:

| Old name                  | New name                                  | Kind     |
|---------------------------|-------------------------------------------|----------|
| `analyze_conversation`    | `midstream/v1/analysis.analyze_conversation` | Tool     |
| `compare_sequences`       | `midstream/v1/analysis.compare_sequences`    | Tool     |
| `detect_patterns`         | `midstream/v1/analysis.detect_patterns`      | Tool     |
| `analyze_behavior`        | `midstream/v1/analysis.analyze_attractor`    | Tool     |
| `meta_learn`              | `midstream/v1/learning.record_event`         | Tool     |
| `get_status`              | (resource) `midstream://status`              | Resource |
| `stream_websocket`        | `midstream/v1/admin.start_websocket_server`  | Tool     |
| `stream_sse`              | `midstream/v1/admin.start_sse_server`        | Tool     |

Additions:

- Resource `midstream://metrics` — current metric values (read-only).
- Resource `midstream://streams` — list of active stream IDs.
- Resource `midstream://config` — effective config (with secrets
  redacted per [ADR-0019](0019-config-system.md)).

Schema rules:

- Every tool argument and return shape is a Rust type with
  `#[derive(Serialize, Deserialize, JsonSchema, TS)]`; the MCP
  argument schema is generated, never hand-rolled.
- Every tool has `idempotency_key: Option<String>` and
  `dry_run: bool` arguments in addition to its domain inputs.
- Every tool returns `Result<Output, ToolError>` where `ToolError` is
  a typed enum (per [ADR-0018](0018-error-policy.md)) with stable
  string discriminants for protocol round-tripping.

Deprecation:

- Old names continue to work for one release with a
  `deprecation_warning` field in their response payload.
- The deprecation table goes in `docs/MCP_DEPRECATIONS.md`; updated
  for each rename.

### Positive consequences

- Tool names are greppable, namespaced, and versioned.
- The LLM's tool budget is reclaimed: read-only data moves to
  resources.
- Argument schemas drift-free with Rust types.
- Lifecycle calls are no longer mistakable for analysis calls.

### Negative consequences

- All current consumers of the MCP server need to update tool names.
  Mitigated by the one-release deprecation window.
- The Rust-side MCP server crate (introduced by
  [ADR-0027](0027-rust-js-boundary.md)) inherits this convention;
  the TS MCP server stays in sync until it can be retired.

## Implementation notes

- Add `crates/midstreamer-mcp-protocol` with the typed tool
  inputs/outputs and the `ToolError` enum.
- Add `crates/midstreamer-mcp-server` (per
  [ADR-0027](0027-rust-js-boundary.md)) that registers tools under
  the new names.
- Update `npm/src/mcp-server.ts` to register both old and new names
  for one release; emit `deprecation_warning` on old-name calls.
- Generate `docs/MCP_TOOLS.md` from the JSON-Schema bundle so the
  tool inventory is documented automatically.

## Links

- Related: [ADR-0018](0018-error-policy.md),
  [ADR-0019](0019-config-system.md),
  [ADR-0027](0027-rust-js-boundary.md),
  [ADR-0031](0031-dashboard-architecture.md).
- MCP spec: https://modelcontextprotocol.io/specification/
