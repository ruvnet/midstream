# 0034 — Workspace-wide lint policy via `[workspace.lints]`

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** lints, quality, policy

## Context and Problem Statement

A grep for `[lints]` / `[workspace.lints]` across every Cargo manifest
in the repo returns **zero hits**:

```
$ grep -cE '^\[(workspace\.)?lints\]' Cargo.toml crates/*/Cargo.toml
Cargo.toml:0
crates/temporal-compare/Cargo.toml:0
crates/temporal-neural-solver/Cargo.toml:0
crates/strange-loop/Cargo.toml:0
crates/nanosecond-scheduler/Cargo.toml:0
crates/quic-multistream/Cargo.toml:0
crates/temporal-attractor-studio/Cargo.toml:0
```

Lint coverage today is entirely provided by CI (`.github/workflows/rust-ci.yml:46`
runs `cargo clippy --all-targets --all-features -- -D warnings`),
which:

- only catches what default Clippy already flags,
- doesn't enforce additional lint groups
  (`clippy::pedantic`, `clippy::nursery`, `clippy::cargo`,
  `clippy::restriction` subset),
- doesn't enforce rustdoc lints (`missing_docs`,
  `rustdoc::broken_intra_doc_links`),
- doesn't ban dangerous patterns the project actually cares about
  (`unwrap_used`, `expect_used`, `panic` in libs — already discussed
  in [ADR-0018](0018-error-policy.md)).

Several upcoming ADRs reference lint behaviour:

- [ADR-0018](0018-error-policy.md) wants
  `clippy::unwrap_used = "deny"` in crates and src, `warn` in tests,
  examples, benches.
- [ADR-0011](0011-quic-tls-verification.md) wants
  `clippy::missing_safety_doc` and similar.
- [ADR-0014](0014-supply-chain-pinning.md) wants
  `clippy::multiple_crate_versions` discoverable.

This ADR collects the lint policy in one place and turns it on via
the `[workspace.lints]` table that Cargo introduced in 1.74 — and
which the workspace's MSRV (per [ADR-0023](0023-msrv-policy.md):
1.81) easily clears.

## Decision Drivers

- **Single source of lint truth.** Bumping a lint should be a
  workspace edit, not per-crate `lazy_static! / #![deny]` headers.
- **PR-time enforcement.** Lints belong in the manifest so
  `cargo check` fails locally before CI ever runs.
- **Discoverable.** A new contributor reading `Cargo.toml` sees the
  policy.
- **Granular allow / warn / deny.** Some lints (`clippy::pedantic`)
  are too noisy to deny outright; warn-level is the right pressure.

## Considered Options

1. **Keep CI as the only lint surface.** Status quo.
2. **Add `[workspace.lints]` with a curated rule set inherited by
   every crate.** Single source of truth; cargo 1.74+ feature.
3. **Per-crate `#![deny(...)]` headers.** Cumbersome; drifts; misses
   `Cargo.lock`-related lints.
4. **External tool (`rust-clippy --config clippy.toml`).** Provides
   only some knobs; doesn't replace `[lints]`.

## Decision Outcome

**Chosen option: Option 2 — `[workspace.lints]` is the canonical
policy table; per-crate `[lints] workspace = true` inherits.**

Initial policy (root `Cargo.toml`):

```toml
[workspace.lints.rust]
unsafe_code = "deny"                   # cf. ADR-0011: unsafe is opt-in per crate
missing_docs = "warn"                  # libs eventually deny via override
rust_2018_idioms = { level = "warn", priority = -1 }
missing_debug_implementations = "warn"
unreachable_pub = "warn"
trivial_numeric_casts = "warn"
unused_lifetimes = "warn"
unused_qualifications = "warn"
nonstandard_style = "warn"

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"
private_intra_doc_links = "warn"
invalid_codeblock_attributes = "deny"
missing_crate_level_docs = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }       # baseline
pedantic = { level = "warn", priority = -1 }  # noisy but useful
cargo = { level = "warn", priority = -1 }

# Deny on real footguns (cf. ADR-0018)
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
dbg_macro = "deny"
print_stdout = "deny"          # use tracing per ADR-0010
print_stderr = "deny"

# Off (too noisy or false-positive heavy)
module_name_repetitions = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
must_use_candidate = "allow"
needless_pass_by_value = "allow"
similar_names = "allow"
```

Per-crate inheritance (`crates/*/Cargo.toml`):

```toml
[lints]
workspace = true
```

Overrides for tests / examples / benches:

- `tests/` files: `#![allow(clippy::unwrap_used, clippy::expect_used,
  clippy::panic, clippy::print_stdout)]` at the top of the test crate
  / file.
- `examples/` files: same; examples are demonstration code.
- `benches/` files: same.
- `xtask` / build scripts: same.

Per-crate strictness ladder:

- **alpha** crates ([ADR-0024](0024-semver-and-api-stability.md)):
  `missing_docs = "warn"`.
- **beta** crates: `missing_docs = "deny"`.
- **stable** crates: `missing_docs = "deny"` + `unreachable_pub = "deny"`
  + `missing_debug_implementations = "deny"`.

`unsafe` opt-in:

- `unsafe_code` is `deny` at the workspace level.
- A crate that legitimately needs `unsafe` (none today; cf. the
  security review's "zero unsafe in workspace" finding) overrides
  with `unsafe_code = "allow"` plus a doc-comment justification at
  the top of `lib.rs`.

### Positive consequences

- One file declares the entire lint posture. Reviewers can challenge
  any new `allow` with a doc-comment requirement.
- `cargo check` fails locally on lint regressions, before CI runs.
- Per-crate inheritance prevents per-crate drift.
- The strictness ladder gives stable crates real teeth without
  blocking experimental crates.

### Negative consequences

- The first PR after this lands will surface dozens of lint hits.
  Expected; the migration PR threads the fixes per crate.
- `clippy::pedantic` at warn-level is noisy at first. Mitigated by
  the `allow` list above for the most-false-positive lints.
- `print_stdout` denial breaks ad-hoc debug prints. Right call;
  tracing per [ADR-0010](0010-allocator-observability.md) is the
  replacement.

## Implementation notes

- Land this ADR; do the migration in a follow-up PR
  `chore: adopt workspace lints per ADR-0034`.
- Add the `[workspace.lints]` blocks to root `Cargo.toml`.
- Add `[lints] workspace = true` to every member.
- Per-crate strictness override in `crates/midstreamer-temporal-compare/Cargo.toml`
  and `.../midstreamer-scheduler/Cargo.toml` (beta tier per
  [ADR-0024](0024-semver-and-api-stability.md)).
- Fix the resulting lint hits crate by crate. Add suppression
  comments only where genuinely needed; each `#[allow]` carries a
  `// reason:` comment.

## Links

- Related: [ADR-0011](0011-quic-tls-verification.md),
  [ADR-0018](0018-error-policy.md),
  [ADR-0023](0023-msrv-policy.md),
  [ADR-0024](0024-semver-and-api-stability.md).
- Cargo `[workspace.lints]`:
  https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section
