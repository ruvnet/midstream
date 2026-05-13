# 0028 — Multi-modal scope: trim the README to what code actually backs

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** scope, multimodal, hygiene

## Context and Problem Statement

`README.md:103-110` claims:

> ### 🎥 Streaming Integration
> - **QUIC/HTTP/3** - Multiplexed transport with 0-RTT and stream prioritization
> - **RTMP/RTMPS** - Real-Time Messaging Protocol support
> - **WebRTC** - Peer-to-peer audio/video streaming
> - **HLS** - HTTP Live Streaming support
> - **WebSocket/SSE** - Bidirectional and server-sent events
> - Audio transcription framework (Whisper-ready)
> - Video object detection framework (TensorFlow-ready)

What the code actually backs:

| Claim                | Rust-side dep / module | TS-side module | Status |
|----------------------|------------------------|----------------|--------|
| QUIC/HTTP/3          | `quinn 0.11` in `crates/quic-multistream` | `npm/src/quic-integration.ts` | **Partial** — basic transport, TLS-skipping ([ADR-0011](0011-quic-tls-verification.md)), no HTTP/3 layer |
| RTMP / RTMPS         | none (no `rtmp`, `rml_rtmp`, `flvparse` deps) | grepped: text references only | **Absent in Rust**; mentioned in `npm/src/restream-integration.ts` text |
| WebRTC               | none (no `webrtc`, `webrtc-rs` deps) | text references only | **Absent** |
| HLS                  | none (no `hls`, `hls-m3u8`, `m3u8-rs` deps) | text references only | **Absent** |
| WebSocket / SSE      | `eventsource-stream 0.2` (root); `tokio-tungstenite` not present | `npm/src/openai-realtime.ts`, `npm/src/streaming.ts` | **Partial** — WS/SSE only on the TS side |
| Audio transcription (Whisper) | no `whisper-rs`, `whisper-rs-sys` deps | no whisper code | **Absent** |
| Video object detection (TensorFlow) | no `tract`, `tflite`, `tch`, `candle-onnx` deps | no detection code | **Absent** |

So 5 of 7 advertised modalities **have no implementation in this
repository**, only text references in dashboards or examples. The
remaining 2 (QUIC, WS/SSE) are partial.

This is a credibility problem for the project (users come for the
feature list, find nothing) and a planning problem (every PR review
has to relitigate whether multi-modal is in scope).

## Decision Drivers

- **README honesty.** The README is a contract; the code must back
  every claim.
- **Either implement or scope out.** Half-claims are worse than no
  claim — they trick consumers into starting integrations that fail.
- **Where multi-modal *does* belong**, it should sit behind a
  well-defined trait, not be sprinkled across the codebase.

## Considered Options

1. **Status quo.** Keep the claims; pretend the gaps don't exist.
2. **Implement everything claimed.** Audio transcription, video
   detection, RTMP, WebRTC, HLS — each is a multi-week investment.
   Total: months; well beyond the project's apparent scale.
3. **Trim the README to what the code actually backs**, and create
   ADR-tracked stubs (with status "Not started — owner needed") for
   any modality we intend to keep on the roadmap.
4. **Move multi-modal claims into a separate `midstream-media`
   sibling project** (new repo or new crate family) and link to it
   from the main README only when something ships.

## Decision Outcome

**Chosen option: Option 3 — trim the README and stub the future
work.**

Concretely:

- The README's "Streaming Integration" section is rewritten to list
  only **QUIC (transport)** and **WebSocket/SSE (text streaming)**.
- The "Audio transcription framework (Whisper-ready)" and "Video
  object detection framework (TensorFlow-ready)" bullets are
  deleted. The word "framework" in those phrases isn't doing any
  work — there's no framework, only the absence of one.
- "RTMP", "WebRTC", "HLS" are moved to a new `docs/ROADMAP.md`
  section titled "Multi-modal transport (not started)" with owners =
  `?` so anyone interested can claim.
- The `Multi-Modal Streaming` claim in `README.md:88` and the
  emoji-prefixed bullets in `:90` are rewritten or removed.

If/when audio or video lands, it goes behind a clean trait:

```rust
pub trait MediaSink: Send + Sync + 'static {
    async fn ingest(&self, frame: MediaFrame) -> Result<(), MediaError>;
}
```

…and is gated behind feature flags
(`provider-whisper`, `provider-yolo`, `transport-rtmp`, etc., per
[ADR-0025](0025-feature-flags-policy.md)). Until then, the README
does not claim them.

### Positive consequences

- The README stops over-promising. Trust restored.
- Reviewers gain a clear "is this in scope?" lookup for any future
  PR.
- The roadmap moves to a place where it can be discussed (issues,
  ADRs) instead of marketed.

### Negative consequences

- The README shrinks (~10 bullets removed). Marketing impact;
  necessary correction.
- Anyone who already integrated based on the README claim has to
  deal with the gap explicitly rather than discovering it during
  integration.

## Implementation notes

- Update `README.md` in the same PR that lands this ADR. Specifically
  remove or rewrite:
  - `:88` "Multi-Modal Streaming" feature heading,
  - `:90` "Multi-Modal Understanding" paragraph,
  - `:104-110` the streaming-integration bullet list (keep QUIC + WS/SSE),
  - `:67` the multi-modal callout in the "What is MidStream?" section.
- Move every "audio", "video", "rtmp", "webrtc", "hls" mention in
  `docs/` into `docs/ROADMAP.md` with a clear "not started" tag.
- The TS demos (`npm/examples/dashboard-demo.ts --mode audio` etc.)
  either run against synthetic data and stay as
  proofs-of-concept (clearly labelled), or get deleted. Decide in
  the same PR.

## Links

- Related: [ADR-0020](0020-docs-triage.md),
  [ADR-0021](0021-quic-implementation-quinn.md),
  [ADR-0025](0025-feature-flags-policy.md).
