# 0007 — Bounded backpressure: `mpsc::channel(N)` + `Semaphore`, never unbounded

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** perf, streaming, reliability

## Context and Problem Statement

There is **no explicit backpressure anywhere in the pipeline**. A grep
for `unbounded_channel`, `mpsc::unbounded`, `bounded_channel`,
`Semaphore`, and `buffer_unordered` across `src/`, `crates/*/src/`, and
`hyprstream-main/src/` returns zero hits.

Concretely:

- `src/midstream.rs:171-183` consumes `BoxStream<'static, String>` via
  `while let Some(content) = stream.next().await { ... messages.push(...) }`,
  with no per-stream cap, no rate limit, and no abort path. A
  misbehaving (or hostile) LLM upstream can OOM the process.
- The metric store at `src/midstream.rs:162` is an
  `Arc<tokio::sync::Mutex<Vec<MetricRecord>>>` with no rotation or
  ceiling. It grows forever.
- `crates/nanosecond-scheduler/src/lib.rs:210` exposes a
  `max_queue_size`/`QueueFull` return — the only real backpressure
  surface in the codebase — but no caller respects it.

This is a correctness problem, not just a perf problem: under
adversarial input or upstream burst the entire pipeline can DoS itself.

## Decision Drivers

- **Liveness under load.** Producers slower than consumers must apply
  back-pressure; producers faster than consumers must block, drop with
  policy, or load-shed — never silently buffer to OOM.
- **Composability.** Every channel boundary in the pipeline must use the
  same primitive so that backpressure propagates transitively from the
  network ingress to the sinks.
- **Observability.** Each channel needs a `current_len()` / `capacity()`
  surface for `tracing` spans (see [ADR-0010](0010-allocator-observability.md)).

## Considered Options

1. **Keep status quo, document the risk.** Cheapest. The OOM bug stays.
2. **Bounded `tokio::sync::mpsc::channel(N)` at every stage boundary,
   plus `Semaphore::acquire` for cross-stream concurrency.** Industry
   standard. Producers `await` on full channels.
3. **`async-channel` (closeable, MPMC) everywhere.** Slightly more
   flexible than tokio's mpsc; loses tokio's runtime integration.
4. **`tokio-util::sync::PollSender` + `tower::Service` rate
   limiter.** Highest ceremony; gives proper service-style timeouts and
   load-shedding via `tower::limit::RateLimit` and `tower::load`.

## Decision Outcome

**Chosen option: Option 2 with a sprinkle of Option 4 at the public
boundaries.** Every internal stage uses `tokio::sync::mpsc::channel(N)`;
the *public* ingress (where untrusted LLM bytes enter the system) is
wrapped in a `tower::Service` chain with a `tower::limit::ConcurrencyLimit`
+ `tower::limit::RateLimit` + `tower::timeout::Timeout` stack.

`N` is sized at deployment time from a `StreamingConfig` (default: 64
chunks per stage, derived from expected token-rate × max-tolerable-tail
latency).

### Positive consequences

- Producer-consumer mismatch surfaces as a measurable, observable
  channel-full event rather than as a memory exhaustion crash.
- The pipeline composes: when the metric sink slows, the channel fills
  back to ingress and slows the LLM read.
- The metric store stops being an unbounded `Vec`. It either rotates
  (ring buffer) or load-sheds (oldest drops) — both policies are
  expressible at channel construction time.

### Negative consequences

- Public API of every streaming function changes (sync `Vec` return
  becomes async `impl Stream` or a channel pair). Semver-major.
- Operators must tune `N` per-deployment. Wrong values cause either
  latency spikes (too small) or memory pressure (too large). Mitigated
  by sane defaults + `tracing` spans that show channel fill ratio.

## Implementation notes

- Replace `Vec<LLMMessage>` accumulator in
  `src/midstream.rs:174` with `tokio::sync::mpsc::channel::<LLMMessage>(N)`
  where `N` defaults to 64.
- Replace `Arc<Mutex<Vec<MetricRecord>>>` in `src/midstream.rs:71` with
  a `MetricStore` enum: `Channel(mpsc::Sender)` for live sinks,
  `Ring(SkipList…)` for the in-memory rolling window.
- Add `StreamingConfig::{chunk_buffer, concurrency_limit, rate_limit,
  timeout}`.
- Wire the public LLM ingress through `ServiceBuilder::new()
  .concurrency_limit(M).rate_limit(rps, Duration::SECOND)
  .timeout(d).service(MidstreamService::new(...))`.
- Backpressure events emit `tracing::warn!` spans with channel name +
  fill ratio so the next [ADR-0010](0010-allocator-observability.md)
  observability layer can surface them.

## Links

- Related: [ADR-0006](0006-zero-copy-bytes-streaming.md),
  [ADR-0010](0010-allocator-observability.md).
- Reference: tower-rs `Service` trait,
  https://docs.rs/tower/latest/tower/trait.Service.html
