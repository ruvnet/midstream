# Project Governance

This document describes how midstream is currently maintained. It is
deliberately short: the project is small, the maintainer set is
small, and an elaborate governance document would just be wishful
thinking.

## Roles

- **Maintainer.** Has merge rights on `main`. Triages issues,
  reviews PRs, makes release calls. Today the maintainer is
  **@ruv**.
- **Contributor.** Anyone who has had a PR merged. Contributors get
  reviewer assignments via `CODEOWNERS` based on the area they work
  in.

There is no "core team", no steering committee, no voting body. If
the maintainer becomes unresponsive for 90+ days, contributors with
substantial merge history (≥10 substantive PRs over 6+ months) may
elect a replacement by majority vote on a public issue.

## Decisions

- **Architectural decisions** are captured in
  [`docs/adr/`](docs/adr/README.md). Anything that crosses a module
  boundary, changes a public contract, or sets a policy needs an ADR.
  Smaller changes don't.
- **Code changes** require a PR with at least one maintainer review,
  passing CI, and DCO sign-off on every commit (see
  [CONTRIBUTING.md](CONTRIBUTING.md)).
- **Disputes about scope or direction** are resolved by:
  1. Discussion on the relevant issue / PR.
  2. A draft ADR if the disagreement is architectural.
  3. Maintainer call if no consensus emerges within a week.

The maintainer's call is not a veto — anyone who disagrees can fork.
But it is a decision that ends the discussion.

## Becoming a maintainer

After ~10 substantive merged PRs across at least 6 months, an active
contributor may be invited to the maintainers team. There is no
formal application. If you think you should be a maintainer and
haven't been invited, ask.

Invited maintainers commit to:

- Reviewing PRs in their area of expertise within 14 days of request.
- Following the security disclosure process in
  [SECURITY.md](SECURITY.md).
- Mentoring new contributors.

Maintainers can step down at any time by opening an issue.

## Releases

Releases are cut by the maintainer following the process in
[ADR-0017](docs/adr/0017-release-and-publishing.md):
`release-plz` opens a release PR; merging it publishes to crates.io
and creates a signed GitHub release.

Security releases follow the same flow but bypass the normal release
cadence — they ship as soon as the fix is reviewed and CI is green.

## Conflicts of interest

If a maintainer is reviewing a PR they have a financial or personal
stake in (e.g. it was authored by an employer's account), they should
recuse themselves and request review from another maintainer or a
trusted contributor.

## Changing this document

Update via PR. Substantive changes require an ADR.
