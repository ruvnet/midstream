# 0014 — Supply-chain hygiene: `cargo audit`/`cargo deny` in CI, version pinning policy

- **Status:** Accepted (implemented in #10; promoted to hard gate in #55)
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** security, supply-chain, ci

## Context and Problem Statement

A fresh `cargo audit` against `Cargo.lock` (date of this ADR) reports:

| Crate                  | Issue                                                                                   |
|------------------------|-----------------------------------------------------------------------------------------|
| `rustls-webpki 0.102.8` | 4 active advisories: RUSTSEC-2026-0049, -0098, -0099, -0104 (CRL parse panic, name-constraint bypass for URI/wildcard, CRL distribution-point matching). Fix: `>=0.103.13`. |
| `lru 0.12.5`           | RUSTSEC-2026-0002 (unsound `IterMut`). Used by `midstreamer-temporal-compare` and `src/midstream`. |
| `bincode 1.3.3`        | Unmaintained.                                                                            |
| `dotenv 0.15.0`        | Unmaintained.                                                                            |
| `paste 1.0.15`         | Unmaintained.                                                                            |
| `rustls-pemfile 1.0.4 + 2.2.0` | Two versions linked; older one unmaintained.                                      |
| `yaml-rust 0.4.5`      | Unmaintained.                                                                            |

`docs/SECURITY_*` reports do not mention any of these. `security-report.json`
at the repo root flags only `.env file not excluded`, which is now
stale (`.env` is in `.gitignore`).

Additionally, `cargo tree --workspace --duplicates`
(from the perf review) shows pervasive version skew:

- **`rustls`**: 0.22.4 (via quinn 0.11) **and** 0.23.40 — two TLS
  stacks compiled.
- `tower` 0.4 + 0.5; `hyper` 0.14 + 1.9; `http` 0.2 + 1.4; `h2` 0.3 +
  0.4; `arrow-*` 53 + 54; `base64` 0.13/0.21/0.22; `hashbrown`
  0.12/0.14/0.15/0.17.
- `thiserror` version skew across workspaces: root pins 2.0,
  `AIMDS/Cargo.toml:42` pins 1.0.

No CI step enforces any of this.

## Decision Drivers

- **Active CVEs in the dependency closure are not acceptable in a
  security-adjacent product.** Especially with `aimds-*` shipping
  next to `rustls-webpki`'s CRL bug.
- **Drift must be detected automatically.** Manual `security-report.json`
  files in repo root are stale within weeks.
- **Workspace consistency.** A single `Cargo.lock` (cf.
  [ADR-0001](0001-single-cargo-workspace.md)) plus enforced
  duplicate-detection eliminates most of the version-skew class.

## Considered Options

1. **Status quo.** Hand-written security reports, no enforcement.
2. **`cargo audit` in CI** as a PR gate (fail on advisory, configurable
   ignore list with expiry dates).
3. **`cargo audit` + `cargo deny check` in CI.** `cargo deny` enforces
   licence allowlist, advisory database, duplicate-version bans, and
   maintainer allowlist in one config (`deny.toml`).
4. **Option 3 + reproducible builds via `cargo vet`** for first-party
   review of every transitive dep. Highest assurance; large up-front
   review cost.

## Decision Outcome

**Chosen option: Option 3 (`cargo audit` + `cargo deny`) for now;
`cargo vet` (Option 4) deferred to a follow-up ADR once the active
CVEs are cleared.**

`deny.toml` is the single source of policy. It enforces:

- **Advisories:** all RUSTSEC entries are fail-on; documented ignore
  entries carry an expiry date (re-justification required after).
- **Licences:** allowlist `Apache-2.0`, `MIT`, `BSD-3-Clause`,
  `ISC`, `Unicode-DFS-2016`. Forbid `GPL-*`, `AGPL-*`, `SSPL-*`.
- **Bans:** specific bans for `openssl-sys` (use rustls),
  `native-tls` (use rustls), `serde-yaml ≤ 0.9.32` (yaml-rust transitive).
- **Duplicates:** error on multiple versions of `rustls`, `tower`,
  `hyper`, `http`, `tokio`, `serde`. Warn on others.
- **Sources:** registries restricted to crates.io; git sources require
  explicit per-dep allowlist with SHA pinning (see
  [ADR-0002](0002-unvendor-hyprstream.md)).

### Positive consequences

- Active CVEs (rustls-webpki, lru) fail CI today, forcing a
  bump-or-document decision at PR time.
- Version skew (two rustls, three hyper, four hashbrown) becomes
  visible and fails CI by default.
- `cargo deny` config doubles as the SBOM input spec for future tooling.

### Negative consequences

- Some bumps are non-trivial. `lru 0.12 → 0.18` changes the `peek_mut`
  return; `arrow 54 → 58` is multi-step. PRs that touch those need
  more review.
- The "duplicate version" error will initially block every PR. Mitigated
  by landing this ADR's enforcement *after* a one-shot dedup PR.

## Implementation notes

- Add `deny.toml` at the repo root with the policy above.
- Add a GitHub Actions step
  (`.github/workflows/ci.yml` or new `audit.yml`):

  ```yaml
  - name: cargo audit
    uses: rustsec/audit-check@v1
    with: { token: ${{ secrets.GITHUB_TOKEN }} }
  - name: cargo deny
    uses: EmbarkStudios/cargo-deny-action@v2
    with: { command: check, log-level: warn }
  ```

- Pin `rustls-webpki >= 0.103.13` via root `[workspace.dependencies]`.
- Pin `lru = "0.18"` (and migrate the API change-points).
- Replace `dotenv = "0.15"` with `dotenvy = "0.15"` (active fork) in
  root `Cargo.toml:33`.
- Replace `yaml-rust` users (transitively via `config = "0.13"`) by
  bumping `config` to `0.15` (uses `yaml-rust2`).
- Document any time-bounded `deny.toml` ignore entry in `docs/adr/`
  with an explicit follow-up.

## Links

- Related: [ADR-0001](0001-single-cargo-workspace.md),
  [ADR-0002](0002-unvendor-hyprstream.md),
  [ADR-0011](0011-quic-tls-verification.md).
- `cargo deny`: https://github.com/EmbarkStudios/cargo-deny
- `cargo audit`: https://github.com/rustsec/rustsec
