# 0038 — Fuzz + property tests for the parsing/streaming surface

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** testing, security, fuzz

## Context and Problem Statement

A grep across the workspace for `proptest`, `quickcheck`, `arbitrary`,
`cargo-fuzz`, `libfuzzer`, `honggfuzz` returns **zero hits**. The
project has unit tests, integration tests, doc tests, and benches —
but **no property-based tests and no fuzz targets**.

This matters because the surface that accepts untrusted input is
non-trivial:

- **LLM byte stream → DTW/LCS/edit-distance** in
  `crates/temporal-compare`. Pattern detection over an attacker-
  controlled sequence: are there inputs that explode allocation,
  trigger pathological DP-matrix traversal, or violate the cost
  function's invariants (negative costs, NaN propagation)?
- **QUIC framing → typed messages** in `crates/quic-multistream`. Any
  fuzz target on the wire decoder? No.
- **MCP JSON-RPC → tool dispatch** in `npm/src/mcp-server.ts` and
  (per [ADR-0032](0032-mcp-tool-surface.md)) the future Rust MCP
  server. The JSON parser is `serde_json`; the *interpretation* of
  shapes is hand-rolled and unfuzzed.
- **Config parsing** through `config = "0.13"` (per
  [ADR-0019](0019-config-system.md), to become `figment`). User-
  facing config files are attacker-controlled in multi-tenant
  deployments.
- **SSE / WebSocket framing** in `src/midstream.rs` (and TS twin in
  `npm/src/openai-realtime.ts`). The SSE parser sees server-
  controlled bytes.

The earlier `docs/NAN_PANIC_FIX_SUMMARY.md` exists, which implies at
least one NaN-related crash was discovered the hard way. Property
tests would have caught it before merge.

## Decision Drivers

- **Inputs come from untrusted parties.** LLM outputs, transport
  bytes, config files, MCP messages — all are adversarial surfaces in
  a real deployment.
- **Algorithmic surfaces have invariants we can state.** Pattern
  detection should be `monotone` w.r.t. prefix extension, DTW should
  satisfy triangle-inequality bounds, edit-distance is a metric,
  schedule()'s deadline ordering is total — every one of these is a
  property a test can check.
- **Compile-once, test-forever.** Fuzz corpora discovered on the
  first run get reused; the regression set grows over time.
- **Cheap when wired correctly.** Property tests run in CI in
  seconds; fuzz runs in a separate, time-bounded job.

## Considered Options

1. **Status quo.** Unit tests + benches only.
2. **Property tests only (`proptest` or `quickcheck`).** Cheap;
   excellent coverage of invariants; runs in normal CI.
3. **Fuzz tests only (`cargo-fuzz` / `libfuzzer-sys`).** Best at
   parser / state-machine surfaces; needs a separate CI job and a
   corpus directory.
4. **Both, layered.** Property tests on every algorithmic crate;
   fuzz targets on each parser surface. Industry SOTA.

## Decision Outcome

**Chosen option: Option 4 — property tests as a baseline, fuzz
targets on every parser/state-machine surface.**

### Property tests (`proptest`)

Every workspace crate gains a `[dev-dependencies] proptest = "1"`
and a `tests/proptest_*.rs` file. Per-crate baseline invariants:

| Crate                            | Invariants                                                                 |
|----------------------------------|----------------------------------------------------------------------------|
| `midstreamer-temporal-compare`   | edit-distance is a metric (non-negative, symmetric, triangle inequality); DTW(x,x)=0; LCS bounded by min(|x|,|y|); no NaN escape |
| `midstreamer-scheduler`          | tasks scheduled with earlier deadline pop first; missed-deadlines monotonic in late arrivals; `schedule` is wait-free under uncontended load |
| `midstreamer-attractor`          | Lyapunov estimator non-negative for bounded sequences; classification is total (alpha-stable, periodic, chaotic, fixed-point) — no `unknown` leak |
| `midstreamer-neural-solver`      | LTL `□p` ≡ `¬◇¬p`; safety↔liveness duality holds on the implementation     |
| `midstreamer-strange-loop`       | self-reference cycle detection terminates on bounded graphs                |
| `midstreamer-quic-multistream`   | frame encode-then-decode = id; oversize frames rejected; no stream-id collisions |

