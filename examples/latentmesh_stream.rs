//! LatentMesh bridge demo (ADR-0041): stream an LLM's chunks through
//! MidStream and emit each analyzed chunk as a LatentMesh `LatentFrame` on
//! the wire encoding a LatentMesh receiver decodes.
//!
//! Run: `cargo run --example latentmesh_stream`

use bytes::Bytes;
use futures::stream::{iter, BoxStream};
use midstream::{LLMClient, Midstream, StreamProcessor};
use midstreamer_latentmesh::{decode_frame, encode_frame, LatentEmitter};

struct DemoLLM;

impl LLMClient for DemoLLM {
    fn stream(&self) -> BoxStream<'static, Bytes> {
        Box::pin(iter(vec![
            Bytes::from_static(b"analyzing the incident timeline"),
            Bytes::from_static(b"the scheduler stalled at 14:02"),
            Bytes::from_static(b"root cause: unbounded retry queue"),
        ]))
    }
}

struct NoopHypr;

#[async_trait::async_trait]
impl midstream::HyprService for NoopHypr {
    async fn ingest_metric(
        &self,
        _metric: midstream::MetricRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn query_aggregate(
        &self,
        _window: midstream::TimeWindow,
        _func: midstream::AggregateFunction,
    ) -> Result<f64, Box<dyn std::error::Error>> {
        Ok(0.0)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. MidStream processes the live LLM stream as usual.
    let midstream = Midstream::new(Box::new(DemoLLM), Box::new(NoopHypr));
    let messages = midstream.process_stream().await?;

    // 2. Each analyzed chunk becomes a latent frame: monotonic sequence,
    //    hashed provenance, observe-only authority (the receiving mesh's
    //    gate decides everything else).
    let mut emitter = LatentEmitter::new("midstream-demo", "mesh-receiver", "demo-transform", 64);
    let mut wire = Vec::new();
    for message in &messages {
        let ts = message.timestamp.timestamp().max(0) as u64;
        let frame = emitter.emit(&message.content, 0.8, ts);
        wire.extend_from_slice(&encode_frame(&frame)?);
    }
    println!(
        "emitted {} frames, {} bytes on the wire",
        messages.len(),
        wire.len()
    );

    // 3. The receiving side (LatentMesh's `latentmesh-stream` speaks this
    //    exact framing) decodes them back out of the byte stream.
    let mut cursor = &wire[..];
    while let Some((frame, consumed)) = decode_frame(cursor)? {
        println!(
            "  frame seq={} dim={} authority={:?} context_hash={}…",
            frame.sequence,
            frame.payload.dim,
            frame.authority,
            &frame.provenance.context_hash[..12]
        );
        cursor = &cursor[consumed..];
    }
    Ok(())
}
