# 0041 — LatentMesh bridge: mirror the wire vocabulary, share a golden fixture

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** @ruvnet
- **Tags:** transport, workspace, integration

## Context and Problem Statement

LatentMesh (ruvnet/LatentMesh) streams agent hidden-state slices as
`LatentFrame` packets and, per its ADR-015, transports them over this
repository's published `midstreamer-quic 0.3` surface using a length-prefixed
JSON framing. MidStream needs to speak that framing — emit frames from its
analyzed stream chunks and decode inbound ones — but the latentmesh crates are
not published on crates.io, and a git dependency would couple every MidStream
build to a research prototype's repository. How does MidStream participate in
the latent streaming protocol without depending on unpublished code?

## Decision Drivers

- No git/path dependency on an unpublished research repository.
- Byte-level wire compatibility must be *tested*, not assumed.
- The bridge must ride the published `QuicTransport` embedding trait, which
  deliberately has no framing of its own (raw `send`/`recv`).
- Workspace lints (ADR-0034), proptest coverage (ADR-0038), and bench
  conventions (ADR-0009) apply to any new crate.

## Considered Options

1. **Git-dependency on `latentmesh-core`.** True single source of the types,
   but couples MidStream's build and MSRV to an unpublished prototype and
   breaks `--locked` reproducibility guarantees on any upstream force-push.
2. **Wait for latentmesh crates on crates.io.** Cleanest long-run, but blocks
   the live integration indefinitely on someone else's release schedule.
3. **Mirror the serde vocabulary + shared golden fixture.** A new
   `midstreamer-latentmesh` crate declares shape-exact mirror types
   (`LatentFrameView` et al.) and the same codec; a canonical encoded frame is
   checked into both repositories, and both CI suites assert decode +
   byte-exact re-encode against it, so drift fails a test instead of a
   deployment.

## Decision Outcome

**Chosen option: "3 — mirror + golden fixture"** because it delivers the live
integration now with a tested compatibility guarantee and zero build coupling;
if latentmesh crates are later published, the mirror types can be replaced by
a re-export without changing the wire.

### Positive consequences

- `crates/latentmesh-bridge` (`midstreamer-latentmesh`) ships the codec
  (4-byte big-endian length prefix + JSON, hard 1 MiB bound enforced before
  allocation), an incremental bounded `FrameDecoder`, `QuicFrameIo` over
  `QuicStream`, `open/accept_latent_stream` over any `QuicTransport`, and a
  `LatentEmitter` (monotonic sequencing, SHA-256 context provenance — raw
  text never rides a frame, `observe_only` default authority: the receiving
  mesh's gate decides influence, never the sender).
- Wire compatibility is a CI property on both sides of the repo boundary
  (`tests/golden_fixture.rs` here; `latentmesh-stream`'s golden test there).
- The placeholder hash-based embedding is documented as non-semantic, the
  same honesty rule ruvector's AgenticDB warning sets.

### Negative consequences

- The mirror types are a deliberate duplication: a *breaking* change to
  LatentMesh's frame vocabulary requires a coordinated fixture update in both
  repositories (the failing golden test is the coordination signal).
- JSON framing is larger than a binary codec; acceptable at the 1 MiB bound
  and revisit-able behind the same fixture discipline.

## Implementation notes

- `crates/latentmesh-bridge/` — `frame.rs`, `codec.rs`, `emitter.rs`,
  `quic.rs`; `testdata/latent_frame_golden.hex` (shared fixture);
  `tests/{golden_fixture,proptest_latent_codec}.rs`;
  `benches/latentmesh_codec_bench.rs`.
- Root example `examples/latentmesh_stream.rs` wires `Midstream::process_stream`
  output through the emitter to the wire encoding and back.

## Links

- Supersedes: none
- Superseded by: none
- Related: ADR-0021 (quinn transport), ADR-0011 (QUIC TLS), LatentMesh
  ADR-002/004/015 (frame vocabulary, streaming contract, live wiring)
