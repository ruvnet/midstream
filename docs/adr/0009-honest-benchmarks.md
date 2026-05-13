# 0009 — Benchmarks against real workloads, not mocks

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** perf, benchmarks, ci

## Context and Problem Statement

The numbers in `docs/BENCHMARK_RESULTS.md`,
`docs/PERFORMANCE_VALIDATION.md`, and `BENCHMARKS_SUMMARY.md` are a mix
of credible micro-benches and outright fabrications. A walk through the
22 advertised targets in `BENCHMARKS_SUMMARY.md`:

- **"QUIC throughput >1 GB/s", "stream <1ms", "multiplexing <100µs"** —
  measured against `benches/quic_bench.rs:71-85`, which uses an in-
  memory `mock::MockConnection` with a `tokio::time::sleep`. There is
  no quinn, no UDP socket, no TLS handshake, no congestion control. The
  numbers measure tokio's timer + atomic increments, not QUIC.
- **"Schedule overhead <100ns"** (`docs/BENCHMARK_RESULTS.md:65`) — the
  bench builds a fresh `RealtimeScheduler` + fresh `Action` +
  `HashMap::new()` *inside* each `b.iter` (see
  `benches/lean_agentic_bench.rs:394-417`). Construction overhead
  dominates; the real schedule cost is hidden.
- **"DTW <10ms for n=1000"** — defensible as an upper bound (DP-matrix
  alloc + populate), but a banded or Hirschberg DTW would beat it 5–10x.
- **End-to-end / concurrent_sessions** benches construct a full
  `LeanAgenticSystem` (4 `Arc<RwLock<_>>`, plus 4 sub-systems) and a
  fresh `AgentContext` inside `b.iter` — easily 90% construction
  overhead. Useless for regression tracking.
- No CI gate runs any of these benches. Numbers are captured by hand,
  past-tense, into markdown.

The credible ones (DTW/LCS/Edit small-n) sit next to the fabricated
ones (QUIC throughput) under the same heading. Anyone reading the docs
cannot tell which is which.

## Decision Drivers

- **Truth in advertising.** Public benchmark numbers must be
  reproducible against the published crates by anyone with `cargo`.
- **Regression detection.** Perf regressions should fail CI, not appear
  in a hand-written report three months later.
- **Workload realism.** Network benches must run against a real
  network stack; cache benches must run against representative key
  distributions; pipeline benches must measure the pipeline, not its
  constructors.

## Considered Options

1. **Status quo.** Leave the marketing numbers in place. Reputation
   risk grows with adoption.
2. **Tag every bench with a `credibility` level** (`micro`,
   `realistic`, `mock-only`) in the markdown — still hand-written, just
   honest. Cheap; doesn't fix the underlying benches.
3. **Rewrite the offending benches** so that:
   - QUIC benches run against a real loopback `quinn` endpoint with
     real TLS;
   - construction happens *outside* `b.iter` (use `b.iter_batched` with
     `BatchSize::SmallInput`);
   - cache benches replay a recorded key distribution from a real
     workload, not a synthetic uniform random one;
   - end-to-end benches drive the public `Service`/`Stream` API, not
     internal struct constructors.
   Add `criterion-perf-events` for hardware counters (IPC, cache
   misses) and `dhat`/`stats_alloc` for allocation regression detection.
4. **Replace `criterion` with `divan`** (newer, lower overhead). Better
   raw harness; requires rewriting all benches.

## Decision Outcome

**Chosen option: Option 3.** Rewrite benches in place; add real-network
QUIC benches, `criterion-perf-events`, and `dhat` regression
detection; gate PRs in CI on no regression beyond a configurable
threshold (default 5% per metric).

The hand-written `BENCHMARKS_SUMMARY.md` is replaced by a
machine-generated `bench/REPORT.md` produced by `cargo criterion
--message-format=json` + a small post-processor, with 95% confidence
intervals on every number.

### Positive consequences

- Numbers in docs become reproducible by anyone running the bench
  suite.
- Regressions fail PRs at review time, not at release time.
- The benches start measuring *the system* instead of *the
  constructors of the system*.

### Negative consequences

- One-time effort to rewrite the seven benches in `benches/*.rs`. The
  QUIC bench specifically requires a fresh loopback `quinn` setup
  (server + client + cert).
- CI bench job is slower (minutes per push). Mitigate by running the
  full set only on `main` and on PR labels (`bench`), with a fast
  subset on every PR.

## Implementation notes

- Replace `benches/quic_bench.rs` mock-mode harness with a real
  `quinn::Endpoint` server/client pair on loopback; assert latency and
  throughput against thresholds checked into source.
- Rewrite every `b.iter(|| { let s = Struct::new(); s.method(...) })`
  to `b.iter_batched(|| Struct::new(), |s| s.method(...),
  BatchSize::SmallInput)`.
- Add `[dev-dependencies] dhat = "0.3"` and a `bench_alloc_regressions.rs`
  bench that tracks bytes-allocated per chunk.
- Add `criterion-perf-events = "0.4"` to expose IPC and L1/L2/LLC miss
  counts on Linux.
- Add a GitHub Actions matrix job that runs `cargo criterion --workspace`
  on push to `main`, posts a comment to associated PRs with the diff.

## Links

- Related: [ADR-0006](0006-zero-copy-bytes-streaming.md),
  [ADR-0008](0008-lock-free-scheduler-cache.md),
  [ADR-0010](0010-allocator-observability.md).
- `criterion-perf-events`:
  https://github.com/jbreitbart/criterion-perf-events
- `dhat`: https://docs.rs/dhat/
