# 0003 — One canonical WASM crate, three publish targets

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** wasm, packaging, releases

## Context and Problem Statement

The repository ships **three different WASM crates** with overlapping
scope and no documented division of labour:

| Path              | Crate name           | Depends on            | Apparent purpose                                                                |
|-------------------|----------------------|-----------------------|---------------------------------------------------------------------------------|
| `wasm/`           | `lean-agentic-wasm`  | none of `midstream`   | A WASM port of the Lean-Agentic system with hand-rolled WebSocket + SSE         |
| `wasm-bindings/`  | `midstream-wasm`     | `midstream` (path)    | "WASM bindings for MidStream Lean Agentic Learning System"                      |
| `npm-wasm/`       | (Rust crate inside the npm package `@midstream/wasm`) | midstreamer-* crates  | NPM-distributed WASM build targeting web, bundler, and nodejs via `wasm-pack`   |

All three:

- export overlapping symbols around `LeanAgentic*` / scheduling / temporal,
- pull in `wasm-bindgen`, `js-sys`, `web-sys` with overlapping but not
  identical feature sets,
- are excluded from the root `[workspace.members]` so `cargo check
  --workspace` does not catch breakage in any of them.

Consequence: a user looking for "the WASM binding" cannot tell which one
to use, the three diverge over time, and a fix in one does not propagate
to the others.

## Decision Drivers

- **Single canonical implementation** of the Rust→JS surface, with one
  authoritative `#[wasm_bindgen]` signature per concept.
- **Multiple distribution targets** still need to be supported: ESM for
  bundlers, `web` for direct browser, `nodejs` for server-side, and a
  bare-`.wasm` artefact for non-Node hosts (e.g. Cloudflare Workers).
- **No fragmentation of the JS API.** Every published surface must
  re-export the same TypeScript types from the same `.d.ts`.

## Considered Options

1. **Status quo: three crates, three teams of one.** No code change.
   The divergence grows.
2. **One canonical crate at `crates/midstream-wasm`** (workspace
   member), with the `wasm-pack` build matrix producing the three
   distribution artefacts (`--target web`, `--target bundler`,
   `--target nodejs`). `npm-wasm/` becomes the **npm package
   directory** only — no Rust source, just `package.json`, the
   generated `pkg-*/` outputs, and an `index.{js,mjs,d.ts}` that
   re-exports.
3. **Two crates: `crates/midstream-wasm-core`** (logic +
   `#[wasm_bindgen]`) and **`crates/midstream-wasm-net`** (the
   WebSocket/SSE/ReadableStream glue that depends on `web-sys`). Useful
   only if downstream consumers want one without the other. No evidence
   yet that they do.
4. **No WASM in-tree; publish externally.** Move the WASM crate to a
   sibling repo and depend on midstream by published version. Cleanest
   separation; loses the ability to keep the WASM surface in lockstep
   with the native API.

## Decision Outcome

**Chosen option: Option 2 — one canonical `crates/midstream-wasm` crate;
`wasm/`, `wasm-bindings/`, and the Rust source under `npm-wasm/` all
disappear in favour of it.** `npm-wasm/` survives as a pure-JS package
directory (build scripts + generated artefacts + `package.json`).

### Positive consequences

- One `#[wasm_bindgen]` surface to maintain.
- One `Cargo.toml` whose `web-sys` features list is authoritative.
- `cargo check --workspace` includes the WASM crate (via
  `[workspace.members]`, kept out of `default-members` so native
  `cargo check` is unaffected).
- A single `npm publish` (`@midstream/wasm`) ships ESM + CJS + bare
  WASM; consumers don't have to choose between three packages.

### Negative consequences

- Migration touches three directories and an npm package. Any external
  consumer importing `lean-agentic-wasm` or `midstream-wasm` directly
  must be redirected (cf. `npm deprecate`).
- The WASM crate ends up with a larger `web-sys` feature footprint
  than any of the three current crates had individually. Compile time
  for WASM CI increases proportionally.

## Implementation notes

- Inventory the public `#[wasm_bindgen]` symbols across `wasm/src/*.rs`,
  `wasm-bindings/src/*.rs`, and `npm-wasm/src/*.rs`. Merge into
  `crates/midstream-wasm/src/lib.rs` with one signature per concept.
- `wasm-pack build --target {web,bundler,nodejs}` runs three times in
  the npm-wasm `build` script; outputs into `pkg-{web,bundler,node}/`.
- `npm-wasm/index.{mjs,cjs}` re-exports from `pkg-bundler` (default)
  with `package.json` `exports` map routing per environment.
- Old crate names (`lean-agentic-wasm`, `midstream-wasm` from
  `wasm-bindings`) get `[deprecated]` README notes pointing at
  `@midstream/wasm` and `crates/midstream-wasm`.

## Links

- Related: [ADR-0001](0001-single-cargo-workspace.md).
- `wasm-pack` target docs:
  https://rustwasm.github.io/docs/wasm-pack/commands/build.html
- npm `exports` conditional exports:
  https://nodejs.org/api/packages.html#conditional-exports
