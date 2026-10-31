# 0042 — aidefence 3.0: publish from main, side-effect-free library entry

- **Status:** Accepted
- **Date:** 2026-10-31
- **Deciders:** @ruvnet
- **Tags:** security, release, aimds

## Context and Problem Statement

npm `aidefence@2.3.0` (and its alias `aidefense`) is the AIMDS TypeScript package in `AIMDS/`. Three problems surfaced in a ruvnet-ecosystem dependency review (ruflo ADR-513):

1. **Not built from main.** The tarball's `gitHead` is `3145670` on `ops/aimds-deep-review`. That branch is five commits ahead of its base and never merged: the RUSTSEC-2024-0421 fix (`validator` 0.20), 29 npm advisory fixes, and the gateway changes that made the package publishable. The published manifest (`name: aidefence`, `version: 2.3.0`, `files`) matches no committed file; it was set on a dirty tree at publish time. Meanwhile main gained the pattern-pack engine (#105) and the Rust `aidefence-core` (#102). Neither has ever been published to npm.
2. **Importing the library starts a server.** `main` was `dist/index.js`, which called `main()` at module load. So `require('aidefence')` built an Express gateway and listened on `0.0.0.0:3000` (all interfaces) as a side effect. Library consumers could not use the detector without opening a network listener.
3. **The detector is not exported.** `InjectionDetector` and the pack loader exist under `src/detection/`, but the package entry never exported them. The 53-pattern packs (55/55 on the review corpus, against 7/55 for the old vendored detector) were unreachable from npm.

## Decision Drivers

- A published version must be reproducible from main.
- Importing a security library must not open a port.
- Detection should be usable as a plain library by ruflo, Slack bots and other consumers.

## Considered Options

1. **Republish 2.3.x from the branch.** This keeps main and npm diverged and still omits the packs.
2. **Publish main as 2.4.0 without the branch.** This drops the RUSTSEC and npm advisory fixes and the publishable gateway.
3. **Merge the branch into main, split the entry, publish 3.0.0.** This is a major version because `main` no longer starts a server.

## Decision Outcome

**Chosen option 3**, because it is the only option where npm equals main and nothing already shipped is lost.

- `ops/aimds-deep-review` is merged into main. There were two `Cargo.toml` conflicts: main's 0.2.0 crate versions are kept, and the branch's `validator = "0.20"` is taken alongside main's `async-trait`.
- `src/index.ts` is now a pure library entry: `export * from './detection'`, `AIMDSGateway`, and the types. It has no top-level side effects.
- The server moves to `src/server.ts`, exposed as the `aidefence` bin and used by `npm start`. Its default `HOST` is `127.0.0.1`; set `HOST=0.0.0.0` to expose it deliberately.
- The manifest is committed as published: `name: aidefence`, `version: 3.0.0`, `bin`, `types`, `files: [dist, patterns, README.md, CHANGELOG.md]`. `patterns/` ships, because `pattern-loader` resolves `<package>/patterns` from `dist/detection`.

### Consequences

- **Breaking:** code that relied on `require('aidefence')` to start the gateway must run `npx aidefence` (or `node node_modules/aidefence/dist/server.js`) instead. Deployments that relied on the all-interfaces default must set `HOST=0.0.0.0`.
- Verified on a packed tarball installed into an empty project:
  - `require('aidefence')` leaves 0 active servers.
  - `createInjectionDetector()` loads all 6 packs (53 patterns).
  - `npx aidefence` listens on `127.0.0.1` and `/health` reports healthy.
- AIMDS test suite after the merge: 134 passed, 1 skipped, 0 failed. `cargo check` is clean.
- ruflo's `@claude-flow/aidefence` vendors the same packs (ruflo ADR-517), so both lineages read one data set.
