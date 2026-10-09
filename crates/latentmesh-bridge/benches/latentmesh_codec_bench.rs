//! LatentMesh bridge benchmarks. Coverage:
//! - frame encode (512-dim F32 payload) — target: <50 µs per frame
//! - frame decode — target: <50 µs per frame
//! - emitter end-to-end (embed + hash + encode) — target: <200 µs per chunk
//!
//! These bound the per-chunk overhead the bridge adds to MidStream's hot
//! path; they say nothing about network latency.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use midstreamer_latentmesh::{decode_frame, encode_frame, LatentEmitter};

fn bench_codec(c: &mut Criterion) {
    let mut emitter = LatentEmitter::new("bench", "mesh", "t0", 512);
    let frame = emitter.emit(b"a representative analyzed chunk of stream text", 0.9, 1);
    let wire = encode_frame(&frame).expect("encodes");

    let mut group = c.benchmark_group("latentmesh_codec");
    group.throughput(Throughput::Bytes(wire.len() as u64));
    group.bench_function("encode_512d_f32", |b| {
        b.iter(|| encode_frame(black_box(&frame)).expect("encodes"))
    });
    group.bench_function("decode_512d_f32", |b| {
        b.iter(|| {
            decode_frame(black_box(&wire))
                .expect("decodes")
                .expect("complete")
        })
    });
    group.finish();
}

fn bench_emitter(c: &mut Criterion) {
    let mut emitter = LatentEmitter::new("bench", "mesh", "t0", 512);
    let chunk = vec![b'x'; 4096];
    c.bench_function("emit_4k_chunk_512d", |b| {
        b.iter(|| {
            let frame = emitter.emit(black_box(&chunk), 0.9, 1);
            encode_frame(&frame).expect("encodes")
        })
    });
}

criterion_group!(benches, bench_codec, bench_emitter);
criterion_main!(benches);
