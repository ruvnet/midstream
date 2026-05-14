# Changelog

All notable changes to the AIMDS workspace crates are documented here.

This file follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The four crates (`aimds-core`, `aimds-detection`, `aimds-analysis`,
`aimds-response`) are released in lockstep at the same workspace
version — see `Cargo.toml`'s `[workspace.package]`.

## [0.1.1] - 2026-05-14

First post-launch correctness pass. No public-API breaking changes;
every change is either a bug fix, a security fix, or workspace
hygiene. Existing 0.1.0 callers compile without modification.

### Security
- **Cleared RUSTSEC-2024-0421.** Bumped `validator` 0.18 → 0.20 in
  `aimds-core`, which transitively retires the vulnerable
  `idna 0.5.0` (Punycode-masking host-name attack) and the
  unmaintained `proc-macro-error 1.0.4`.
  `cargo deny check {advisories,bans,licenses,sources}` is now clean.
- **`unsafe_code = "deny"` workspace lint** verified across all four
  crates.

### Fixed
- **`aimds-detection::sanitizer`**: prompt-injection neutralization
  regex previously required *exactly one* adjective between the verb
  and the noun (`ignore + ONE word + instructions`), missing the
  realistic shape `ignore all previous instructions`. Replaced with
  a 0..4-modifier-word window between the verb set
  (`ignore | disregard | forget | override`) and the noun set
  (`instruction[s] | prompt[s] | rule[s] | context | system-prompt`).
  Added role-hijack patterns (`you are now …` / `act as …` /
  `pretend to be …`) and jailbreak markers (`DAN mode`,
  `developer mode`, `god mode`, `root mode`).
- **`aimds-response::audit::AuditLogger`**: `total_mitigations()`
  and `successful_mitigations()` returned hardcoded `0` (a
  `// In production we'd use an atomic` TODO that never closed).
  Added `AtomicU64` hot-path counters bumped before the
  lock-protected stats so observers never see a lower count than
  the source of truth. `ResponseSystem::metrics()` now returns
  accurate numbers.
- **`aimds-response::meta_learning`**:
  - `calculate_optimization_effectiveness()` only read the
    `pattern_effectiveness` HashMap and ignored the per-rule
    success/failure counts in `learned_patterns`. Optimization
    level could never advance from feedback that drove the Vec.
    Now blends both signals.
  - `optimize_strategy()` used `get_mut`, silently dropping every
    feedback signal for a strategy_id that hadn't been seen.
    Switched to `entry().or_insert_with()` so the first feedback
    seeds the metrics row.
- **`aimds-response` doctest**: previously imported nonexistent
  `aimds_core::ThreatIncident` and used an undefined binding;
  `cargo test --doc` failed. Rewrote as a minimal example.
- **Stale benches**: `aimds-detection/benches/detection_bench.rs`
  and `aimds-analysis/benches/analysis_bench.rs` referenced types
  removed during the layer rewrite (`DetectionEngine`,
  `DetectionConfig`, `ThreatLevel`, `ThreatPattern`,
  `ThreatScheduler`, `aimds_core::{Action, State}`). Replaced with
  benches that exercise the current public API (Sanitizer,
  PatternMatcher, BehavioralAnalyzer, PolicyVerifier, LTLChecker).

### Changed
- **Workspace package metadata** updated to publish-quality:
  `authors`, `repository`, `homepage`, `documentation`, `keywords`,
  `categories` set to real values (previously
  `https://github.com/your-org/aimds` placeholder).
- **`aimds-response`** normalized to inherit from
  `[workspace.package]` like the other three crates (previously
  carried its own hardcoded `version`, `authors`, `license`).
- **Dependency bumps**:
  - `metrics` 0.21 → 0.24 (the `counter!(name).increment(n)`
    chain syntax used in `mitigations.rs` needs 0.24+).
  - `metrics-exporter-prometheus` 0.12 → 0.16 (matches).
  - `criterion`: `async_tokio` feature added at the workspace
    level (response benches drive an async mitigator).

### Quality gates (CI baseline as of 0.1.1)
- `cargo test --workspace` — 79 pass, 0 fail.
- `cargo clippy --workspace --all-targets -- -D warnings` —
  0 warnings.
- `cargo deny check {advisories,bans,licenses,sources}` —
  all four pass.

## [0.1.0] - 2026-05-13

Initial release.

- Four-tier latency design:
  - `aimds-core` — types, traits, audit-log shape.
  - `aimds-detection` — fast path (<10 ms): pattern matching,
    prompt-injection detection, PII sanitization.
  - `aimds-analysis` — deep path (<100 ms): temporal pattern
    analysis, anomaly detection, baseline learning, LTL policy
    checking, dependent-type verification.
  - `aimds-response` — mitigation (<50 ms): meta-learning,
    strategy optimization, rollback.
- Production observability: structured logging, Prometheus metrics,
  audit trails.
- Workspace lint: `unsafe_code = "deny"`.

[0.1.1]: https://github.com/ruvnet/midstream/releases/tag/aimds-v0.1.1
[0.1.0]: https://github.com/ruvnet/midstream/releases/tag/aimds-v0.1.0
