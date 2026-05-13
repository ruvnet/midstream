# 0017 — Release model: `cargo-release` + `git-cliff`, drop hand-written publish scripts

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** release, ci, publishing

## Context and Problem Statement

Releasing midstream today involves **four hand-written shell scripts
and one broken GitHub Actions workflow**:

```
publish_aimds.sh
publish_aimds_crates.sh
publish_midstream_crates.sh
publish_midstreamer_crates.sh
```

…and `.github/workflows/release.yml`, which has multiple concrete
defects:

1. **Wrong crate names.** The publish step (`release.yml:213-233`)
   runs `cargo publish -p temporal-compare`, `-p nanosecond-scheduler`,
   etc. — but the crates were renamed to `midstreamer-temporal-compare`,
   `midstreamer-scheduler`, etc. (per
   `MIDSTREAMER_RENAME_STATUS.md`). The workflow as written will fail
   or publish to the wrong crate names.
2. **Deprecated GitHub Actions.** `actions/create-release@v1`
   (`release.yml:55`) and `actions/upload-release-asset@v1`
   (`release.yml:130-138`) are archived; both should be replaced by
   `softprops/action-gh-release@v2` or `gh release create`.
3. **`--allow-dirty` everywhere** (`release.yml:217-232`). This
   bypasses the working-tree check, which is the *only* safety net
   between "someone edited a file" and "a wrong artifact ships".
4. **`sleep 10` between publishes** (`release.yml:218-232`). Brittle.
   crates.io propagation can take much longer; the right answer is
   `cargo publish --no-verify --token … && cargo wait --crate …` or
   `cargo-release`'s built-in `publish.wait-after-publish` knob.
5. **Hardcoded topological order in shell.** The DAG lives in
   `Cargo.toml`; encoding it in YAML is duplication that drifts.
6. **`sed -i` to bump versions** (`release.yml:198-205`). Easy to get
   wrong; `cargo set-version` (from `cargo-edit`) or `cargo-release`'s
   `cargo release version` does this safely.
7. **No changelog discipline.** `git-cliff` is installed inside the
   workflow (`release.yml:42-49`) but no `cliff.toml` exists in the
   repo, so it picks up defaults — chronological dumps of every
   commit, no conventional-commits filtering.
8. **No SBOM, no provenance, no signed releases.** Important for a
   security-adjacent product.

## Decision Drivers

- **Single source of truth** for crate versions and publish ordering
  (the workspace manifest, not shell scripts or YAML).
- **Fail-safe defaults.** A release must refuse to ship on a dirty
  tree, on a tag that doesn't match the manifest version, or on a
  missing CHANGELOG entry.
- **Provenance.** Each release must produce a signed artefact (GitHub
  attestations) and an SBOM (CycloneDX) for downstream audit.
- **Reproducibility.** Re-running the workflow on the same tag must
  produce identical artefacts.

## Considered Options

1. **Keep shell scripts + current workflow.** Status quo. Known
   defects persist.
2. **`cargo-release`** as the canonical local + CI release tool. It
   derives topo order from `Cargo.toml`, handles version bumps,
   tagging, changelog regeneration, and crate publish with `--wait`.
3. **`release-plz`** (release-please for Rust). PR-based release flow:
   bot opens a release PR, you merge it. Higher ceremony, very robust
   for multi-crate workspaces.
4. **Hand-roll everything in `release.yml`** with `cargo publish
   --wait-for-publish` (nightly). Avoids new tools; doesn't solve the
   topo-order or version-bump problem.

## Decision Outcome

**Chosen option: Option 3 — `release-plz` as the orchestrator, with
`cargo-release` knobs available for one-off manual cuts.** A
`release-plz` config at the repo root drives PR-based releases;
`cargo-release` stays installable for emergency local cuts.

Concretely:

- Delete `publish_*.sh` (4 files) and the four steps in
  `release.yml:213-233`.
- Add `release-plz.toml` listing every workspace member (so
  `midstreamer-temporal-compare` etc. are named correctly), with
  `release_commits = "^feat:|^fix:|^perf:|^BREAKING:"` so docs-only
  commits don't trigger releases.
- Add `cliff.toml` with conventional-commits parsing, group templates,
  and link templates for the project's GitHub issues/PRs.
- `release.yml` shrinks to:
  - Validate (`cargo audit`, `cargo deny check`, `cargo test
    --workspace`, build matrix).
  - Run `release-plz release` (publishes, tags, GH release).
  - Build cross-target binaries and attach to the GH release via
    `softprops/action-gh-release@v2`.
  - Generate CycloneDX SBOM via `cargo cyclonedx --all --output-file
    sbom.cdx.json` and attach.
  - Generate GitHub provenance attestations.

### Positive consequences

- Crate names live in `Cargo.toml` and `release-plz.toml` only. No
  shell-script copy.
- Changelog is auto-generated from conventional-commits; quality of
  the release notes becomes a property of commit-message discipline.
- SBOM + provenance ship with every release. Downstream consumers
  can verify provenance via `gh attestation verify`.
- One-line emergency manual cut still available
  (`cargo release patch --workspace --execute`).

### Negative consequences

- Conventional-commits discipline must be enforced (pre-commit hook +
  CI lint).
- One-time migration: every crate's existing version becomes the
  starting point; the first `release-plz` PR will look noisy.
- `release-plz` opens a PR per crate-bump cycle; reviewers must learn
  the workflow.

## Implementation notes

- Delete: `publish_aimds.sh`, `publish_aimds_crates.sh`,
  `publish_midstream_crates.sh`, `publish_midstreamer_crates.sh`.
- Add: `release-plz.toml`, `cliff.toml`, `commitlint.config.js` (or a
  `.github/workflows/commit-lint.yml` using `wagoid/commitlint-github-action`).
- Rewrite `.github/workflows/release.yml` to ~80 lines using:
  - `MarcoIeni/release-plz-action@v0.5`
  - `softprops/action-gh-release@v2`
  - `actions/attest-build-provenance@v1`
  - `CycloneDX/cargo-cyclonedx` for SBOM.
- Pin all third-party actions by SHA, not by tag (supply-chain
  hygiene; cf. [ADR-0014](0014-supply-chain-pinning.md)).

## Links

- Related: [ADR-0001](0001-single-cargo-workspace.md),
  [ADR-0014](0014-supply-chain-pinning.md).
- `release-plz`: https://release-plz.ieni.dev/
- `cargo-release`: https://github.com/crate-ci/cargo-release
- `git-cliff`: https://git-cliff.org/
- `cargo-cyclonedx`: https://github.com/CycloneDX/cyclonedx-rust-cargo
