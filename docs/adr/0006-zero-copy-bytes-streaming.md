# 0006 — Zero-copy `bytes::Bytes` end-to-end streaming pipeline

- **Status:** Accepted (implemented in #7)
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** perf, streaming, alloc

## Context and Problem Statement

The streaming hot path moves every chunk as an owned `String`. A short walk
of `src/midstream.rs:115-165`:

- `process_message` takes `content: &str` and immediately allocates a
  `LLMMessage { content: content.to_string(), ... }` (line 130-ish).
- For each chunk it constructs a `MetricRecord` whose `name` and
  `tags: Vec<(String, String)>` are 4–5 fresh `String`s (`:144-154`),
  plus a `format!("{:?}", message.intent)` and a `to_string()` of
  `content.len()`.
- The error path uses `format!("Failed to ingest metric: {}", e)` (`:158`).
- `process_stream` (`:171-183`) accumulates into a `Vec<LLMMessage>` with
  no `with_capacity` hint.

`crates/temporal-compare/src/lib.rs` is no better: cache keys are built
with `format!("patterns:{:?}:{}:{}", ...)` per `compare` call (`:388-394,
455-459, 532-537`), so cache hits *and* misses allocate. Pattern
detection at `:554` calls `sequence[start_idx..start_idx+pattern_len]
.to_vec()` per window.

`grep` for `bytes::Bytes` and `BytesMut` returns **zero hits** across the
workspace.

For a system whose pitch is "real-time, nanosecond-scheduled streaming
LLM analysis", per-token heap churn is the single biggest perf miss.

## Decision Drivers

- **Allocator-free hot path.** A token arriving over the wire should be
  passed through the pipeline by reference-counted slice, not copied.
- **Predictable tail latency.** `String::push`/`Vec::reserve`/`format!`
  insert unbounded latency spikes at allocation boundaries.
- **Compatibility with existing transports.** `reqwest` returns
  `bytes::Bytes` natively; `tokio_util::codec` produces `BytesMut`;
  `arrow-flight` already passes `Bytes` around. The right primitive is
  *already* available in our dependency closure.

## Considered Options

1. **Status quo: `String` everywhere.** No code change. Ongoing per-token
   allocation, format! cache keys, no zero-copy path.
2. **`Arc<str>` for content.** Halfway: avoids `String` clones but still
   requires UTF-8 validation and doesn't compose with `BytesMut` codecs.
3. **`bytes::Bytes` end-to-end, with a thin `BytesMut` writer
   front-end.** Tokens land in a single `BytesMut` per stream, get sliced
   into `Bytes` (zero-copy `Arc`-backed) at chunk boundaries, and flow
   through `LLMMessage`, `MetricRecord`, and the comparator as
   `Bytes`. Cache keys precomputed as `u64` (xxhash3 of the slice), never
   formatted to `String`.
4. **`bytes::Bytes` + a custom interned-string pool for tags.** Treats
   metric *tag keys* (which are small and repeat) differently from
   *content* (which is large and unique). Most complete; biggest API
   churn.

## Decision Outcome

**Chosen option: Option 3 — `bytes::Bytes` end-to-end with `BytesMut`
write-side.** Tags stay as `&'static str` keys plus `Bytes` values for
v1; the intern-pool variant (Option 4) is deferred until we see real tag
cardinality data.

### Positive consequences

- One allocation per *stream*, not one per *token*. Each chunk is an
  `Arc::clone` of an existing buffer, plus a slice index.
- Cache keys become `u64` (xxhash3 over the slice) instead of `format!`,
  eliminating two `String` allocations per `compare` call.
- The pipeline can finally accept transport-layer buffers
  (reqwest's `Bytes`, quinn's `Bytes`, tokio_util codec `BytesMut`)
  without re-allocating.

### Negative consequences

- Public types in `src/midstream.rs` (`LLMMessage::content`,
  `MetricRecord::*`) change shape. Semver-major bump for any crate that
  exports them.
- Code that uses `&str`/`String` ergonomics (e.g. log lines, JSON
  serialization of metrics) needs explicit `std::str::from_utf8` or
  `String::from_utf8_lossy` shims. We must accept a few ergonomics
  losses on the cold path to preserve the hot path.

## Implementation notes

- Add `bytes = "1"` (already transitively present) to root
  `[workspace.dependencies]`.
- Convert `LLMMessage.content: String` → `LLMMessage.content: Bytes`.
- Replace `MetricRecord.name: String` → `&'static str` (small fixed
  set of metric names); replace tag values with `Bytes`.
- In `crates/temporal-compare`, replace
  `Arc<Mutex<LruCache<String, _>>>` with `quick_cache` or
  `moka::sync::Cache` keyed by `u64` (`xxhash_rust::xxh3::xxh3_64`
  over the slice). Coupled to ADR-0008.
- Bench reference: add `bench_hotpath_alloc_per_chunk` measuring
  allocations per chunk via `dhat`/`stats_alloc`.

## Links

- Related: [ADR-0007](0007-bounded-backpressure.md),
  [ADR-0008](0008-lock-free-scheduler-cache.md).
- `bytes` crate: https://docs.rs/bytes/
