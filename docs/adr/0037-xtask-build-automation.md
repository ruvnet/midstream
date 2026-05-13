# 0037 — `xtask` build automation: replace shell scripts with Rust

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** automation, build, tooling

## Context and Problem Statement

Repository automation today lives in **a fan of root-level shell
scripts**:

```
install.sh                       (329 B)
setup.sh                         (482 B)
publish_aimds.sh                 (1.7 KB)
publish_aimds_crates.sh          (1.8 KB)
publish_midstream_crates.sh      (3.1 KB)
publish_midstreamer_crates.sh    (3.1 KB)
```

Plus an executable test artefact (`test_exports`, 13 MB) that lives
*at the repo root* and is treated as a dev tool. None of these have:

- a documented contract (inputs, outputs, exit codes),
- error handling beyond shell's default,
- portability (they all assume Linux/macOS bash; Windows contributors
  have to use WSL).

The publish scripts duplicate the topological dependency order that
lives in `Cargo.toml`, encoded as ordered `cargo publish -p NAME &&
sleep 10` chains. Multiple of them coexist (the older
`publish_midstream_crates.sh` and the newer `publish_midstreamer_crates.sh`)
with no signal as to which is current.

[ADR-0017](0017-release-and-publishing.md) proposes replacing these
with `release-plz`, but `release-plz` covers only the publish path.
The non-publish chores — running benches with consistent flags,
generating the schema bundle, building all WASM targets, running
`cargo deny / cargo audit`, regenerating the docs, hashing release
artefacts — have nowhere coherent to live.

The Rust-native solution is the **`xtask` pattern** (Matthias Endler,
matklad): a workspace member named `xtask` whose `main()` parses a
subcommand and runs the chore in Rust. Invoked via `cargo xtask
<cmd>`, which a `.cargo/config.toml` alias makes possible.

## Decision Drivers

- **Cross-platform.** Contributors on Windows shouldn't need bash.
- **Single source of truth.** Topological order, version-bumping
  rules, and CI invocations live in one Rust file, not three shell
  files.
- **Testable.** Rust chores can be unit-tested; shell scripts can't,
  in practice.
- **No surprise deps.** A shell-script chore can shell out to
  anything; an `xtask` chore explicitly lists its dependencies in
  `Cargo.toml`.

## Considered Options

1. **Status quo.** Shell scripts at the root forever.
2. **`Makefile`.** Better than ad-hoc shell; still Linux/macOS only;
   still hard to test.
3. **`cargo-make` (Makefile.toml).** Declarative; popular; another
   tool to install.
4. **`just`.** Modern make-replacement; same install-and-discoverability
   tax.
5. **`xtask` pattern.** Rust workspace member; invoked via
   `cargo xtask`; no extra tool install.

## Decision Outcome

**Chosen option: Option 5 — `xtask`.**

Layout (post-[ADR-0001](0001-single-cargo-workspace.md)):

```
xtask/
├── Cargo.toml          # `name = "xtask"`, `publish = false`
└── src/
    ├── main.rs         # clap-derived CLI dispatch
    ├── bench.rs        # `cargo xtask bench` orchestration
    ├── ci.rs           # `cargo xtask ci` — runs the same gates CI runs
    ├── deny.rs         # wrappers over `cargo deny` / `cargo audit`
    ├── docs.rs         # `cargo xtask docs` regenerate ADR index, SCHEDULER_SLO, etc.
    ├── release.rs      # version bumps + release-plz hand-off
    ├── schemas.rs      # regenerate ts-rs + schemars outputs
    ├── wasm.rs         # `wasm-pack` orchestration
    └── util.rs         # shared helpers (process spawn, paths)
```

`.cargo/config.toml` aliases:

```toml
[alias]
xtask = "run --package xtask --release --"
```

So contributors run `cargo xtask <cmd>` everywhere.

Initial subcommand inventory:

| Subcommand            | Replaces                                                           |
|-----------------------|--------------------------------------------------------------------|
| `cargo xtask ci`      | the GHA `rust-ci.yml` happy path (so local-CI reproduces it)       |
| `cargo xtask bench`   | the `cargo bench --workspace` orchestration with SLO checks ([ADR-0033](0033-scheduler-slo-contract.md)) |
| `cargo xtask deny`    | `cargo audit && cargo deny check` ([ADR-0014](0014-supply-chain-pinning.md)) |
| `cargo xtask docs`    | regenerate `docs/MCP_TOOLS.md`, `docs/SCHEDULER_SLO.md`, `docs/adr/README.md` index |
| `cargo xtask schemas` | regenerate ts-rs + schemars outputs ([ADR-0027](0027-rust-js-boundary.md)) |
| `cargo xtask wasm`    | `wasm-pack build --target {web,bundler,nodejs}` ([ADR-0003](0003-wasm-consolidation.md)) |
| `cargo xtask release` | hands off to `release-plz` ([ADR-0017](0017-release-and-publishing.md)) |
| `cargo xtask install` | dev-machine setup (replaces `install.sh` / `setup.sh`)             |

Deletions:

- `publish_*.sh` × 4 (also covered by [ADR-0017](0017-release-and-publishing.md)).
- `install.sh`, `setup.sh` (chore: their content moves into
  `xtask::install`).
- `test_exports` (the 13 MB binary). It's a build artefact that
  shouldn't be in git.

Dependencies inside the `xtask` crate:

- `clap` (subcommand parsing), `xshell` (process spawn, well-typed
  errors), `anyhow` (error glue), `serde` (config), `walkdir`,
  `cargo_metadata` (workspace introspection, replaces hard-coded
  crate lists).

### Positive consequences

- Cross-platform; Windows works.
- Topological publish order derives from `cargo_metadata` at runtime,
  not from a hand-written list.
- Each chore is testable.
- CI calls `cargo xtask ci` — contributors can run *the same job
  locally* with one command. No more "passes locally, fails in CI"
  surprises.

### Negative consequences

- `xtask` adds compile time (~30s cold, ~5s warm). Mitigated by
  `--release` for distribution but `--profile dev` for fast iteration.
- One more workspace member; the `[default-members]` list excludes
  it from regular `cargo check`.
- One-time migration: each chore's contract gets re-implemented in
  Rust. Order: `ci` first (highest value), then `bench`/`deny`/`schemas`.

## Implementation notes

- Land this ADR; do the migration crate by crate in follow-ups.
- New `xtask/Cargo.toml`:

  ```toml
  [package]
  name = "xtask"
  version = "0.0.0"
  edition = "2021"
  publish = false

  [dependencies]
  anyhow = "1"
  clap = { version = "4", features = ["derive"] }
  xshell = "0.2"
  cargo_metadata = "0.18"
  serde = { version = "1", features = ["derive"] }
  walkdir = "2"
  ```

- Add `.cargo/config.toml` with the alias above.
- The publish scripts come out in the same PR that lands
  [ADR-0017](0017-release-and-publishing.md)'s `release-plz` shift;
  `cargo xtask release` thin-wraps `release-plz` so contributors have
  a uniform entry point.
- `test_exports` is added to `.gitignore` and removed via `git rm`.

## Links

- Related: [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0017](0017-release-and-publishing.md),
  [ADR-0027](0027-rust-js-boundary.md),
  [ADR-0033](0033-scheduler-slo-contract.md).
- `xtask` pattern (matklad): https://github.com/matklad/cargo-xtask
- `xshell`: https://docs.rs/xshell/
