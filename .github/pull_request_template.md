<!--
  Thanks for the PR. Fill in the sections below; delete anything that
  doesn't apply.

  Quick links:
   - Contributing guide:  CONTRIBUTING.md
   - ADR index:           docs/adr/README.md
   - Security reports:    DO NOT use a PR — see SECURITY.md
-->

## Summary

<!-- 1–3 sentences. What does this PR do, and why? -->

## Linked ADR / issue

<!--
  If this PR implements or supersedes an ADR, cite it explicitly:
    Implements ADR-0NNN.
    Closes #1234.
  If it changes a public contract, attach the ADR before requesting
  review.
-->

## Checklist

- [ ] Every commit is signed off (`git commit -s`) per the DCO.
- [ ] Commit messages follow Conventional Commits (`feat:` / `fix:` /
      `chore:` / …) and use `!` for breaking changes.
- [ ] CI is green locally (`cargo check --workspace`, relevant tests).
- [ ] Public API changes are documented and, if applicable, have a
      semver bump per ADR-0024.
- [ ] If this PR touches performance-sensitive paths, benches were
      run and the numbers are in the PR description (ADR-0009).
- [ ] If this PR touches the licence story, dependencies, or any
      published-crate metadata, `cargo deny check` was re-run locally
      (ADR-0014).
- [ ] Working tree only contains changes for this PR (no unrelated
      file deletions or moves).

## Test plan

<!--
  Bullet list of what was tested and how. "Existing CI" is fine for
  trivial mechanical changes; otherwise be specific.
-->

- [ ] …

## Out of scope / follow-ups

<!--
  Anything you noticed but decided not to fix in this PR. List as
  issues to file (or note that you already filed them).
-->
