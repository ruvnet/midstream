//! `midstreamer-latentmesh` — the MidStream side of the LatentMesh
//! live-streaming integration (ADR-0041; LatentMesh ADR-015).
//!
//! LatentMesh streams an agent's hidden-state slices as `LatentFrame` packets
//! (LatentMesh ADR-002) over MidStream's transport. This crate provides the
//! three pieces MidStream needs to participate without depending on the
//! unpublished latentmesh crates:
//!
//! - [`frame::LatentFrameView`] — a serde-shape-exact mirror of
//!   `latentmesh_core::LatentFrame` (field names, order, and enum casing),
//!   held byte-compatible by a golden fixture checked into both repositories;
//! - [`codec`] — the shared wire framing: 4-byte big-endian length prefix +
//!   JSON body, hard 1 MiB bound, incremental decoder;
//! - [`quic`] — send/receive helpers over `midstreamer_quic::QuicStream` and
//!   the published `QuicTransport` embedding trait;
//! - [`emitter::LatentEmitter`] — turns analyzed stream chunks into frames
//!   with monotonic sequencing and hashed (never raw) context provenance.
//!
//! Authority semantics are LatentMesh's: frames emitted here default to
//! `observe_only` — the *receiving* mesh's gate decides what any frame may
//! influence, never the sender.

pub mod codec;
pub mod emitter;
pub mod frame;
pub mod quic;

pub use codec::{decode_frame, encode_frame, FrameDecoder, MAX_FRAME_BYTES};
pub use emitter::LatentEmitter;
pub use frame::{
    AuthorityView, BridgeError, EncodingView, LatentFrameView, PayloadView, ProvenanceView,
};
pub use quic::{accept_latent_stream, open_latent_stream, QuicFrameIo};
