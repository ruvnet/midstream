# 0026 — TypeScript monorepo via `pnpm` workspaces

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** workspace, typescript, packaging

## Context and Problem Statement

The repository contains **three separate TypeScript packages plus one
stray TS file**, each independently lockfiled and unaware of the others:

| Path                     | npm name              | Version | LOC (src) | Lockfile |
|--------------------------|-----------------------|---------|-----------|----------|
| `npm/`                   | `midstream-cli`       | 0.1.0   | 2,832 src + 1,429 tests | `package-lock.json` (npm) |
| `npm-wasm/`              | `@midstream/wasm`     | 1.0.0   | (Rust + JS glue)        | `package-lock.json` (npm) |
| `lean-agentic-js/`       | `@midstream/lean-agentic` | 1.0.0 | 798 src   | (none committed) |
| `integrations/agentic_flow_bridge.ts` | — (no package.json) | — | 1 file ~250 LOC | — |

Effects:

- **Version skew.** `lean-agentic-js` ships at 1.0.0 while the Rust
  crates that back it are at 0.1.0; downstream consumers reading the
  npm version will assume a stable surface that does not exist
  upstream.
- **Multiple `package-lock.json` files** mean each package resolves
  its own `axios`, `typescript`, `ws` versions; minor bumps that
  affect a shared interaction surface drift independently.
- **`integrations/agentic_flow_bridge.ts` has no `package.json`** at
  all — it imports `@midstream/lean-agentic` but is not part of any
  build chain, has no `tsconfig`, no tests, and no CI step.
- **No shared dev tooling.** Each package brings its own
  `eslint`/`prettier`/`jest` config; the three configs drift, and
  changes to one don't propagate.
- **TS tests don't run in CI.** `.github/workflows/rust-ci.yml`
  covers Rust only; the ~1,400 LOC of TS tests in
  `npm/src/__tests__/` only get run by hand.

## Decision Drivers

- **Single dependency graph for JS.** One resolved `axios`, one
  resolved `ws`, one resolved `typescript`.
- **One CI gate** for the JS side; PRs that break a TS test should
  fail the same way as PRs that break a Rust test.
- **No orphan TS files** outside a tsconfig-rooted package.
- **Per-package versioning still wanted.** The three packages have
  legitimately different release cadences.

## Considered Options

1. **Status quo.** Three packages, three lockfiles, drift forever.
2. **Single npm package containing all TS.** Loses per-package
   versioning; the CLI and the lean-agentic client have nothing in
   common.
3. **`pnpm` workspaces.** A root `pnpm-workspace.yaml` lists
   `packages/*`; each package keeps its own `package.json` and
   version. One lockfile, one `node_modules` (deduped), one
   `tsconfig` base, one ESLint/Prettier config, single
   `pnpm -r test` for CI.
4. **`turbo` or `nx` on top of `pnpm` workspaces.** Adds build-graph
   caching; useful at 10+ packages, overkill for 3.
5. **`yarn berry` workspaces.** Same outcome as pnpm; pnpm has
   better disk-deduplication and is the current TC39 recommendation
   for monorepos in 2026.

## Decision Outcome

**Chosen option: Option 3 — `pnpm` workspaces** with the existing
three packages moved under `packages/`:

```
packages/
├── cli/                   (was npm/)             midstream-cli
├── wasm/                  (was npm-wasm/)        @midstream/wasm
└── lean-agentic/          (was lean-agentic-js/) @midstream/lean-agentic
```

`integrations/agentic_flow_bridge.ts` gets a proper home: either
folded into `packages/lean-agentic/src/integrations/` (if it's part of
the lean-agentic public API) or extracted into its own
`packages/agentic-flow-bridge/` package with a `package.json`,
`tsconfig`, and tests.

Tooling:

- One `pnpm-workspace.yaml` at the repo root.
- One root `tsconfig.base.json` with `strict: true`, `noUncheckedIndexedAccess: true`,
  `target: "ES2022"`, `module: "Node16"`. Each package extends it.
- One root `.eslintrc.cjs` and `.prettierrc` shared across packages.
- One root `package.json` with `private: true`, dev tooling
  (`typescript`, `eslint`, `prettier`, `jest`, `vitest`) hoisted.
- `pnpm -r build`, `pnpm -r test`, `pnpm -r lint` run all packages.
- `changesets/changesets` manages per-package versions independently
  (so `lean-agentic` can ship 1.1.0 without bumping the CLI).

Version reconciliation:

- `@midstream/lean-agentic` is **renamed back to 0.x to match the
  Rust crates' stability**. It went out as 1.0.0 prematurely; a
  next release after this ADR ships as 0.2.0 with a deprecation note
  on the old 1.x.
- `@midstream/wasm` likewise drops to 0.2.0 (already partly
  addressed by [ADR-0003](0003-wasm-consolidation.md)).
- `midstream-cli` stays 0.x.

### Positive consequences

- One install, one lockfile, one CI entry-point for the JS side.
- Stray `integrations/agentic_flow_bridge.ts` gains a home.
- Per-package versioning preserved via `changesets`.
- TS tests run in CI (cf. [ADR-0029](0029-js-ci-matrix.md)).

### Negative consequences

- One-time directory move: `npm/` → `packages/cli/` and friends.
  Any external link that points at the old paths breaks.
- `pnpm` requires a global install (`corepack enable pnpm`) on
  contributor machines and in CI; acceptable in 2026.
- `lean-agentic-js` version drop from 1.0 → 0.2 is technically a
  break for the (handful of) external consumers; mitigated by
  shipping a final 1.0.x with a deprecation message pointing at the
  0.2.0 stream.

## Implementation notes

- Land this ADR; do the migration in a follow-up PR titled
  `chore: migrate to pnpm workspaces per ADR-0026`.
- Add `pnpm-workspace.yaml`:
  ```yaml
  packages:
    - "packages/*"
  ```
- Move `npm/` → `packages/cli/`; `npm-wasm/` → `packages/wasm/`;
  `lean-agentic-js/` → `packages/lean-agentic/`. Update relative
  paths (notably the WASM `wasm-pack` output paths used by the CLI's
  `build:wasm` script).
- Adopt `changesets`:
  ```bash
  pnpm dlx @changesets/cli init
  ```
- Update `.github/workflows/rust-ci.yml` (or split into `js-ci.yml`)
  to run `pnpm -r build && pnpm -r test && pnpm -r lint` on Node 18,
  20, and 22 (cf. [ADR-0029](0029-js-ci-matrix.md)).
- Reconcile per-package `tsconfig.json` files: each `extends:
  "../../tsconfig.base.json"` and overrides only paths/lib/types.

## Links

- Related: [ADR-0003](0003-wasm-consolidation.md),
  [ADR-0024](0024-semver-and-api-stability.md),
  [ADR-0027](0027-rust-js-boundary.md),
  [ADR-0029](0029-js-ci-matrix.md),
  [ADR-0030](0030-integrations-directory.md).
- pnpm workspaces: https://pnpm.io/workspaces
- changesets: https://github.com/changesets/changesets
