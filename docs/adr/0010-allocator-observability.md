# 0010 — Allocator and observability baseline (`mimalloc` + OpenTelemetry)

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** perf, observability, baseline

## Context and Problem Statement

Two related gaps:

### Allocator

The midstream binary and all benches run on the default system
allocator (glibc `malloc` on Linux). No `#[global_allocator]` is set
anywhere. For an alloc-heavy hot path (see
[ADR-0006](0006-zero-copy-bytes-streaming.md)) this leaves 10–30% on
the table on multi-threaded workloads, and contradicts the
"nanosecond-scheduler" pitch.

### Observability

`tracing = "0.1"` is in root `Cargo.toml:27`, but:

- No `tracing-subscriber` initialization in any binary.
- No spans on the hot path (`process_message`, `process_stream`,
  `compare`, `schedule`).
- No `tracing-opentelemetry` exporter to OTLP. Distributed tracing
  unbuildable.
- No `console-subscriber` / `tokio-console`. Tokio task starvation,
  blocked tasks, and runtime stalls are invisible.
- `arrow-flight = "54"` is declared in root `Cargo.toml:22` but **not
  used** by any first-party code (verified via grep for `FlightData`,
  `arrow_flight` — zero hits outside the unused vendored
  `hyprstream-main`). It drags in the entire tonic/hyper-0.x stack —
  two extra HTTP stacks, two extra rustls versions.

The result: the system claims real-time performance with no way to
observe whether it's behaving in real-time, and pays for two transport
stacks it never uses.

## Decision Drivers

- **Drop unused deps before adding new ones.** `arrow-flight` removal
  also collapses duplicate `tokio`/`tower`/`hyper`/`rustls` versions.
- **Allocator override is a 1-line change with measurable upside.**
- **Observability must be the *first* thing added**, not the last,
  because every other ADR (backpressure, cache, scheduler) wants
  spans to confirm it's working.

## Considered Options

### Allocator

1. **Default system allocator.** Status quo.
2. **`mimalloc`** as the global allocator (musl-friendly, fast on
   threaded workloads, MIT licence).
3. **`jemallocator`** (well-tested, but `jemalloc` upstream is now
   archived; long-term risk).
4. **`snmalloc-rs`** (Microsoft Research, very fast, less battle-tested
   in the Rust ecosystem).

### Observability

1. **`tracing-subscriber` JSON to stdout only.** Minimum viable.
2. **`tracing-subscriber` + `tracing-opentelemetry` + `opentelemetry-
   otlp` (gRPC)** with spans on every public boundary.
3. **Option 2 plus `console-subscriber` behind a feature flag** for
   runtime task introspection.

## Decision Outcome

**Chosen allocator: Option 2 — `mimalloc` as `#[global_allocator]`** in
the `midstream` binary and all bench harnesses. Crates that are also
published to crates.io do **not** set the global allocator (they have
no `main`).

**Chosen observability: Option 3 — `tracing-subscriber` +
`tracing-opentelemetry` + `opentelemetry-otlp`, with `console-subscriber`
behind the `tokio-console` feature flag.** Every public async function
gets a `#[instrument]` span; spans carry channel-fill-ratio and
backpressure events from [ADR-0007](0007-bounded-backpressure.md).

**Dependency drop:** remove `arrow-flight` from root `Cargo.toml` and
verify with `cargo tree --workspace --duplicates` that the duplicate
`tokio`/`tower`/`hyper`/`rustls` graphs collapse.

### Positive consequences

- mimalloc typically reduces hot-path latency by 10–30% on alloc-heavy
  Rust services. Free perf.
- One CLI flag (`--otlp-endpoint`) enables full distributed traces
  exported to any OTLP-compatible backend (Tempo, Jaeger, Honeycomb,
  Datadog).
- `tokio-console` makes task starvation and blocked tasks debuggable
  for the first time.
- Dropping `arrow-flight` shaves ~2 GB of compile time (`cargo tree`)
  and collapses dual `rustls`/`tower`/`hyper` graphs.

### Negative consequences

- mimalloc adds ~100KB to release binary size. Trivially acceptable.
- `tracing-opentelemetry` has a non-trivial dep closure of its own;
  but the trade is paying it once vs paying for `arrow-flight` (which
  we don't use) every build.
- Removing `arrow-flight` is a semver-major if anything downstream
  depends on midstream re-exporting it (audit before removal).
- `console-subscriber` requires the binary to be built with
  `RUSTFLAGS="--cfg tokio_unstable"`; we put it behind a feature flag
  so default builds are unaffected.

## Implementation notes

- Add to `src/bin/main.rs` (or a shared `src/runtime.rs`):

  ```rust
  #[global_allocator]
  static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
  ```

- Add to `[workspace.dependencies]`:

  ```toml
  mimalloc = { version = "0.1", default-features = false }
  tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
  tracing-opentelemetry = "0.27"
  opentelemetry = "0.27"
  opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic"] }
  console-subscriber = "0.4"   # behind feature flag
  ```

- In `src/main.rs`, install the subscriber stack with `OTLP_ENDPOINT`
  env var driving the exporter (no flag → stdout JSON only).
- `#[tracing::instrument(skip_all, fields(stream_id, chunk_len))]` on
  `process_message`, `process_stream`, `compare`, `schedule`.
- Drop `arrow-flight` from root `Cargo.toml:22`. Verify `cargo tree
  --workspace --duplicates` shows fewer dupes.

## Links

- Related: [ADR-0006](0006-zero-copy-bytes-streaming.md),
  [ADR-0007](0007-bounded-backpressure.md),
  [ADR-0009](0009-honest-benchmarks.md).
- `mimalloc-rs`: https://github.com/microsoft/mimalloc
- `tracing-opentelemetry`: https://docs.rs/tracing-opentelemetry/
