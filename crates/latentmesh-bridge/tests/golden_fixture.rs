//! Cross-repository golden fixture: the byte-exact wire encoding of one
//! canonical LatentMesh `LatentFrame`, identical to
//! `LatentMesh/crates/latentmesh-stream/testdata/latent_frame_golden.hex`.
//! Decoding it and re-encoding it must reproduce the exact bytes — that
//! byte-equality is what keeps this crate's mirror types and codec
//! compatible with the LatentMesh side without a shared dependency.

use midstreamer_latentmesh::{decode_frame, encode_frame, AuthorityView, EncodingView};
use std::path::PathBuf;

fn fixture_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/latent_frame_golden.hex");
    let hex = std::fs::read_to_string(path).expect("golden fixture present");
    let hex = hex.trim();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid hex"))
        .collect()
}

#[test]
fn golden_fixture_decodes_and_re_encodes_byte_for_byte() {
    let fixture = fixture_bytes();
    let (frame, consumed) = decode_frame(&fixture)
        .expect("fixture decodes")
        .expect("fixture is complete");
    assert_eq!(consumed, fixture.len());

    // The canonical frame's load-bearing fields, pinned.
    assert_eq!(frame.id, "golden-frame-0001");
    assert_eq!(frame.sender_model, "sender-model-a");
    assert_eq!(frame.receiver_space, "receiver-space-b");
    assert_eq!(frame.transform_hash, "golden-transform");
    assert_eq!(frame.sequence, 42);
    assert_eq!(frame.payload.encoding, EncodingView::Int8);
    assert_eq!(frame.payload.dim, 16);
    assert_eq!(frame.payload.bytes.len(), 16);
    assert!(frame.payload.int8_params.is_some());
    assert_eq!(frame.authority, AuthorityView::ContextInject);
    assert_eq!(frame.provenance.parents, vec!["parent-0000".to_string()]);
    assert_eq!(frame.timestamp, 1_756_166_400);

    // Byte-exact re-encode: serde shape, field order, and float formatting
    // all match the LatentMesh encoder.
    let re_encoded = encode_frame(&frame).expect("re-encodes");
    assert_eq!(
        re_encoded, fixture,
        "re-encoding diverged from the golden fixture — the mirror types have \
         drifted from latentmesh-core's serde shape"
    );
}
