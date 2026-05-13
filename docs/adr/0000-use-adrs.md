# 0000 — Use Architecture Decision Records (MADR)

- **Status:** Accepted
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** process, governance

## Context and Problem Statement

The `midstream` repository has accumulated ~50 `docs/*_SUMMARY.md`,
`*_REPORT.md`, `*_STATUS.md`, and `*_VALIDATION.md` files over its short
history. They overlap, contradict each other, are mostly undated, and
none of them clearly answer the question *"why does the project look this
way?"* — e.g. why are there three WASM crates, why is AIMDS a sibling
workspace instead of a member, why is `hyprstream-main/` a vendored
copy, why does `src/lean_agentic/` shadow modules already in `crates/`.

When the next contributor (or the next agent) walks in cold, they have no
way to tell which doc is current, what was already considered and
rejected, or what they are allowed to change without breaking an
unwritten constraint.

## Decision Drivers

- **Auditability.** Each non-trivial structural choice must have a
  single, dated, immutable home — not a fan-out of summary markdown.
- **Reversibility.** Decisions need to be challengeable; the format
  should make it cheap to write *"this superseded by ADR-NNNN"* and move
  on, rather than rewriting an essay.
- **Bot-friendliness.** Agents in this repo (claude-flow, ruflo-adr,
  reviewers) already understand MADR-style ADRs and can index, link, and
  verify them.
- **Low ceremony.** ADRs that take a day to write don't get written.
  Target one screen.

## Considered Options

1. **Continue with ad-hoc `docs/*.md` reports.** Zero new process. Keeps
   the existing 50+ files as the source of truth.
2. **Use Nygard-style ADRs.** Minimal sections (Context, Decision,
   Consequences). Lowest possible overhead.
3. **Use MADR 4.0.** Adds "Decision Drivers" and "Considered Options"
   sections, supports supersession, has tooling and bot support.
4. **Use full IETF-style RFCs.** Highest rigor; far too heavyweight for
   a single-repo project.

## Decision Outcome

**Chosen option: MADR 4.0**, because the explicit *Decision Drivers* and
*Considered Options* sections capture the reasoning that the current
`docs/*_REPORT.md` files routinely omit, and the format is supported by
the agents already wired into this repo (e.g. `ruflo-adr:adr-create`,
`adr-architect`).

### Positive consequences

- A single, ordered, indexed history of architectural decisions lives in
  `docs/adr/`.
- Decision provenance survives PR merges; old decisions remain readable
  as `Superseded` rather than being silently rewritten.
- Reviewers can demand an ADR for any PR that crosses a module boundary
  or changes a public crate's contract.
- New contributors read `docs/adr/README.md` instead of trying to
  reconcile contradictory summary docs.

### Negative consequences

- One more thing to write. Mitigated by the one-screen target and the
  template at `docs/adr/TEMPLATE.md`.
- The 50+ existing `docs/*_REPORT.md` files don't disappear by writing
  ADRs — they still need to be triaged (kept, archived, or deleted) in a
  follow-up.

## Implementation notes

- `docs/adr/README.md` carries the index table.
- `docs/adr/TEMPLATE.md` is the canonical template.
- ADRs are numbered with 4-digit zero-padded prefixes starting at 0000
  (this file). 0001+ are the substantive decisions.
- PR template will be updated in a follow-up to link to this directory.

## Links

- MADR project: https://adr.github.io/madr/
- Michael Nygard's original "Documenting Architecture Decisions" post:
  https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions
