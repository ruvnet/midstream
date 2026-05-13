# 0033 — Real-time scheduler SLO contract

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** perf, slo, scheduler

## Context and Problem Statement

`crates/nanosecond-scheduler/src/lib.rs:3` advertises:

> //! Ultra-low-latency real-time task scheduler with nanosecond precision.

But the crate ships **no SLO** — no numeric statement of latency
percentiles, no statement of contention behaviour, no statement of
queue-depth limits beyond `SchedulerError::QueueFull`. The
implementation also visibly contradicts the marketing: `schedule()`
(`:201-227`) takes **three separate `RwLock::write()` locks** —
`task_queue.write()`, `next_task_id.write()`, `stats.write()` — in
sequence per scheduled task. Each `parking_lot::RwLock::write` costs
20–40 ns uncontended; the claimed "<100ns" floor is not consistent
with the implementation.

[ADR-0008](0008-lock-free-scheduler-cache.md) proposes rewriting the
scheduler to be lock-free (`crossbeam_queue::SegQueue` per priority,
`AtomicU64` stats). That ADR lays out *how* to make the scheduler
fast. This ADR lays out *what* fast means: the published, testable
SLO that the rewrite must meet.

Without a stated SLO:

- the lock-free rewrite has no acceptance criterion,
- consumers don't know what to expect,
- regressions are detected by feel, not by a number.

## Decision Drivers

- **Honesty.** A "nanosecond-precision" claim needs a stated, tested
  contract. Otherwise it's marketing.
- **Per-percentile clarity.** p50 is not p99 is not p999. Real-time
  systems care about the tail; a stated SLO must include the tail.
- **Contention dimension.** A claim valid at N=1 thread is
  meaningless for a multi-core deployment.
- **Workload dimension.** Different queue depths and priority
  distributions hit different bottlenecks; the SLO must specify the
  distribution.
- **Testable in CI.** The SLO is a `criterion-perf-events` bench
  (per [ADR-0009](0009-honest-benchmarks.md)) with regression alarms.

## Considered Options

1. **No SLO; rely on a single "<100ns" claim.** Status quo.
2. **Stated SLO with one number per metric (no percentiles).**
   "Average <100ns" — easy to write, useless against the tail.
3. **Stated SLO with per-percentile + per-contention numbers.**
   Like a real-time-systems datasheet. Most informative; needs the
   bench harness to back it.
4. **Adopt an external standard** (e.g. POSIX `SCHED_FIFO`-style
   guarantees). Overkill for a userspace tokio scheduler.

## Decision Outcome

**Chosen option: Option 3 — per-percentile, per-contention SLO,
backed by CI-enforced benches.**

### SLO

Workload definitions:

- **W-small:** queue depth ≤ 256, 4 priority levels, deadline ≥ 1 ms.
  Models the LLM-token streaming use case.
- **W-medium:** queue depth ≤ 16,384, 4 priority levels, deadline ≥ 1 ms.
  Models batch-style workloads.
- **W-burst:** queue depth ≤ 256, but with a 16x burst arrival followed
  by drain. Models adversarial input.

Latency (`schedule()` end-to-end, post-rewrite per
[ADR-0008](0008-lock-free-scheduler-cache.md)):

| Metric        | W-small | W-medium | W-burst |
|---------------|---------|----------|---------|
| p50           | ≤ 100 ns | ≤ 150 ns | ≤ 150 ns |
| p99           | ≤ 500 ns | ≤ 1 µs   | ≤ 2 µs   |
| p999          | ≤ 2 µs   | ≤ 5 µs   | ≤ 10 µs  |
| max           | ≤ 50 µs  | ≤ 100 µs | ≤ 500 µs |

These are stated for `cargo bench` on a stock x86_64 Linux box
running `rustc 1.81` release with `mimalloc` (per
[ADR-0010](0010-allocator-observability.md)). ARM64 numbers are
informational; not contractual until benched.

Contention (concurrent producers + 1 consumer):

| Producers | p99 schedule() |
|-----------|----------------|
| 1         | ≤ 500 ns       |
| 4         | ≤ 1 µs         |
| 16        | ≤ 5 µs         |
| 64        | ≤ 20 µs        |

Throughput floors:

- **W-small:** ≥ 5 M schedules/sec on 4 cores.
- **W-medium:** ≥ 2 M schedules/sec on 4 cores.

Missed-deadline policy:

- A task is "missed" if executed after `deadline + slack` where
  `slack = max(SLO.p999, deadline / 10)`.
- Missed-deadline rate target: ≤ 1 in 10⁶ tasks under W-small;
  ≤ 1 in 10⁴ under W-burst.
- The scheduler exposes `SchedulerStats::missed_deadlines` (already
  present at `:175`); CI assert checks against the targets.

### Acceptance / regression

- `benches/scheduler_slo_bench.rs` (new) implements W-small,
  W-medium, W-burst and emits per-percentile numbers via
  `criterion-perf-events`.
- Each run is 30s warm-up + 60s steady-state.
- CI fails if any percentile in the table is exceeded by more than
  10%; warns at 5%.
- The full SLO table is regenerated into `docs/SCHEDULER_SLO.md`
  on every push to `main` (cf. [ADR-0009](0009-honest-benchmarks.md)).

### Positive consequences

- The "nanosecond" claim is a number, not a vibe.
- The lock-free rewrite ([ADR-0008](0008-lock-free-scheduler-cache.md))
  has a binary pass/fail bar.
- Consumers can size deployments against a stated table instead of
  guessing.
- Regressions are detected automatically.

### Negative consequences

- A failed SLO blocks a release. That's the point — but it means we
  cannot ship a scheduler regression and treat it as "we'll fix it
  later."
- The SLO is platform-specific. ARM64 / Apple Silicon / Graviton
  numbers will need a separate column once those builds are tested.
- W-burst tail latencies depend on allocator behaviour; the SLO
  presumes `mimalloc`. If the allocator changes, the SLO must be
  re-derived.

## Implementation notes

- New bench `benches/scheduler_slo_bench.rs`. Uses
  `criterion-perf-events` and `hdrhistogram` for tail percentiles.
- Add `tests/scheduler_slo_test.rs` for a coarse-grained CI check
  (single-threaded W-small): if p99 exceeds 1 µs, fail.
- Add `docs/SCHEDULER_SLO.md` regenerated by the bench harness; do
  **not** hand-edit.
- The SLO table is part of `crates/nanosecond-scheduler`'s README
  (auto-included) so docs.rs shows it.

## Links

- Related: [ADR-0008](0008-lock-free-scheduler-cache.md),
  [ADR-0009](0009-honest-benchmarks.md),
  [ADR-0010](0010-allocator-observability.md).
- `hdrhistogram`: https://docs.rs/hdrhistogram/
