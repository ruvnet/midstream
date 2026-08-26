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

/// Encode one frame as `[len: u32 BE][json bytes]`.
pub fn encode_frame(frame: &LatentFrameView) -> Result<Vec<u8>, BridgeError> {
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
    Ok(Some((frame, total)))
}

/// Incremental decoder for chunked byte streams (the QUIC receive path).
/// Buffering is bounded: an oversized declared length is rejected before the
/// body is buffered, so a peer cannot grow memory past the frame bound.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes currently buffered awaiting a complete frame.
    pub fn buffered(&self) -> usize {
        self.buffer.len()
    }

    /// Append received bytes, failing fast on an oversized declared length.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), BridgeError> {
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
