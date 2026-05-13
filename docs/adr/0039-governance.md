# 0039 — Governance: CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, CODEOWNERS

- **Status:** Accepted (implemented in #12)
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** governance, process, security

## Context and Problem Statement

The repository ships **no governance documents at all**:

```
$ ls CONTRIBUTING* CODE_OF_CONDUCT* SECURITY* GOVERNANCE* CODEOWNERS \
     .github/CONTRIBUTING* .github/CODE_OF_CONDUCT* \
     .github/SECURITY* .github/CODEOWNERS
ls: cannot access … : No such file or directory
```

Absent: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`,
`GOVERNANCE.md`, `CODEOWNERS`, and the GitHub issue / PR templates
under `.github/ISSUE_TEMPLATE/` and `.github/pull_request_template.md`.

Effects:

- **Contributors don't know how to contribute.** No build / test /
  PR-style guidance; no DCO/CLA statement
  (cf. [ADR-0036](0036-license-reconciliation.md)).
- **Security researchers don't know how to report.** A real
  `SECURITY.md` lists a contact (email or GitHub security
  advisories), an SLA for triage, supported versions, and the public-
  disclosure timeline. Without it, the only options are filing a
  public issue (bad) or guessing.
- **Reviewers don't auto-route.** No `CODEOWNERS` means the right
  person isn't auto-requested on PRs touching the QUIC crate, the
  AIMDS crates, the WASM bindings, etc.
- **No issue templates.** "Bug report" issues land as freeform prose
  with no repro section; triage takes longer.
- **No Code of Conduct.** Many downstream packagers (Linux distros,
  CNCF-style projects) treat missing CoC as an adoption blocker.

This matters now because the project is publishing to crates.io and
npm with real users.

## Decision Drivers

- **Discoverability.** Standard files in standard locations so
  GitHub renders the right banners ("New contributor — read
  CONTRIBUTING.md") and `gh repo view` surfaces them.
- **Security-disclosure path.** Coordinated disclosure must be a
  paved road.
- **Auto-routed review.** PRs to security-sensitive crates auto-
  request the security-architect; PRs to AIMDS auto-request the AIMDS
  maintainer.
- **No invented bureaucracy.** Documents must be short and
  actionable, not aspirational essays.

## Considered Options

1. **Status quo.** No governance docs. Inconvenience and risk grow.
2. **Add the standard set with the Contributor Covenant** (CoC),
   `coordinated disclosure` SECURITY.md, DCO-signed-off
   contributions, plain CONTRIBUTING + CODEOWNERS. Industry-standard
   baseline.
3. **Add the standard set + a heavier governance document**
   (steering committee, voting, release manager rotation). Overkill
   for a single-author-plus-contributors project.

## Decision Outcome

**Chosen option: Option 2 — the standard set, no more.**

### Files to add

| Path                                    | Content                                                                              |
|-----------------------------------------|--------------------------------------------------------------------------------------|
| `CONTRIBUTING.md`                       | How to clone, build (`cargo xtask install`), test (`cargo xtask ci`), open a PR; DCO `Signed-off-by:` requirement; commit-message convention (Conventional Commits per [ADR-0017](0017-release-and-publishing.md)); link to ADR index. |
| `CODE_OF_CONDUCT.md`                    | Contributor Covenant v2.1 verbatim; enforcement contact = `conduct@<project>` (set up via Anubis/GitHub email forwarding). |
| `SECURITY.md`                           | Reporting contact (GitHub security advisories preferred; backup email); supported versions (per [ADR-0024](0024-semver-and-api-stability.md) stability tiers — only `beta` and `stable` crates get security backports); 90-day coordinated-disclosure default; PGP key fingerprint. |
| `GOVERNANCE.md`                         | One screen: maintainer = `@ruv`; PRs merged by maintainer; major changes via ADR; how to become a maintainer (history of merged PRs + maintainer invite). |
| `CODEOWNERS`                            | Routing: `crates/quic-multistream/* @ruv @security-team`; `AIMDS/* @ruv @aimds-team`; `crates/midstreamer-mcp-* @ruv @mcp-team`; `docs/adr/* @ruv`. Teams resolve to whatever GitHub teams exist; for now they're effectively `@ruv`. |
| `.github/pull_request_template.md`      | Checklist: linked issue?; ADR updated if architectural?; tests added?; benches gated if perf-sensitive?; `Signed-off-by:` present?. |
| `.github/ISSUE_TEMPLATE/bug_report.yml` | Structured: repro steps, expected, actual, environment, version. |
| `.github/ISSUE_TEMPLATE/feature_request.yml` | Structured: problem statement, proposed solution, alternatives considered. |
| `.github/ISSUE_TEMPLATE/security_report.yml` | Locked to a notice that says "**Do not file security issues here. Use a security advisory instead.**" with a link. Stops accidental public disclosure. |
| `.github/dependabot.yml`                | Cargo + npm + GitHub Actions update PRs; weekly; grouped by ecosystem. |

### Process commitments

- **DCO over CLA.** Contributions accepted under the dual licence
  ([ADR-0036](0036-license-reconciliation.md)) via per-commit
  `Signed-off-by:` trailers. Enforced by `dco.action` on PRs.
- **Conventional Commits enforced** by `commitlint` ([ADR-0017](0017-release-and-publishing.md)).
- **Triage SLO**: bug reports acknowledged within 14 days; security
  reports within 72 hours. Misses logged in `docs/triage-log.md` so
  the SLO is honest.
- **Maintainer onboarding**: after ~10 substantive merged PRs over
  6+ months, a maintainer may invite a contributor to the
  maintainers team. Documented in GOVERNANCE.md.

### Positive consequences

- Discoverable contribution path; lower friction for new
  contributors.
- A real security-reporting channel.
- Reviewers automatically pulled in on relevant PRs.
- Dependabot keeps deps fresh without manual sweeps.

### Negative consequences

- One-time writing effort (~half a day for all files).
- We commit to triage SLOs; if we miss them, we say so publicly in
  `docs/triage-log.md`. Honesty has a cost.
- CodeOwners gates merges (requires owner review). Mitigated by
  keeping owners small and active; can be relaxed for non-protected
  paths.

## Implementation notes

- Land this ADR; write the files in a follow-up PR
  `docs: add governance documents per ADR-0039`.
- For `SECURITY.md`, enable **GitHub Security Advisories** on the
  repo settings page in the same PR so the file's "report here" link
  is live.
- For `dependabot.yml`, group Cargo updates so we don't get N PRs per
  week on minor bumps.

## Links

- Related: [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0017](0017-release-and-publishing.md),
  [ADR-0024](0024-semver-and-api-stability.md),
  [ADR-0036](0036-license-reconciliation.md).
- Contributor Covenant: https://www.contributor-covenant.org/
- GitHub security advisories docs:
  https://docs.github.com/en/code-security/security-advisories
- DCO sign-off: https://developercertificate.org/
