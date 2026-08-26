//! Turns MidStream's analyzed stream chunks into latent frames. The embedding
//! is a deterministic hash-based projection — a stand-in with the same shape
//! as a real hidden-state slice, NOT a semantic embedding (the same honesty
//! rule as ruvector's AgenticDB warning: similar text does not produce
//! similar vectors here). Real deployments replace [`LatentEmitter::embed`]'s
//! output with actual model states; everything else (sequencing, provenance
//! hashing, authority defaults, wire encoding) is production-shaped.

use crate::frame::{AuthorityView, EncodingView, LatentFrameView, PayloadView, ProvenanceView};
use sha2::{Digest, Sha256};

/// Stateful per-stream emitter: monotonic sequence numbers, hashed context
/// provenance, `observe_only` authority by default (the receiver's gate — not
/// the sender — decides what a frame may influence).
#[derive(Debug, Clone)]
pub struct LatentEmitter {
    sender_model: String,
    receiver_space: String,
    transform_hash: String,
    dimensions: usize,
    next_sequence: u64,
    /// Mixed into both the placeholder embedding and the context hash.
    /// Empty = deterministic across streams, which makes hashes *linkable
    /// and guess-confirmable*: an observer who can guess a chunk's content
    /// can confirm the guess by hashing it. Supply a per-stream salt via
    /// [`LatentEmitter::with_salt`] when frames may cross an untrusted
    /// network and chunk contents are low-entropy.
    salt: Vec<u8>,
}

impl LatentEmitter {
    /// `transform_hash` identifies the alignment transform the receiving mesh
    /// trusts for this edge (LatentMesh ADR-002); an unknown hash is refused
    /// by the receiver's gate.
    pub fn new(
        sender_model: impl Into<String>,
        receiver_space: impl Into<String>,
        transform_hash: impl Into<String>,
        dimensions: usize,
    ) -> Self {
        LatentEmitter {
            sender_model: sender_model.into(),
            receiver_space: receiver_space.into(),
            transform_hash: transform_hash.into(),
            dimensions: dimensions.max(1),
            next_sequence: 0,
            salt: Vec::new(),
        }
    }

    /// Same emitter with a per-stream salt mixed into the embedding and the
    /// context hash, making provenance hashes unlinkable across streams and
    /// resistant to dictionary confirmation of low-entropy chunk contents.
    /// The receiver treats `context_hash` as opaque, so salting is free.
    pub fn with_salt(mut self, salt: impl Into<Vec<u8>>) -> Self {
        self.salt = salt.into();
        self
    }

    /// The sequence the next frame will carry.
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Deterministic placeholder embedding: SHA-256 expanded over the content
    /// and lane index, mapped to `[-1, 1]`. Same content, same vector — and
    /// explicitly NOT semantic similarity.
    fn embed(&self, content: &[u8]) -> Vec<f32> {
        let mut values = Vec::with_capacity(self.dimensions);
        let mut lane = 0u32;
        'outer: loop {
            let mut hasher = Sha256::new();
            hasher.update(b"midstream-latent-placeholder-v1");
            hasher.update(&self.salt);
            hasher.update(lane.to_be_bytes());
            hasher.update(content);
            for pair in hasher.finalize().chunks_exact(2) {
                let raw = u16::from_be_bytes([pair[0], pair[1]]);
                values.push(f32::from(raw) / f32::from(u16::MAX) * 2.0 - 1.0);
                if values.len() == self.dimensions {
                    break 'outer;
                }
            }
            lane += 1;
        }
        values
    }

    /// Emit one frame for an analyzed chunk. `content` is hashed into
    /// provenance — raw text never rides the frame (LatentMesh ADR-007's
    /// provenance rule).
    pub fn emit(&mut self, content: &[u8], confidence: f32, timestamp: u64) -> LatentFrameView {
        let values = self.embed(content);
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let context_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&self.salt);
            hasher.update(content);
            hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        LatentFrameView {
            id: format!("{}-{}", self.sender_model, sequence),
            sender_model: self.sender_model.clone(),
            receiver_space: self.receiver_space.clone(),
            transform_hash: self.transform_hash.clone(),
            sequence,
            payload: PayloadView {
                encoding: EncodingView::F32,
                dim: values.len(),
                bytes,
                int8_params: None,
            },
            confidence: confidence.clamp(0.0, 1.0),
            provenance: ProvenanceView {
                sender_model: self.sender_model.clone(),
                context_hash,
                parents: Vec::new(),
            },
            authority: AuthorityView::ObserveOnly,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequences_are_monotonic_and_content_is_hashed_not_carried() {
        let mut emitter = LatentEmitter::new("midstream", "mesh", "t0", 16);
        let a = emitter.emit(b"the user said something private", 0.8, 1);
        let b = emitter.emit(b"another chunk", 0.9, 2);
        assert_eq!(a.sequence, 0);
        assert_eq!(b.sequence, 1);
        assert_eq!(a.payload.dim, 16);
        // Provenance carries a hex hash, never the content.
        assert_eq!(a.provenance.context_hash.len(), 64);
        assert!(!a.provenance.context_hash.contains("private"));
        // Deterministic embedding: same content, same vector.
        let mut emitter2 = LatentEmitter::new("midstream", "mesh", "t0", 16);
        let a2 = emitter2.emit(b"the user said something private", 0.8, 1);
        assert_eq!(a.payload.bytes, a2.payload.bytes);
        // Sender defaults to the lowest authority rung.
        assert_eq!(a.authority, AuthorityView::ObserveOnly);
    }

    #[test]
    fn salted_emitters_produce_unlinkable_hashes() {
        let mut unsalted = LatentEmitter::new("m", "r", "t", 8);
        let mut salted = LatentEmitter::new("m", "r", "t", 8).with_salt(*b"stream-nonce-01!");
        let a = unsalted.emit(b"same content", 0.5, 0);
        let b = salted.emit(b"same content", 0.5, 0);
        assert_ne!(a.provenance.context_hash, b.provenance.context_hash);
        assert_ne!(a.payload.bytes, b.payload.bytes);
    }

    #[test]
    fn confidence_is_clamped() {
        let mut emitter = LatentEmitter::new("m", "r", "t", 4);
        assert_eq!(emitter.emit(b"x", 7.5, 0).confidence, 1.0);
        assert_eq!(emitter.emit(b"x", -1.0, 0).confidence, 0.0);
    }
}
