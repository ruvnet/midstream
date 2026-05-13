# 0029 — JS/TS CI matrix: lint, typecheck, test on every PR

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** ci, typescript, quality

## Context and Problem Statement

`.github/workflows/rust-ci.yml` exercises Rust thoroughly: format,
clippy, multi-OS test matrix, multi-crate builds, WASM build,
benchmarks, docs, security audit, coverage.

But the repository ships **~3,300 LOC of TypeScript across three
packages** plus **~1,400 LOC of TS tests in `npm/src/__tests__/`**,
and **none of it runs in CI**:

- No `npm install` / `pnpm install` step.
- No `tsc --noEmit` step (so a broken `.ts` file lands without
  warning until someone runs `npm run build` locally).
- No `jest` step (so a failing TS test lands the same way).
- No `eslint` step (so config drift goes undetected).
- No JS/Node security audit (`npm audit` / `pnpm audit`).
- No `wasm-pack build` step for the WASM crate that consumes the
  Rust workspace.

The Rust CI does build the workspace WASM crates
(`.github/workflows/rust-ci.yml:122-150`), but only via `cargo build
--target wasm32-unknown-unknown` — without `wasm-pack`, the produced
`.wasm` is never wrapped for npm consumers, so a regression in the
npm-side build is invisible.

## Decision Drivers

- **PR feedback loop.** A TS-only change must surface failures at
  PR time, not at release time.
- **Cross-package coherence.** After [ADR-0026](0026-typescript-monorepo.md)
  lands, `pnpm -r build && pnpm -r test` should be the single
  command CI runs.
- **Node version matrix.** Node 18 LTS is supported per
  `npm/package.json:84` (`engines.node >=18.0.0`); CI must prove it.
- **WASM publish-readiness.** Each PR that touches the WASM crate
  should produce a publishable npm package, not just a `.wasm` blob.

## Considered Options

1. **Status quo.** TS untested in CI.
2. **Add a JS CI workflow that runs lint + typecheck + test on a
   Node-version matrix.** Cheapest meaningful coverage.
3. **Option 2 + WASM-pack publish check + npm audit.** Comprehensive.
4. **Bigger: add Playwright e2e for the dashboard.** Future work;
   not justified until the dashboard has a real UI.

## Decision Outcome

**Chosen option: Option 3.**

New workflow `.github/workflows/js-ci.yml`:

```yaml
name: JS / TS CI

on:
  push:
    branches: [main]
  pull_request: {}

jobs:
  lint-and-test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        node: [18, 20, 22]
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with: { version: 9 }
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node }}
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - run: pnpm -r lint
      - run: pnpm -r typecheck     # `tsc --noEmit` per package
      - run: pnpm -r test
      - run: pnpm audit --audit-level high
        continue-on-error: false

  wasm-pack:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - uses: jetli/wasm-pack-action@v0.4.0
      - uses: pnpm/action-setup@v4
        with: { version: 9 }
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: pnpm }
      - run: pnpm install --frozen-lockfile
      - run: pnpm --filter @midstream/wasm build
      - run: pnpm --filter @midstream/wasm test
```

Required updates to the JS packages:

- Each package must expose `lint`, `typecheck`, `test`, and `build`
  scripts in its `package.json`. `typecheck` is `tsc --noEmit`.
- Root `package.json` declares `private: true` and the dev tooling
  (eslint, prettier, typescript, vitest/jest, husky/lint-staged).
- ESLint config: `eslint-plugin-import` to enforce no-cycles, no-
  default-export (TS files), no-deep-relative-imports.
- Jest → `vitest` migration (faster, native ESM, integrates with
  pnpm workspaces) — optional; if not done, current Jest stays.

### Positive consequences

- TS regressions caught at PR time.
- Node 18/20/22 compatibility verified on every PR.
- The WASM npm package proves it actually builds and packs end-to-end.
- `pnpm audit` surfaces npm-side advisories on every PR (mirror of
  the Rust-side `cargo audit` from
  [ADR-0014](0014-supply-chain-pinning.md)).

### Negative consequences

- CI minutes grow. Mitigated by `cache: pnpm` (heavy hit on first run,
  cheap thereafter) and by running only `pull_request` for the matrix
  (single Node on `push`).
- The first PR after this lands will likely have lint and typecheck
  errors to fix. Acceptable.
- The `wasm-pack` job depends on
  [ADR-0003](0003-wasm-consolidation.md) landing first — until the
  three WASM crates collapse, this step picks one and runs it.

## Implementation notes

- Land [ADR-0026](0026-typescript-monorepo.md) first (so `pnpm -r`
  works). If that's a blocker, ship an interim `js-ci.yml` that
  `cd`s into each package and runs `npm ci && npm test` manually.
- Pin all third-party Actions by SHA, per
  [ADR-0014](0014-supply-chain-pinning.md).
- Update the README's CI badge to reference both `rust-ci` and
  `js-ci`.

## Links

- Related: [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0017](0017-release-and-publishing.md),
  [ADR-0026](0026-typescript-monorepo.md),
  [ADR-0003](0003-wasm-consolidation.md).
