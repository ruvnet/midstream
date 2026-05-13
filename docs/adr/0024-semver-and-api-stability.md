# 0024 — Semver discipline and public-API stability policy

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** policy, semver, releases

## Context and Problem Statement

Every Rust crate in the workspace is pinned at `version = "0.1.0"`:

```
Cargo.toml:                                version = "0.1.0"
crates/quic-multistream/Cargo.toml:        version = "0.1.0"
crates/strange-loop/Cargo.toml:            version = "0.1.0"
crates/temporal-attractor-studio/Cargo.toml: version = "0.1.0"
crates/temporal-compare/Cargo.toml:        version = "0.1.0"
crates/nanosecond-scheduler/Cargo.toml:    version = "0.1.0"
crates/temporal-neural-solver/Cargo.toml:  version = "0.1.0"
```

…and the README claims 5 are already published. So the public surface
has been frozen at `0.1.0` while the codebase keeps changing.

Effects:

- **Every breaking change goes out as a `0.1.x` patch.** Cargo
  treats `0.y.z` such that `0.1.5` may break `0.1.3` consumers
  *technically* (`0.x` semver is "anything can change") — but in
  practice downstream consumers pin `^0.1` and break anyway.
- **No `[[deprecated]]` discipline.** Items get renamed silently
  (e.g. the impending `TemporalError` → `CompareError`/`SolverError`
  rename in [ADR-0018](0018-error-policy.md)).
- **No public API surface tests.** A change to a `pub fn` signature
  is not detected as a breaking change until a downstream consumer
  reports it.
- **No documented stability tier.** Some crates are conceptually
  "experimental, expect breakage" (`strange-loop`) and others are
  more stable (`temporal-compare`). Both ship as `0.1.0`.

## Decision Drivers

- **Predictable breakage.** Consumers should be able to pin a version
  range and trust it.
- **Clear stability tiers.** Per-crate "alpha / preview / stable" so
  consumers can choose how much breakage they're willing to absorb.
- **Detection.** Breaking changes detected in CI, not in the wild.

## Considered Options

1. **Status quo.** Every change is a `0.1.x` patch. No tools.
2. **Adopt strict semver per crate** with `cargo-semver-checks`
   enforced in CI. Bump rules:
   - any breaking change → minor bump in `0.x` (`0.1 → 0.2`); major in `1.x+`,
   - any new public item → minor bump,
   - any internal-only change → patch.
3. **Option 2 + stability tiers (alpha / beta / stable).**
   Per-crate documented level; alpha crates may waive semver in
   exchange for an opt-in feature flag (`use-alpha-api`).
4. **Move directly to `1.0.0` on every crate.** Forces semver
   discipline immediately; loses the implicit "early days" signal of
   `0.x`.

## Decision Outcome

**Chosen option: Option 3 — strict semver enforced by
`cargo-semver-checks`, with per-crate stability tiers.**

Per-crate stability tier table (initial mapping):

| Crate                                       | Tier   | Current | Next bump |
|---------------------------------------------|--------|---------|-----------|
| `midstreamer-temporal-compare`              | beta   | 0.1.0   | 0.2.0 when ADR-0006/0008 land |
| `midstreamer-scheduler`                     | beta   | 0.1.0   | 0.2.0 after ADR-0008 lock-free rewrite |
| `midstreamer-quic-multistream`              | alpha  | 0.1.0   | 0.2.0 after ADR-0011 TLS hardening + ADR-0021 feature flag |
| `midstreamer-attractor` (`temporal-attractor-studio`) | alpha | 0.1.0 | 0.2.0 |
| `midstreamer-neural-solver` (`temporal-neural-solver`) | alpha | 0.1.0 | 0.2.0 |
| `midstreamer-strange-loop`                  | alpha  | 0.1.0   | 0.2.0 |
| `midstream` (top-level crate/binary)        | alpha  | 0.1.0   | 0.2.0 after ADR-0006 / ADR-0007 / ADR-0016 |

Tier semantics:

- **alpha:** breaking changes allowed in any release; consumers
  expected to track the latest tagged release; deprecations may be
  zero-cycle (deleted in the same release as the rename).
- **beta:** breaking changes require a minor bump and a deprecation
  cycle of ≥1 release (item marked `#[deprecated]`, then removed in
  the next minor).
- **stable (post-1.0):** breaking changes require a major bump and a
  deprecation cycle of ≥2 releases.

`cargo-semver-checks` runs in CI on every PR that touches `crates/`
or `src/`. Breaking changes flagged with a clear bump-required hint.

### Positive consequences

- Consumers can read the README, see the tier, and choose accordingly.
- `cargo-semver-checks` mechanically detects breaking changes; PRs
  that lower the bar fail at review time.
- The tier system lets `strange-loop` (genuinely experimental) move
  fast without dragging the more-mature `temporal-compare` into
  breakage.

### Negative consequences

- `cargo-semver-checks` is itself in active development; occasional
  false positives. Document the override in PR description; require
  reviewer signoff.
- Per-crate tier requires per-crate care at bump time; the release
  PR template (cf. [ADR-0017](0017-release-and-publishing.md)) must
  surface the tier in its checklist.

## Implementation notes

- Add `cargo-semver-checks` to CI:

  ```yaml
  - uses: obi1kenobi/cargo-semver-checks-action@v2
    with:
      crate-name: ${{ matrix.crate }}
  ```

  Matrix one entry per workspace member.
- Add a `stability` keyword in each crate's `[package.metadata.docs.rs]`
  (rendered on docs.rs and in the crate description on crates.io).
- The first release after this ADR lands bumps every crate per the
  table above. Subsequent bumps follow the tier rules.
- Add `cargo-public-api` to a manual-run maintenance script for
  quarterly diff review.

## Links

- Related: [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0017](0017-release-and-publishing.md),
  [ADR-0018](0018-error-policy.md),
  [ADR-0023](0023-msrv-policy.md).
- `cargo-semver-checks`: https://github.com/obi1kenobi/cargo-semver-checks
- `cargo-public-api`: https://github.com/Enselic/cargo-public-api
