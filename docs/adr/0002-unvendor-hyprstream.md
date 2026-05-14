# 0002 — Stop vendoring `hyprstream`, depend on it externally

- **Status:** Accepted (implemented in #13)
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** dependencies, supply-chain, build

## Context and Problem Statement

The root `Cargo.toml` declares:

```toml
hyprstream = { path = "hyprstream-main" }
```

and `hyprstream-main/` is a **full vendored copy** of an external crate
(its own `Cargo.toml`, `src/`, `examples/`, `config/`, `LICENSE`,
`README.md`) checked into this repo. There is no `.git`, no upstream
remote pointer, no `vendor.toml` describing where it came from or what
SHA was vendored, and no patch series describing local divergence (if
any).

This pattern has three failure modes:

1. **Silent upstream drift.** The upstream `hyprstream` is being
   developed; there is no record of which revision this snapshot maps
   to, and no automated way to refresh it.
2. **License and provenance ambiguity.** The vendored
   `hyprstream-main/LICENSE` is present but `docs/` does not record the
   upstream origin, the SHA, or the date of vendoring. This is brittle
   in any future audit (`cargo about`, FOSSA, SBOM tooling).
3. **Doubles workspace surface.** `hyprstream-main/` adds its own
   `Cargo.toml` and dependency closure. See
   [ADR-0001](0001-single-cargo-workspace.md) for the workspace impact.

## Decision Drivers

- **Reproducible builds.** Any consumer (CI, downstream users, security
  auditors) must be able to point at a single commit or version and get
  exactly the dependency graph we built against.
- **Patchability.** If we *do* need local changes to `hyprstream`, they
  should live as a small, reviewable patch series — not a silent fork.
- **Audit trail.** The supply-chain story for `hyprstream` should match
  the supply-chain story for every other dependency: name + version +
  checksum, anchored in `Cargo.lock` and surfaceable via `cargo audit`
  and `cargo deny`.

## Considered Options

1. **Keep vendoring as-is.** Easiest. Zero migration cost. Ongoing
   audit and drift cost forever.
2. **Vendor with explicit metadata.** Keep `hyprstream-main/` but add
   a `hyprstream-main/.upstream` file recording the upstream URL, SHA,
   and date. Add a periodic CI job that diffs against upstream and
   warns on drift. Cheap; addresses provenance but not maintainability.
3. **Depend on `hyprstream` by `git = "…", rev = "…"`.** Pins to a
   specific commit, refresh is `cargo update -p hyprstream`. Standard
   pattern for not-yet-published forks.
4. **Depend on `hyprstream` by published version (`hyprstream = "X.Y"`).**
   Requires the upstream to ship a release; cleanest answer if available.
5. **Submit local patches upstream and depend on the published crate.**
   Highest-quality outcome; requires upstream cooperation.

## Decision Outcome

**Chosen option: Option 4 (published version) if a compatible version
exists on crates.io; otherwise Option 3 (git+rev pin) as a stepping
stone.** Option 5 is the long-term ideal but cannot be a precondition
because it depends on a third party.

The vendored `hyprstream-main/` directory is removed in the same change.
Any local divergence from upstream is captured as a patch series at
`patches/hyprstream/*.patch` and applied via a build script or — better
— a `[patch.crates-io]` section pointing at a fork repo whose branch
holds the applied patches.

### Positive consequences

- Single, auditable upstream origin for `hyprstream`. `cargo audit` and
  `cargo deny` cover it like every other dep.
- The repo loses ~one whole crate's worth of source, build artefacts,
  and workspace complexity.
- Refreshing `hyprstream` becomes a one-line bump in `Cargo.toml`
  instead of an interactive rebase against an undocumented snapshot.

### Negative consequences

- We lose the ability to silently hot-patch `hyprstream` from inside
  this repo. Any change must be a real patch, reviewed and applied
  through `[patch.crates-io]` or a fork branch.
- If no compatible upstream version exists, we are tied to either a
  git+rev pin (which `cargo audit` reports as "unknown source") or to
  publishing our own fork crate (extra release work).

## Implementation notes

- Audit `hyprstream-main/` against the upstream repo
  (https://github.com/jmanteau/hyprstream or whichever fork is canonical
  for this project) to identify local divergence.
- If divergence is zero: bump root `Cargo.toml` to `hyprstream = "X.Y"`,
  delete `hyprstream-main/`.
- If divergence is non-zero: turn each delta into a patch file under
  `patches/hyprstream/`, push the applied set as a fork branch, and use
  `[patch.crates-io]` to redirect to that fork. Refresh quarterly.
- Update `docs/SECURITY_AUDIT_SUMMARY.md` and the SBOM (when
  introduced) to reflect the new origin.

## Links

- Related: [ADR-0001](0001-single-cargo-workspace.md) (single workspace).
- Cargo patch overrides:
  https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html
