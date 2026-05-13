# 0013 — AIMDS integration contract: where it sits on the hot path

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** security, integration, aimds

## Context and Problem Statement

AIMDS (AI Manipulation Defense System) was merged into the repo via
PR #2. It currently:

- Lives at `AIMDS/` as its own workspace ([ADR-0004](0004-aimds-workspace-member.md)
  proposes folding it into the root workspace).
- Path-depends on `midstreamer-temporal-compare` and
  `midstreamer-scheduler` (one-way, `../crates/*`).
- Is **never invoked** by any midstream code. `grep` for `aimds::`,
  `Sanitizer`, `detect_pii` across `src/`, `crates/`, `wasm-bindings/`,
  and `npm-wasm/` returns zero hits.

So AIMDS is currently a *sibling project shipped under midstream*, not
a *defence layer for midstream*. The stream ingress at
`src/midstream.rs:115-166` ships every chunk straight to
`hypr_service.ingest_metric` with no AI-defence filter in between.

[ADR-0004](0004-aimds-workspace-member.md) addresses the *workspace*
question. This ADR addresses the *contract* question: **where does
AIMDS plug into the streaming pipeline, with what trait, and what
happens when it fires?**

## Decision Drivers

- **Trust boundary.** AIMDS exists to defend against untrusted LLM
  output (prompt injection, PII exfiltration, jailbreak attempts). It
  must run *before* downstream code treats the content as trusted.
- **Latency budget.** AIMDS adds work to the hot path. The overhead
  must be measurable and bounded; pipelines that don't need defence
  must be able to opt out.
- **Composable policy.** Detection (scan) and response (block, redact,
  warn) are separate concerns. Other deployments may want detect-only.

## Considered Options

1. **AIMDS as a sibling library; callers wire it manually.** Status
   quo. Reliable defence requires every caller to remember.
2. **AIMDS as a hard-coded preprocessor inside
   `StreamProcessor::process_message`.** Strong default; loses the
   ability to disable.
3. **AIMDS via a `Sanitizer` trait in `src/midstream.rs`**, with a
   default implementation that calls into `aimds_detection`. Consumers
   can swap in a no-op `Sanitizer` if they truly need to. AIMDS-aware
   detection is opt-out, not opt-in.
4. **AIMDS as a `tower::Layer` in front of the streaming `Service`**
   (from [ADR-0007](0007-bounded-backpressure.md)). Composable;
   matches the rest of the architecture.

## Decision Outcome

**Chosen option: hybrid Option 3 + Option 4.** A `Sanitizer` trait
sits between `StreamProcessor` and downstream consumers; for the
public service ingress, AIMDS is wired as a `tower::Layer`. The
defaults trip every layer; the only way to disable AIMDS is to wire a
no-op `Sanitizer` explicitly, which is a visible decision in caller
code.

Contract:

```rust
pub trait Sanitizer: Send + Sync + 'static {
    async fn scan(&self, chunk: &Bytes, ctx: &StreamContext)
        -> Result<Verdict, SanitizerError>;
}

pub enum Verdict {
    Allow,
    AllowWithWarning { reason: SmolStr },
    Redact { redacted: Bytes, reason: SmolStr },
    Block { reason: SmolStr },
}
```

Failure-mode policy:

- `SanitizerError::Timeout` → **fail closed** (block the chunk).
- `SanitizerError::Internal` → fail closed.
- The deployer can opt to `fail open` via `StreamingLimits.sanitizer_fail_open
  = true` (default `false`), which logs an `error!` span and downgrades to
  `Allow`. This knob is intentionally noisy.

AIMDS sits in `aimds_detection::default_sanitizer()` and implements
the trait. `aimds_response::default_policy()` interprets `Verdict` into
the actual chunk handling: `Block` drops the chunk and closes the
stream; `Redact` replaces it; `AllowWithWarning` emits a metric.

### Positive consequences

- AIMDS becomes an actual defence layer, not a sibling project.
- The trait surface keeps midstream usable in deployments that don't
  want AIMDS (no-op sanitizer is one line of Rust).
- Failure-mode policy is explicit: fail-closed by default, fail-open
  opt-in and loud.

### Negative consequences

- AIMDS now sits on every chunk. Per-chunk overhead becomes part of the
  perf SLO. Mitigation: bench-gate the PR that wires it (see
  [ADR-0009](0009-honest-benchmarks.md)); cap the per-chunk SLO at
  e.g. 500 µs for typical chunk sizes.
- The `Sanitizer` async trait + tower layer adds API surface that
  consumers must understand. Mitigation: provide a `MidstreamBuilder`
  fluent API so the default-secure path is the one-liner.

## Implementation notes

- New trait `Sanitizer` in `src/sanitizer.rs` (or
  `crates/midstreamer-sanitizer` if extracted later).
- `StreamProcessor::process_message` calls
  `self.sanitizer.scan(&content, &ctx).await?` *before* allocating
  `LLMMessage`. The chunk is dropped on `Verdict::Block`, replaced on
  `Verdict::Redact`.
- `aimds_detection::DefaultSanitizer` impl lives in
  `AIMDS/crates/aimds-detection`; depends on the trait crate, not
  midstream's root. Reverse dep order keeps the trait reusable.
- `MidstreamBuilder::with_default_sanitizer()` wires
  `aimds_detection::default_sanitizer()` automatically.
  `MidstreamBuilder::without_sanitizer()` wires a no-op. Both names are
  intentional — there is no implicit default-off.
- Add `benches/sanitizer_overhead_bench.rs` measuring per-chunk
  overhead. PR gate: ≤ 500 µs at p99 for 4 KiB chunks.

## Links

- Related: [ADR-0004](0004-aimds-workspace-member.md),
  [ADR-0007](0007-bounded-backpressure.md),
  [ADR-0012](0012-streaming-input-bounds.md).
- AIMDS architecture: `AIMDS/docs/` (in-tree).
