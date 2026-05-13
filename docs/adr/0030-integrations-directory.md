# 0030 — `integrations/` is a package, not a stray file

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** workspace, hygiene, typescript

## Context and Problem Statement

The repository contains an `integrations/` directory at the top level
that holds **exactly one file**:

```
integrations/agentic_flow_bridge.ts   (~250 LOC)
```

That file:

- declares no `package.json`,
- has no `tsconfig.json`,
- has no tests,
- has no build target,
- imports `@midstream/lean-agentic` (which is a sibling package at
  `lean-agentic-js/`),
- declares its own local interfaces for an external `agentic-flow`
  npm package (lines 1-37 of the file) instead of depending on the
  real types.

So it is essentially **dead code that won't even type-check on its
own** (no tsconfig means no compiler invocation has ever seen it).
A grep for `agentic_flow_bridge` across the repo returns only the
file itself — nothing imports it.

This is also a violation of the project's own `CLAUDE.md` rule
("Never save working files / md / tests to the root folder") —
`integrations/` at root is a working-file location with no clear
ownership.

## Decision Drivers

- **Every TS file lives inside a tsconfig-rooted package.** Anything
  else is by definition not built and not tested.
- **No orphan top-level directories.** `integrations/`, `plans/`,
  and similar grab-bag locations accumulate cruft over time. Each
  should either be a real package or be archived.
- **The bridge to `agentic-flow` is a legitimate thing to have**;
  it just needs a real home with real CI.

## Considered Options

1. **Status quo.** Stray file at top level; never built; never
   tested.
2. **Delete the file.** Cheapest. Loses the bridge concept that may
   be useful later.
3. **Move under the lean-agentic package** at
   `packages/lean-agentic/src/integrations/agentic-flow.ts` (after
   [ADR-0026](0026-typescript-monorepo.md) lands). Makes it part of
   the lean-agentic public surface.
4. **Promote to its own package** at
   `packages/agentic-flow-bridge/` with `package.json`, `tsconfig`,
   tests, and CI coverage. Discoverable via npm.

## Decision Outcome

**Chosen option: Option 4 — promote `integrations/agentic_flow_bridge.ts`
to its own package `packages/agentic-flow-bridge/`.**

Rationale:

- The bridge is *between* lean-agentic and an external orchestrator
  (`agentic-flow`); semantically it doesn't belong inside either one.
- If/when other orchestrators get bridges (e.g. `swarms`, `crewai`,
  `langgraph`), the pattern repeats; the parallel structure
  `packages/X-flow-bridge/` becomes natural.
- npm-published bridges can be installed without pulling in lean-
  agentic for users who only want the bridge.

Concrete layout (post-[ADR-0026](0026-typescript-monorepo.md)):

```
packages/agentic-flow-bridge/
├── package.json          # @midstream/agentic-flow-bridge, 0.1.0
├── tsconfig.json         # extends ../../tsconfig.base.json
├── src/
│   └── index.ts          # was integrations/agentic_flow_bridge.ts
├── tests/
│   └── bridge.test.ts    # to be written
└── README.md
```

The `interface AgenticFlowConfig { … }` block (lines 1-37 of the
current file) is replaced by a real `agentic-flow` peer dependency
(`peerDependencies: { "agentic-flow": "*" }`) so the types come from
the real package and stay in sync.

The top-level `integrations/` directory is **deleted** in the same
PR.

### Positive consequences

- The bridge file gets compiled, linted, type-checked, and tested
  like everything else.
- The top-level directory list shrinks by one orphan directory.
- A pattern is established for future "bridge to external
  orchestrator" packages.

### Negative consequences

- One more workspace package. Trivial overhead.
- `agentic-flow` becomes a real `peerDependency`; the bridge will
  fail to compile if the dep's types aren't installed. That's the
  right behaviour; today it silently uses fake local types.

## Implementation notes

- Land after [ADR-0026](0026-typescript-monorepo.md) so the
  `packages/*` convention exists.
- `git mv integrations/agentic_flow_bridge.ts
  packages/agentic-flow-bridge/src/index.ts`.
- `rmdir integrations/`.
- Add `package.json`:
  ```json
  {
    "name": "@midstream/agentic-flow-bridge",
    "version": "0.1.0",
    "license": "MIT",
    "main": "dist/index.js",
    "types": "dist/index.d.ts",
    "scripts": {
      "build": "tsc",
      "lint": "eslint src",
      "test": "vitest run",
      "typecheck": "tsc --noEmit"
    },
    "peerDependencies": {
      "agentic-flow": "*",
      "@midstream/lean-agentic": "workspace:^"
    },
    "devDependencies": {
      "agentic-flow": "*",
      "typescript": "^5.3.0",
      "vitest": "^1.0.0"
    }
  }
  ```
- Strip the duplicate `interface AgenticFlowConfig { … }` block from
  the source; import from `agentic-flow` instead.
- Add a basic `bridge.test.ts` that imports the entry and asserts
  the export shape.

## Links

- Related: [ADR-0026](0026-typescript-monorepo.md),
  [ADR-0029](0029-js-ci-matrix.md).
- File: `integrations/agentic_flow_bridge.ts`.
