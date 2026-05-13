# 0012 — Streaming input bounds (size, rate, buffer)

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** security, streaming, resilience

## Context and Problem Statement

Untrusted data crosses two ingress boundaries with no bounds checks:

### Native ingress

`src/midstream.rs:115-166`. The only validation is
`content.is_empty()` (line 117). There is:

- No per-message size limit.
- No UTF-8 sanity check beyond what `String::from_utf8` enforces
  upstream.
- No content-type filter.
- No backpressure (see [ADR-0007](0007-bounded-backpressure.md)).
- `process_stream` (`:171-183`) pushes every message into a
  `Vec<LLMMessage>` with no cap. The metric store at `:162` is a
  `Vec<MetricRecord>` that grows forever.

A misbehaving or hostile LLM upstream can OOM the process.

### WASM ingress

`wasm-bindings/src/lib.rs:218-247`
(`StreamingHTTPClient::stream`) loops `reader.read()` with no byte
ceiling and no timeout. `WebSocketClient::new(url)` (`:52`) and the
`EventSource` constructors accept arbitrary caller-supplied URLs with
no scheme allowlist.

Combined with [ADR-0007](0007-bounded-backpressure.md) (which sets
*channel* bounds), this ADR sets *content* bounds — per-message, per-
stream, and per-second — at the trust boundary itself.

## Decision Drivers

- **Defence in depth.** Even with backpressure on channels, an
  adversary can push pathologically large messages through them; the
  bound must live *before* the channel accept.
- **Configurable, not magic.** Limits must be policy, not hard-coded
  constants. Deployers tune them.
- **Per-stream, not just per-process.** A single misbehaving stream
  must not exhaust budget shared with healthy streams.

## Considered Options

1. **No bounds (status quo).** OOM risk forever.
2. **Hard-coded constants in source.** `const MAX_CHUNK: usize =
   64 * 1024;`. Cheap; inflexible.
3. **A `StreamingLimits` struct passed through the constructor**,
   with sensible defaults (chunk ≤ 64 KiB, per-stream lifetime ≤ 16
   MiB, per-stream rate ≤ 1000 chunks/s, stream timeout 5 min). Limits
   live in `crates/midstream-config` (new crate or
   `src/config.rs`). All ingress paths consult it.
4. **`tower::limit` layers on the public `Service`.** Composable with
   the rest of the [ADR-0007](0007-bounded-backpressure.md) stack;
   harder to enforce on the WASM side which doesn't run tower.

## Decision Outcome

**Chosen option: Option 3, with tower layers from Option 4 used on the
native ingress where they fit.** A `StreamingLimits` struct holds the
policy; both the native and WASM ingress consult it.

Defaults:

| Knob                          | Default        | Rationale                                                     |
|-------------------------------|----------------|---------------------------------------------------------------|
| `max_chunk_bytes`             | 65,536 (64 KiB) | OpenAI Realtime chunks rarely exceed 8 KiB; cap above typical |
| `max_stream_bytes`            | 16,777,216 (16 MiB) | ~1h of 4 KiB chunks at 1/s; far above typical session    |
| `max_chunks_per_second`       | 1,000          | OpenAI Realtime caps around 25 chunks/s; this is 40× headroom |
| `max_stream_duration`         | 300 s          | Five minutes; tunable for batch jobs                          |
| `wasm_url_scheme_allowlist`   | `["https", "wss"]` | No HTTP/WS plaintext from WASM by default                |
| `wasm_url_host_allowlist`     | `None` (off)   | Caller-supplied via configuration; off ⇒ no allowlist        |
| `metric_store_capacity`       | 10,000         | Ring buffer; oldest evicted                                   |

### Positive consequences

- Process can no longer OOM from a single hostile stream.
- The metric store stops being an unbounded `Vec`.
- WASM consumers can't be tricked into proxying arbitrary HTTP fetches
  through Rust code by injecting an attacker URL.

### Negative consequences

- Three new public types (`StreamingLimits`, `LimitsViolation` error,
  `MetricStore` wrapper). Public API surface grows.
- Deployers must understand the knobs. Mitigation: each knob has an
  inline doc comment with the rationale and recommended-range guidance.
- Existing tests with synthetic large payloads need to either lift the
  default or assert the violation behaviour.

## Implementation notes

- New file `src/config.rs::StreamingLimits` (struct + `Default`).
  Loaded from `config::Config` keys `streaming.*` for runtime
  configurability.
- `src/midstream.rs::StreamProcessor::process_message` checks
  `content.len() > limits.max_chunk_bytes` and returns
  `Err(LimitsViolation::ChunkTooLarge)` before allocating.
- `process_stream` tracks `stream_bytes_total` and `chunks_in_window`
  per stream (window = 1s, sliding); enforces `max_stream_bytes` and
  `max_chunks_per_second`.
- Metric store: `Arc<Mutex<Vec<MetricRecord>>>` becomes
  `Arc<MetricRing>` (cap = `metric_store_capacity`, FIFO eviction).
- `wasm-bindings/src/lib.rs`: every public constructor that takes a
  URL validates `scheme ∈ limits.wasm_url_scheme_allowlist`. If a host
  allowlist is configured, also checks host. `StreamingHTTPClient::stream`
  enforces `max_chunk_bytes` and `max_stream_bytes` while reading.
- Add tests at `tests/streaming_limits_tests.rs` covering each knob
  separately (oversize chunk, oversize stream, rate burst, scheme
  reject, host reject).

## Links

- Related: [ADR-0007](0007-bounded-backpressure.md),
  [ADR-0011](0011-quic-tls-verification.md),
  [ADR-0015](0015-wasm-egress-allowlist.md).
