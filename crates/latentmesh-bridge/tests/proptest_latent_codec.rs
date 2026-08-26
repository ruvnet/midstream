//! Property tests (ADR-0038 convention): the codec round-trips every
//! well-formed frame, and the incremental decoder agrees with the one-shot
//! decoder under arbitrary chunking.

#![allow(clippy::unwrap_used)]

use midstreamer_latentmesh::{
    decode_frame, encode_frame, AuthorityView, EncodingView, FrameDecoder, LatentFrameView,
    PayloadView, ProvenanceView,
};
use proptest::prelude::*;

fn arb_authority() -> impl Strategy<Value = AuthorityView> {
    prop_oneof![
        Just(AuthorityView::ObserveOnly),
        Just(AuthorityView::ContextInject),
        Just(AuthorityView::LatentPrefix),
        Just(AuthorityView::ActionInfluencing),
    ]
}

fn arb_encoding() -> impl Strategy<Value = EncodingView> {
    prop_oneof![
        Just(EncodingView::F32),
        Just(EncodingView::F16),
        Just(EncodingView::Int8),
    ]
}

prop_compose! {
    fn arb_frame()(
        id in "[a-z0-9-]{1,24}",
        sender in "[a-z0-9-]{1,16}",
        receiver in "[a-z0-9-]{1,16}",
        transform in "[a-f0-9]{1,32}",
        sequence in any::<u64>(),
        encoding in arb_encoding(),
        bytes in proptest::collection::vec(any::<u8>(), 0..256),
        confidence in 0.0f32..=1.0,
        context in "[a-f0-9]{1,64}",
        authority in arb_authority(),
        timestamp in any::<u64>(),
    ) -> LatentFrameView {
        LatentFrameView {
            id,
            sender_model: sender.clone(),
            receiver_space: receiver,
            transform_hash: transform,
            sequence,
            payload: PayloadView {
                encoding,
                dim: bytes.len(),
                bytes,
                int8_params: if encoding == EncodingView::Int8 { Some((0.5, 12)) } else { None },
            },
            confidence,
            provenance: ProvenanceView {
                sender_model: sender,
                context_hash: context,
                parents: vec![],
            },
            authority,
            timestamp,
        }
    }
}

proptest! {
    #[test]
    fn codec_round_trips_every_well_formed_frame(frame in arb_frame()) {
        let bytes = encode_frame(&frame).unwrap();
        let (back, consumed) = decode_frame(&bytes).unwrap().unwrap();
        prop_assert_eq!(consumed, bytes.len());
        prop_assert_eq!(back, frame);
    }

    #[test]
    fn incremental_decoder_agrees_under_arbitrary_chunking(
        frames in proptest::collection::vec(arb_frame(), 1..4),
        chunk in 1usize..64,
    ) {
        let mut wire = Vec::new();
        for f in &frames {
            wire.extend_from_slice(&encode_frame(f).unwrap());
        }
        let mut decoder = FrameDecoder::new();
        let mut seen = Vec::new();
        for piece in wire.chunks(chunk) {
            decoder.push(piece).unwrap();
            while let Some(f) = decoder.next_frame().unwrap() {
                seen.push(f);
            }
        }
        prop_assert_eq!(seen, frames);
        prop_assert_eq!(decoder.buffered(), 0);
    }

    #[test]
    fn arbitrary_garbage_never_panics(junk in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = decode_frame(&junk);
        let mut decoder = FrameDecoder::new();
        let _ = decoder.push(&junk);
        let _ = decoder.next_frame();
    }
}
