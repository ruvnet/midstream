//! Serde-shape-exact mirrors of the LatentMesh packet vocabulary
//! (`latentmesh-core`, LatentMesh ADR-002). Field names, declaration order,
//! and enum casing must not drift: the golden-fixture test re-encodes the
//! canonical frame byte-for-byte, and the same fixture is tested in the
//! LatentMesh repository.

use serde::{Deserialize, Serialize};

/// Bridge-side failures. Malformed wire input is an error, never a panic.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("frame of {declared} bytes exceeds the {max} byte bound")]
    FrameTooLarge { declared: usize, max: usize },
    #[error("malformed frame: {0}")]
    Malformed(String),
    #[error("transport failure: {0}")]
    Transport(String),
}

/// Mirror of `latentmesh_core::Encoding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncodingView {
    F32,
    F16,
    Int8,
}

/// Mirror of `latentmesh_core::Payload`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadView {
    pub encoding: EncodingView,
    pub dim: usize,
    pub bytes: Vec<u8>,
    pub int8_params: Option<(f32, i32)>,
}

/// Mirror of `latentmesh_core::Provenance`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceView {
    pub sender_model: String,
    pub context_hash: String,
    pub parents: Vec<String>,
}

/// Mirror of `latentmesh_core::Authority` — the risk-ordered ladder. The
/// derived `Ord` matches LatentMesh's (declaration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityView {
    ObserveOnly,
    ContextInject,
    LatentPrefix,
    ActionInfluencing,
}

/// Mirror of `latentmesh_core::LatentFrame` — the latent packet itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatentFrameView {
    pub id: String,
    pub sender_model: String,
    pub receiver_space: String,
    pub transform_hash: String,
    pub sequence: u64,
    pub payload: PayloadView,
    pub confidence: f32,
    pub provenance: ProvenanceView,
    pub authority: AuthorityView,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_ladder_orders_by_risk() {
        assert!(AuthorityView::ObserveOnly < AuthorityView::ActionInfluencing);
        assert!(AuthorityView::ContextInject < AuthorityView::LatentPrefix);
    }

    #[test]
    fn enum_casing_matches_the_latentmesh_wire() {
        assert_eq!(
            serde_json::to_string(&EncodingView::Int8).expect("serializes"),
            "\"int8\""
        );
        assert_eq!(
            serde_json::to_string(&AuthorityView::ObserveOnly).expect("serializes"),
            "\"observe_only\""
        );
    }
}
