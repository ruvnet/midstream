# 0036 — Licence reconciliation: dual `MIT OR Apache-2.0` everywhere

- **Status:** Accepted (implemented in #9 — dual MIT OR Apache-2.0)
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** licence, legal, supply-chain

## Context and Problem Statement

The repository carries **three different licence postures simultaneously**:

| Surface                                           | Licence declared          |
|---------------------------------------------------|---------------------------|
| Root `LICENSE` file                               | **Apache-2.0** (`Apache License Version 2.0, January 2004` in the header) |
| Root `README.md` badge (`:5`)                     | **Apache-2.0**            |
| Root `Cargo.toml`                                 | **(no `license` field at all)** |
| `crates/temporal-compare/Cargo.toml:6`            | `license = "MIT"`         |
| `crates/nanosecond-scheduler/Cargo.toml:6`        | `license = "MIT"`         |
| `crates/temporal-attractor-studio/Cargo.toml:6`   | `license = "MIT"`         |
| `crates/temporal-neural-solver/Cargo.toml:6`      | `license = "MIT"`         |
| `crates/strange-loop/Cargo.toml:6`                | `license = "MIT"`         |
| `crates/quic-multistream/Cargo.toml:6`            | `license = "MIT"`         |
| `AIMDS/Cargo.toml:14`                             | `license = "MIT OR Apache-2.0"` |
| `AIMDS/crates/aimds-response/Cargo.toml:7`        | `license = "MIT OR Apache-2.0"` |
| `hyprstream-main/LICENSE`                         | **Apache-2.0** (vendored crate) |
| `npm/package.json` (`midstream-cli`)              | `MIT` (per `:65`)         |
| `lean-agentic-js/package.json` (`@midstream/lean-agentic`) | `MIT`            |

Each published crate currently makes a **legally contradictory
claim**: the crates.io metadata says MIT, but the repository's root
`LICENSE` file (the file that downstream consumers read when the
crate's `repository = "…"` URL takes them to GitHub) says
Apache-2.0. An auditor reading both reasonably concludes either:

- the project is MIT and the `LICENSE` file is wrong, or
- the project is Apache-2.0 and the per-crate metadata is wrong, or
- the project intended dual-licence and got the implementation wrong.

The Rust ecosystem convention since 2015 is **dual `MIT OR Apache-2.0`** —
`tokio`, `serde`, `clap`, the rust-lang stdlib, almost everything
significant follows this. AIMDS, the most recent crates added to the
repo, already follow it. The other crates do not.

Separately, vendoring `hyprstream-main/` ([ADR-0002](0002-unvendor-hyprstream.md))
brings a foreign **Apache-2.0** code drop with no `LICENSE-APACHE` /
`LICENSE-MIT` companion files documenting the dual licence — which is
itself a paperwork gap if we go dual.

## Decision Drivers

- **Internal consistency.** All artefacts shipping from this repo
  must agree on the licence.
- **Ecosystem alignment.** `MIT OR Apache-2.0` is the Rust default;
  consumers expect it.
- **No silent re-licensing.** Whichever direction we move must be a
  deliberate, recorded decision (this ADR + commit history) — not an
  implicit drift.
- **Compatibility with vendored / external code.** Anything we
  vendor or fork must be relicensable into our scheme or stay under
  its own clear notice.

## Considered Options

1. **Status quo.** Three licence stories. Audit risk; trust risk.
2. **Make everything `MIT`.** Aligns Cargo metadata with itself, but
   contradicts the existing Apache `LICENSE` file and is more
   permissive than what some contributors may have already signed up
   for.
3. **Make everything `Apache-2.0`.** Aligns Cargo metadata with the
   `LICENSE` file. Loses the Rust-default permissiveness and is a
   downgrade for downstream consumers expecting dual.
4. **Make everything `MIT OR Apache-2.0`** (dual). Rust convention.
   Most permissive for downstream; lets consumers pick.

## Decision Outcome

**Chosen option: Option 4 — `MIT OR Apache-2.0` dual.**

Concrete moves:

1. **Repo root:**
   - Move the existing `LICENSE` → `LICENSE-APACHE`.
   - Add `LICENSE-MIT` with the standard MIT text and the copyright
     line "Copyright (c) 2026 rUv and contributors".
   - Add a top-level `NOTICE` summarising the dual licence:
     ```
     midstream is dual-licensed under MIT OR Apache-2.0 at the
     consumer's choice. See LICENSE-MIT and LICENSE-APACHE.
     ```
2. **Every Cargo manifest** (root + `crates/*`):
   ```toml
   license = "MIT OR Apache-2.0"
   ```
   Use `license.workspace = true` once the workspace consolidation
   from [ADR-0001](0001-single-cargo-workspace.md) lands.
3. **Every `package.json`** (cli, wasm, lean-agentic, agentic-flow-
   bridge per [ADR-0026](0026-typescript-monorepo.md)):
   ```json
   "license": "MIT OR Apache-2.0"
   ```
4. **README badge** updates to `MIT OR Apache-2.0` (shields.io
   supports the SPDX identifier directly).
5. **`hyprstream-main/`** is removed entirely under
   [ADR-0002](0002-unvendor-hyprstream.md). If that ADR cannot land
   first, the vendored copy keeps its own `LICENSE` file unchanged
   and the repo-level NOTICE explicitly carves it out as
   "third-party code under its own licence".
6. **`cargo deny`** ([ADR-0014](0014-supply-chain-pinning.md)) is
   configured to **require** the dual identifier on workspace
   crates and to **allow** the dual set (`MIT`, `Apache-2.0`,
   `MIT OR Apache-2.0`, `Apache-2.0 WITH LLVM-exception`,
   `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`) on transitives — the
   allowlist matches [ADR-0014](0014-supply-chain-pinning.md)'s
   licence section.
7. **Contributor sign-off:** the future `CONTRIBUTING.md` (cf.
   [ADR-0039](0039-governance.md)) states that all contributions
   are received under the dual licence. We do **not** require a
   CLA — the Rust ecosystem norm is to take contributions under the
   project's stated dual licence via DCO sign-off
   (`Signed-off-by:` trailers, enforced by `dco.action` on PRs).

### Positive consequences

- Single, unambiguous licence story across the repo.
- Matches Rust ecosystem norms; downstream picks MIT or Apache as
  they need.
- `cargo deny` enforces the licence boundary on every PR.

### Negative consequences

- Past commits were authored under an unclear licence. Existing
  contributors are implicitly accepting the dual licence by
  continuing to contribute; we do not retroactively re-licence their
  prior work — the dual licence applies to *new* contributions after
  this ADR merges. Old commits remain under whatever the
  then-effective `LICENSE` said.
- One-off paperwork: adding `LICENSE-MIT`, splitting `LICENSE` →
  `LICENSE-APACHE`, updating ~13 manifest files. ~15 minutes of
  mechanical work.
- We must declare the dual choice on the npm side too, which some
  tooling (older yarn versions) parses pedantically.

## Implementation notes

- Land this ADR; mechanical changes in a follow-up PR
  `chore: dual-licence per ADR-0036`.
- The follow-up PR adds the two LICENSE files, the NOTICE, updates
  manifests, updates README badge, and lands a `cargo deny` rule
  matching the allowed licences.
- Add a CI workflow step using `dco.action` to verify
  `Signed-off-by:` on every commit in a PR.

## Links

- Related: [ADR-0002](0002-unvendor-hyprstream.md),
  [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0026](0026-typescript-monorepo.md),
  [ADR-0039](0039-governance.md).
- SPDX expression syntax:
  https://spdx.github.io/spdx-spec/v2-draft/SPDX-license-expressions/
- Why Rust crates dual-licence:
  https://rust-lang.github.io/api-guidelines/necessities.html#crate-and-its-dependencies-have-a-permissive-license-c-permissive
