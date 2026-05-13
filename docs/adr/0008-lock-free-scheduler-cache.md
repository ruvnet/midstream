# 0008 — Lock-free scheduler and cache rewrite

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** perf, concurrency

## Context and Problem Statement

Two hot data structures are textbook lock-contention hotspots:

### nanosecond-scheduler

`crates/nanosecond-scheduler/src/lib.rs:175-180`:

```rust
queue:  Arc<RwLock<BinaryHeap<ScheduledTask<T>>>>,
stats:  Arc<RwLock<SchedulerStats>>,
counter: Arc<RwLock<u64>>,
running: Arc<RwLock<bool>>,
```

`execute_task` (`:236, :252, :263`) takes `stats.write()` **three
separate times** per task. `schedule()` (`:210`) takes `queue.write()`
plus `counter.write()`. The crate's headline claim is "sub-100ns
scheduling overhead"; with `parking_lot::RwLock::write` already costing
20–40ns uncontended, the claim is mathematically incompatible with the
implementation.

### temporal-compare cache

`crates/temporal-compare/src/lib.rs:175-179`:

```rust
dtw_cache:     Arc<Mutex<LruCache<String, f64>>>,
lcs_cache:     Arc<Mutex<LruCache<String, f64>>>,
edit_cache:    Arc<Mutex<LruCache<String, f64>>>,
cache_hits:    Arc<DashMap<String, u64>>,
cache_misses:  Arc<DashMap<String, u64>>,
```

`compare()` (`:223`) does:
1. `cache.lock()` — read attempt
2. on miss: drop the lock, compute, `cache.lock()` again to insert
3. update `cache_hits`/`cache_misses` (DashMap write)
4. cache key is built with `format!("patterns:{:?}:{}:{}", ...)` —
   the `String` is hashed twice (once for LRU, once for the DashMap
   counter)

A genuinely SOTA Rust cache library (`moka`, `quick_cache`) does all of
this lock-free and key-precomputed.

## Decision Drivers

- **Honest perf claims.** The "nanosecond" naming is a contract with
  users; the implementation must live up to it on the uncontended path
  *and* degrade gracefully under contention.
- **No new dependencies without a reason.** `crossbeam` is already in
  the workspace dep set but unused by the scheduler; `moka` is widely
  audited and async-aware.
- **Match the access pattern.** The scheduler queue is a min-heap with
  rare evictions; the temporal cache is read-heavy with occasional
  writes — both are perfect fits for lock-free or fine-grained-lock
  structures.

## Considered Options

1. **Status quo.** Keep `parking_lot::RwLock<BinaryHeap>` and
   `Arc<Mutex<LruCache<String>>>`. Live with the contention; relabel
   the perf claims.
2. **Migrate scheduler to `crossbeam_skiplist::SkipMap<deadline,
   Task>`** (sorted, lock-free, O(log n) insert/pop_first). Stats
   become `AtomicU64`. Counter becomes `AtomicU64`. Running flag
   becomes `AtomicBool`.
3. **Migrate scheduler to a fixed-priority array of
   `crossbeam_queue::SegQueue`** (one queue per priority level,
   lock-free FIFO). Insert/pop is wait-free. Only works if priorities
   are quantized — they already are (`SchedulingPolicy` enum has 4
   levels).
4. **Migrate caches to `moka::sync::Cache<u64, f64>`** keyed by
   pre-computed `xxhash3_64` of the slice. Single cache call (get-or-
   insert) instead of lock + miss + lock-again. Built-in stats — drop
   the DashMap counters.
5. **Migrate caches to `quick_cache::sync::Cache<u64, f64>`.** Lower
   ceremony than moka, slightly fewer features, also keyed by `u64`.

## Decision Outcome

**Chosen option: scheduler = Option 3 (`SegQueue` per priority,
`AtomicU64` stats), caches = Option 4 (`moka::sync::Cache<u64, _>`).**

`SegQueue` per priority maps exactly onto the existing
`SchedulingPolicy` enum and gives wait-free push/pop. `moka` is async-
aware (we already run on tokio) and has the eviction policy knobs we
need.

### Positive consequences

- Scheduler hot path becomes: load priority → SegQueue push (wait-free)
  → fetch_add on `AtomicU64` stats. No locks taken on the happy path.
- Cache hot path becomes: hash slice → moka `get_or_insert_with`. One
  hash, one lookup, no double-lock dance, no `String` allocation.
- The "nanosecond" name stops being a marketing lie. Realistic post-
  rewrite ballpark: 50–200ns per schedule on uncontended cores.

### Negative consequences

- Lose ordering guarantees that a `BinaryHeap` provided (deadlines
  within a priority level are FIFO, not strict-EDF). For systems that
  truly need EDF, expose a feature flag that switches that priority
  level to a `SkipMap` (Option 2 hybrid).
- One new dep (`moka`). Built-in audit overhead.

## Implementation notes

- Pin `crossbeam-queue = "0.3"`, `moka = { version = "0.12", features =
  ["sync"] }`, `xxhash-rust = { version = "0.8", features = ["xxh3"] }`
  in root `[workspace.dependencies]`.
- Rewrite `RealtimeScheduler` internals; preserve public API.
- Migrate `TemporalComparator` cache fields; remove `cache_hits`/
  `cache_misses` DashMaps in favour of `moka`'s `EntryStats`.
- Add `criterion-perf-events` regression benches before and after; gate
  the PR on no regression at any contention level (1, 4, 16, 64 cores).

## Links

- Related: [ADR-0006](0006-zero-copy-bytes-streaming.md),
  [ADR-0009](0009-honest-benchmarks.md).
- `moka` docs: https://docs.rs/moka/
- `crossbeam-queue`: https://docs.rs/crossbeam-queue/
