# Changelog

All notable changes to the AIMDS workspace crates are documented here.

This file follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The four crates (`aimds-core`, `aimds-detection`, `aimds-analysis`,
`aimds-response`) are released in lockstep at the same workspace
version — see `Cargo.toml`'s `[workspace.package]`.

## npm `aidefence` [3.2.0] - 2026-11-09

### Added
- `AgentDBClient.flush()` and `StoreBackend.flush()`: resolves once every write made before the call is durable.
- `StoreBackend.insertMany()`: several documents in one call, and in one commit on rvf.
- `AgentDBConfig.writeBehind` (`maxBatch`, default 256; `flushIntervalMs`, default 50; 0 disables batching).

### Changed
- rvf writes are batched through a bounded write-behind queue instead of one durable commit per document. Measured with `scripts/bench-store.mjs`, timed to `flush()`: 4,308 → 76 µs per document at 1,000 patterns, 15,297 → 94 µs at 10,000. Lookups are unchanged. See "Write batching and durability" in the README for the loss window when you don't flush.
- `storeIncident` writes the incident, threat pattern, reflexion entry and causal links with one `insertMany` call, so they are committed together (one rvf ingest instead of three or more).
- Queued documents are visible to search and count before they are committed.
- `scripts/bench-store.mjs` times writes until `flush()` returns and reports `insert_us_per_doc`.

### Fixed
- An upsert committed together with other writes deletes the replaced id before the ingest, so rvf never re-ingests a live id (which drops its whole HNSW index and rebuilds it on the next query).
- Embeddings with non-finite values are stored without a vector, instead of being rejected by rvf.

## npm `aidefence` [3.1.0] - 2026-11-08

### Added
- Optional durable storage on [`@ruvector/rvf`](https://www.npmjs.com/package/@ruvector/rvf) (an `optionalDependency`, loaded lazily). Set `AgentDBConfig.path` (or `AGENTDB_PATH` for the gateway) to a file, and threat patterns, incidents and reflexion entries survive a restart. Search then runs on rvf's HNSW index, scoped to a collection by a metadata filter. See "Persistence (rvf)" in the README.
- If `path` is set but `@ruvector/rvf` is not installed, the client logs a warning and keeps the in-memory store.
- `AgentDBClient`, `InMemoryStore`, `RvfStore`, `createStore` and `Logger` are exported from the library entry. `AgentDBClient.storageKind` reports `'memory'` or `'rvf'`.
- `scripts/bench-store.mjs` measures lookup latency for both backends.

### Changed
- The default is now in memory with nothing written to disk. `AGENTDB_PATH` used to default to `./data/agentdb`, a path that was never written to. It now defaults to empty.

### Fixed
- Comments and the README no longer claim "HNSW … 150x faster" for the in-memory store, which is a linear scan.
- `InMemoryStore.upsert` keyed by `patternId` but inserted under an auto id, so upserted threat patterns accumulated duplicates. Both now use the same key.

### Rust workspace
- `aimds-detection` drops its unused `lru 0.12` dependency (RUSTSEC-2026-0002, RUSTSEC-2026-0253).
- CI runs `cargo deny` on the AIMDS workspace against a freshly resolved lockfile.

## npm `aidefence` [3.0.2] - 2026-11-06

### Security
- `zod` moves from `^3.22.4` to `^4.6.7`. Zod below 4.6.6 can exhaust memory while validating oversized constrained collections (high). There is no 3.x fix. The request schemas now pass an explicit key type to `z.record`, which zod 4 requires.
- The lockfile is refreshed to fixed versions of `proxy-addr` (critical), `body-parser`, `compression`, `form-data`, `nanoid`, `postcss`, `vite` and `brace-expansion` (high). `npm audit` reports no critical or high findings.

### Fixed
- `package.json` now has a `repository` field.

## npm `aidefence` [3.0.1] - 2026-10-31

### Security
- The leet-speak normaliser (`undoLeet`) was quadratic on letter-free runs: 100 KB of hex or digits took about 2 s per `detect()`, on input an attacker controls. It now matches maximal runs and checks for a letter in the callback, which produces the same output in linear time (0.3 ms). Found while vendoring the packs into ruflo's @claude-flow/aidefence 3.1.0.
- Each encoded blob is capped before decoding (about 12 KB), so one huge base64 run is no longer decoded in full only to be trimmed.

## npm `aidefence` [3.0.0] - 2026-10-31

### Breaking
- Importing the package no longer starts a server. `main` (`dist/index.js`) is a side-effect-free library entry. The gateway runs via the `aidefence` bin or `npm start` (`dist/server.js`).
- The gateway binds `127.0.0.1` by default; set `HOST=0.0.0.0` to listen on all interfaces.

### Added
- Exports the pack-driven detector: `createInjectionDetector`, `InjectionDetector`, `loadPacks`, `normalizeText`, decoders. Ships the 6 pattern packs (53 patterns) in `patterns/`.

### Fixed
- npm now matches main: merged `ops/aimds-deep-review`, which carried 2.3.0's RUSTSEC-2024-0421 and npm advisory fixes but had never reached main. See docs/adr/0042.

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
