//! The shared LatentMesh wire framing: `[len: u32 BE][serde-JSON body]` with
//! a hard byte bound enforced before allocation on the receive path. This
//! must stay byte-identical to `latentmesh-stream`'s codec — the golden
//! fixture test is the tripwire.

use crate::frame::{BridgeError, LatentFrameView};

/// Hard upper bound on one encoded frame (prefix excluded) — the same 1 MiB
/// bound LatentMesh enforces.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Bytes of the big-endian length prefix.
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// Encode one frame as `[len: u32 BE][json bytes]`. Shape-validates first,
/// so a locally-constructed inconsistent frame is caught at the sender
/// instead of by the peer's decoder.
pub fn encode_frame(frame: &LatentFrameView) -> Result<Vec<u8>, BridgeError> {
    validate_payload_shape(frame)?;
    let body = serde_json::to_vec(frame).map_err(|e| BridgeError::Malformed(e.to_string()))?;
    if body.len() > MAX_FRAME_BYTES {
        return Err(BridgeError::FrameTooLarge {
            declared: body.len(),
            max: MAX_FRAME_BYTES,
        });
    }
    let mut out = Vec::with_capacity(LENGTH_PREFIX_BYTES + body.len());
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decode one frame from the front of `input`, returning the frame and bytes
/// consumed. `Ok(None)` means more bytes are needed.
pub fn decode_frame(input: &[u8]) -> Result<Option<(LatentFrameView, usize)>, BridgeError> {
    if input.len() < LENGTH_PREFIX_BYTES {
        return Ok(None);
    }
    let declared = u32::from_be_bytes([input[0], input[1], input[2], input[3]]) as usize;
    if declared > MAX_FRAME_BYTES {
        return Err(BridgeError::FrameTooLarge {
            declared,
            max: MAX_FRAME_BYTES,
        });
    }
    let total = LENGTH_PREFIX_BYTES + declared;
    if input.len() < total {
        return Ok(None);
    }
    let frame: LatentFrameView = serde_json::from_slice(&input[LENGTH_PREFIX_BYTES..total])
        .map_err(|e| BridgeError::Malformed(e.to_string()))?;
    validate_payload_shape(&frame)?;
    Ok(Some((frame, total)))
}

/// Reject frames whose payload shape is internally inconsistent, mirroring
/// the LatentMesh-side check: `bytes.len()` must equal `dim` times the
/// encoding's bytes-per-element, `int8` must carry finite dequantization
/// params, and non-int8 payloads must not carry them. Checked at the wire
/// boundary so no consumer ever trusts a forged `dim`.
pub fn validate_payload_shape(frame: &LatentFrameView) -> Result<(), BridgeError> {
    let payload = &frame.payload;
    let per_element = match payload.encoding {
        crate::frame::EncodingView::F32 => 4usize,
        crate::frame::EncodingView::F16 => 2,
        crate::frame::EncodingView::Int8 => 1,
    };
    let expected = payload
        .dim
        .checked_mul(per_element)
        .ok_or_else(|| BridgeError::Malformed("payload dim overflows".into()))?;
    if payload.bytes.len() != expected {
        return Err(BridgeError::Malformed(format!(
            "payload declares dim {} ({expected} bytes) but carries {} bytes",
            payload.dim,
            payload.bytes.len()
        )));
    }
    match (payload.encoding, payload.int8_params) {
        (crate::frame::EncodingView::Int8, None) => Err(BridgeError::Malformed(
            "int8 payload without dequantization params".into(),
        )),
        (crate::frame::EncodingView::Int8, Some((scale, _))) if !scale.is_finite() => {
            Err(BridgeError::Malformed("int8 scale is not finite".into()))
        }
        (crate::frame::EncodingView::F32 | crate::frame::EncodingView::F16, Some(_)) => Err(
            BridgeError::Malformed("non-int8 payload carries int8 params".into()),
        ),
        _ => Ok(()),
    }
}

/// Incremental decoder for chunked byte streams (the QUIC receive path).
/// Buffering is hard-bounded: `push` refuses growth past
/// [`MAX_BUFFERED_BYTES`] even when the caller never drains.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

/// Hard cap on the decoder's internal buffer: one maximal frame in flight
/// plus one maximal frame of read-ahead.
pub const MAX_BUFFERED_BYTES: usize = 2 * (MAX_FRAME_BYTES + LENGTH_PREFIX_BYTES);

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes currently buffered awaiting a complete frame.
    pub fn buffered(&self) -> usize {
        self.buffer.len()
    }

    /// Append received bytes, failing fast on an oversized declared length
    /// and refusing to grow past [`MAX_BUFFERED_BYTES`] regardless of
    /// content — call [`FrameDecoder::next_frame`] to drain.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), BridgeError> {
        if self.buffer.len().saturating_add(chunk.len()) > MAX_BUFFERED_BYTES {
            self.buffer.clear();
            return Err(BridgeError::Transport(
                "decoder buffer bound exceeded (caller must drain frames)".into(),
            ));
        }
        self.buffer.extend_from_slice(chunk);
        if self.buffer.len() >= LENGTH_PREFIX_BYTES {
            let declared = u32::from_be_bytes([
                self.buffer[0],
                self.buffer[1],
                self.buffer[2],
                self.buffer[3],
            ]) as usize;
            if declared > MAX_FRAME_BYTES {
                self.buffer.clear();
                return Err(BridgeError::FrameTooLarge {
                    declared,
                    max: MAX_FRAME_BYTES,
                });
            }
        }
        Ok(())
    }

    /// Pop the next complete frame, if one is buffered.
    pub fn next_frame(&mut self) -> Result<Option<LatentFrameView>, BridgeError> {
        match decode_frame(&self.buffer)? {
            Some((frame, consumed)) => {
                self.buffer.drain(..consumed);
                Ok(Some(frame))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{AuthorityView, EncodingView, PayloadView, ProvenanceView};

    fn frame(seq: u64) -> LatentFrameView {
        LatentFrameView {
            id: format!("f{seq}"),
            sender_model: "midstream".into(),
            receiver_space: "mesh".into(),
            transform_hash: "t".into(),
            sequence: seq,
            payload: PayloadView {
                encoding: EncodingView::F32,
                dim: 2,
                bytes: vec![0, 0, 128, 63, 0, 0, 0, 64],
                int8_params: None,
            },
            confidence: 0.75,
            provenance: ProvenanceView {
                sender_model: "midstream".into(),
                context_hash: "c".into(),
                parents: vec![],
            },
            authority: AuthorityView::ObserveOnly,
            timestamp: 1,
        }
    }

    #[test]
    fn round_trips_through_the_codec() {
        let f = frame(9);
        let bytes = encode_frame(&f).expect("encodes");
        let (back, consumed) = decode_frame(&bytes).expect("decodes").expect("complete");
        assert_eq!(consumed, bytes.len());
        assert_eq!(back, f);
    }

    #[test]
    fn truncated_input_asks_for_more_bytes() {
        let bytes = encode_frame(&frame(1)).expect("encodes");
        for cut in 0..bytes.len() {
            assert!(decode_frame(&bytes[..cut]).expect("no error").is_none());
        }
    }

    #[test]
    fn oversized_declared_length_is_rejected_before_allocation() {
        let mut bytes = ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(matches!(
            decode_frame(&bytes),
            Err(BridgeError::FrameTooLarge { .. })
        ));
        let mut decoder = FrameDecoder::new();
        assert!(decoder.push(&bytes).is_err());
        assert_eq!(decoder.buffered(), 0);
    }

    #[test]
    fn corrupt_body_is_a_typed_error() {
        let mut bytes = encode_frame(&frame(1)).expect("encodes");
        let last = bytes.len() - 1;
        bytes[last] = b'!';
        assert!(matches!(
            decode_frame(&bytes),
            Err(BridgeError::Malformed(_))
        ));
    }

    /// Craft wire bytes for a frame without going through `encode_frame`'s
    /// own validation, to prove the decoder independently rejects them.
    fn raw_wire(frame: &LatentFrameView) -> Vec<u8> {
        let body = serde_json::to_vec(frame).expect("serializes");
        let mut out = (body.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn shape_mismatched_payloads_are_rejected_by_encoder_and_decoder() {
        let mut bad_dim = frame(1);
        bad_dim.payload.dim = 999;
        assert!(matches!(
            encode_frame(&bad_dim),
            Err(BridgeError::Malformed(_))
        ));
        assert!(matches!(
            decode_frame(&raw_wire(&bad_dim)),
            Err(BridgeError::Malformed(_))
        ));

        let mut stray_params = frame(2);
        stray_params.payload.int8_params = Some((1.0, 0));
        assert!(matches!(
            encode_frame(&stray_params),
            Err(BridgeError::Malformed(_))
        ));
        assert!(matches!(
            decode_frame(&raw_wire(&stray_params)),
            Err(BridgeError::Malformed(_))
        ));
    }

    #[test]
    fn decoder_refuses_unbounded_buffering_when_never_drained() {
        let mut decoder = FrameDecoder::new();
        let frame_bytes = encode_frame(&frame(1)).expect("encodes");
        let mut overflowed = false;
        for _ in 0..(MAX_BUFFERED_BYTES / frame_bytes.len() + 2) {
            if decoder.push(&frame_bytes).is_err() {
                overflowed = true;
                break;
            }
        }
        assert!(overflowed);
        assert_eq!(decoder.buffered(), 0);
    }

    #[test]
    fn incremental_decoder_reassembles_across_chunk_boundaries() {
        let mut wire = Vec::new();
        for seq in 0..3 {
            wire.extend_from_slice(&encode_frame(&frame(seq)).expect("encodes"));
        }
        let mut decoder = FrameDecoder::new();
        let mut seen = Vec::new();
        for chunk in wire.chunks(7) {
            decoder.push(chunk).expect("push");
            while let Some(f) = decoder.next_frame().expect("decode") {
                seen.push(f.sequence);
            }
        }
        assert_eq!(seen, vec![0, 1, 2]);
    }
}