Property runs are tuned for CI: 256 cases per property under `cargo test`,
seed pinned via `PROPTEST_CASES`/`PROPTEST_RNG_ALGORITHM=chacha`.
Failure shrinks committed to `proptest-regressions/*.txt` per crate.

### Fuzz tests (`cargo-fuzz` + `libfuzzer-sys`)

A `fuzz/` directory at the workspace root (treated like `xtask` — a
non-published auxiliary member). Targets:

```
fuzz/
├── Cargo.toml
└── fuzz_targets/
    ├── sse_parser.rs                  # SSE event stream → frame
    ├── quic_frame_decode.rs           # bytes → QuicFrame
    ├── mcp_jsonrpc_dispatch.rs        # JSON-RPC request → tool dispatch
    ├── temporal_compare_compare.rs    # (Bytes,Bytes) → similarity
    ├── scheduler_event_loop.rs        # randomized schedule()/run() event sequence
    └── figment_config_load.rs         # TOML/JSON bytes → MidstreamConfig
```

Each target asserts only "did not panic / did not OOM". Crash
artefacts are committed under `fuzz/artifacts/<target>/`; the seed
corpus under `fuzz/corpus/<target>/`.

CI:

- `cargo xtask ci` runs property tests (fast; required).
- A separate `fuzz.yml` workflow on a schedule (nightly) and on
  PR-label `fuzz` runs each target for **5 minutes**. Crashes fail
  the workflow; the artefact is uploaded as a CI artefact for
  reproducer review.
- Optionally, a long-running fuzz pool via [oss-fuzz](https://github.com/google/oss-fuzz)
  integration. Documented as a stretch goal in
  `docs/adr/0040-future-work.md` — not committed here.

### Positive consequences

- Algorithmic invariants verified, not asserted by hand.
- Parser surfaces continuously fuzzed; new crash inputs accumulate
  into a regression corpus.
- The NaN class of bugs (cf. `docs/NAN_PANIC_FIX_SUMMARY.md`) is
  caught by the metric-invariant property on every PR.

### Negative consequences

- Property tests catch invariant violations; they don't catch
  spec/implementation drift if both move together. Mitigated by
  *also* having unit tests that pin specific inputs.
- Fuzz CI requires a non-default toolchain (`cargo +nightly install
  cargo-fuzz` works only on nightly historically; modern stable
  `cargo-fuzz` should be checked at the time the work is done).
  Mitigated by pinning a specific cargo-fuzz version in `xtask`.
- Fuzz corpora grow indefinitely; we cap committed corpus per target
  at ~100 inputs and rotate older ones into `corpus/archive/`.

## Implementation notes

- Add `[dev-dependencies] proptest = "1"` per crate.
- Add a `tests/proptest_<concept>.rs` template per crate.
- Add `fuzz/` at the workspace root with `cargo fuzz init`.
- The first 6 fuzz targets in the table above are the priority
  list; ship them in a single PR.
- Add `cargo xtask fuzz <target> --time=300s` so contributors can
  run a target locally.
- Add `.github/workflows/fuzz.yml` running on schedule + label.

## Links

- Related: [ADR-0009](0009-honest-benchmarks.md),
  [ADR-0033](0033-scheduler-slo-contract.md),
  [ADR-0034](0034-workspace-lints.md),
  [ADR-0037](0037-xtask-build-automation.md).
- `proptest`: https://docs.rs/proptest/
- `cargo-fuzz`: https://rust-fuzz.github.io/book/cargo-fuzz.html
- OSS-Fuzz: https://github.com/google/oss-fuzz
