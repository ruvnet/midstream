//! Embedding trait for `QuicConnection`.
//!
//! Lets downstream crates write generic code against the QUIC
//! connection surface without binding to a concrete `quinn::Connection`.
//! Notable downstream: `ruflo-federation-peer`'s `TransportProvider`
//! impl (ADR-120 Step 3 — see ruvnet/ruflo) composes this trait with
//! the AIMDS 3-gate `SafetyGate` from `aimds-core` to run the
//! federation hop and the in-flight safety scan in one process.
//!
//! Available on **native** targets only — WASM targets use
//! WebTransport directly through the `wasm::QuicConnection` type, which
//! has a different stream lifecycle (no `accept_bi_stream`).
//!
//! ## Scope of v0.3.0
//!
//! The trait covers the existing async surface of `QuicConnection`
//! (`connect`, `open_bi_stream`, `open_uni_stream`, `accept_bi_stream`,
//! `stats`, `is_alive`) without breaking changes. The blanket
//! `impl QuicTransport for QuicConnection` is added immediately so
//! downstream crates can rely on the trait being present without
//! waiting on a behavior change.

#![cfg(not(target_arch = "wasm32"))]

use crate::{ConnectionStats, QuicError, QuicSendStream, QuicStream, StreamPriority};
use async_trait::async_trait;

/// Pluggable QUIC transport, abstracted over the concrete
/// `QuicConnection` so downstream crates can write generic code or
/// substitute alternative backends in tests.
///
/// All methods mirror the existing inherent methods on
/// `QuicConnection` so the [`QuicConnection`](crate::QuicConnection)
/// type satisfies this trait via a blanket impl below — no opt-in
/// adapter required.
#[async_trait]
pub trait QuicTransport: Send + Sync {
    /// Open a new bidirectional stream with default priority.
    async fn open_bi_stream(&self) -> Result<QuicStream, QuicError>;

    /// Open a new bidirectional stream with explicit priority.
    async fn open_bi_stream_with_priority(
        &self,
        priority: StreamPriority,
    ) -> Result<QuicStream, QuicError>;

    /// Open a unidirectional send stream.
    async fn open_uni_stream(&self) -> Result<QuicSendStream, QuicError>;

    /// Accept the next inbound bidirectional stream from this peer.
    async fn accept_bi_stream(&self) -> Result<QuicStream, QuicError>;

    /// Snapshot of bytes-sent/received counters for this connection.
    fn stats(&self) -> ConnectionStats;

    /// Initiate a graceful close. `error_code` and `reason` are
    /// surfaced to the peer per RFC 9000 connection-close semantics.
    fn close(&self, error_code: u64, reason: &[u8]);
}

#[async_trait]
impl QuicTransport for crate::QuicConnection {
    async fn open_bi_stream(&self) -> Result<QuicStream, QuicError> {
        crate::QuicConnection::open_bi_stream(self).await
    }

    async fn open_bi_stream_with_priority(
        &self,
        priority: StreamPriority,
    ) -> Result<QuicStream, QuicError> {
        crate::QuicConnection::open_bi_stream_with_priority(self, priority).await
    }

    async fn open_uni_stream(&self) -> Result<QuicSendStream, QuicError> {
        crate::QuicConnection::open_uni_stream(self).await
    }

    async fn accept_bi_stream(&self) -> Result<QuicStream, QuicError> {
        crate::QuicConnection::accept_bi_stream(self).await
    }

    fn stats(&self) -> ConnectionStats {
        crate::QuicConnection::stats(self)
    }

    fn close(&self, error_code: u64, reason: &[u8]) {
        crate::QuicConnection::close(self, error_code, reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms `QuicConnection: QuicTransport` at compile time so a
    /// downstream crate's `T: QuicTransport` bound accepts the concrete
    /// type. A doctest can't easily exercise this since constructing
    /// a real `QuicConnection` requires a peer.
    #[test]
    fn quic_connection_satisfies_quic_transport_bound() {
        fn requires_quic_transport<T: QuicTransport>() {}
        requires_quic_transport::<crate::QuicConnection>();
    }
}
