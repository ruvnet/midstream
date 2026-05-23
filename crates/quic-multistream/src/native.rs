//! Native QUIC implementation using quinn

use crate::{ConnectionStats, QuicError, StreamPriority};
use quinn::{ClientConfig, Endpoint, RecvStream, SendStream, VarInt};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// QUIC connection wrapper for native targets
pub struct QuicConnection {
    connection: quinn::Connection,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
}

impl QuicConnection {
    /// Connect to a QUIC server
    ///
    /// # Arguments
    /// * `addr` - Server address (e.g., "localhost:4433")
    ///
    /// # Examples
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use midstreamer_quic::QuicConnection;
    ///
    /// let connection = QuicConnection::connect("localhost:4433").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(addr: &str) -> Result<Self, QuicError> {
        // Parse address
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| QuicError::InvalidConfig(e.to_string()))?
            .next()
            .ok_or_else(|| QuicError::InvalidConfig("Invalid address".to_string()))?;

        // Build the TLS client config. Per ADR-0011 the default verifier is
        // the OS platform trust store; consumers can opt into a skip-verify
        // verifier via the `insecure-dev-only-skip-server-verification`
        // cargo feature (intended for self-signed bench/test servers ONLY).
        let mut crypto = build_client_tls_config()?;

        // Enable ALPN for QUIC
        crypto.alpn_protocols = vec![b"h3".to_vec()];

        let client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
                .map_err(|e| QuicError::TlsError(format!("{:?}", e)))?,
        ));

        // Create endpoint bound to an ephemeral local port.
        // The literal "0.0.0.0:0" is always a valid SocketAddr, so the parse
        // is infallible; map_err covers the OS bind failure that Endpoint::client
        // can still return.
        let bind_addr: std::net::SocketAddr = "0.0.0.0:0"
            .parse()
            .map_err(|e: std::net::AddrParseError| QuicError::InvalidConfig(e.to_string()))?;
        let mut endpoint =
            Endpoint::client(bind_addr).map_err(|e| QuicError::ConnectionFailed(e.to_string()))?;
        endpoint.set_default_client_config(client_config);

        // Connect to server
        let connection = endpoint
            .connect(socket_addr, "localhost")
            .map_err(|e| QuicError::ConnectionFailed(e.to_string()))?
            .await?;

        Ok(Self {
            connection,
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Open a bidirectional stream
    pub async fn open_bi_stream(&self) -> Result<QuicStream, QuicError> {
        let (send, recv) = self.connection.open_bi().await?;
        Ok(QuicStream::new(
            send,
            recv,
            self.bytes_sent.clone(),
            self.bytes_received.clone(),
        ))
    }

    /// Open a bidirectional stream with priority
    pub async fn open_bi_stream_with_priority(
        &self,
        priority: StreamPriority,
    ) -> Result<QuicStream, QuicError> {
        let (send, recv) = self.connection.open_bi().await?;
        let mut stream = QuicStream::new(
            send,
            recv,
            self.bytes_sent.clone(),
            self.bytes_received.clone(),
        );
        stream.set_priority(priority);
        Ok(stream)
    }

    /// Open a unidirectional stream (send-only)
    pub async fn open_uni_stream(&self) -> Result<QuicSendStream, QuicError> {
        let send = self.connection.open_uni().await?;
        Ok(QuicSendStream::new(send, self.bytes_sent.clone()))
    }

    /// Accept an incoming bidirectional stream
    pub async fn accept_bi_stream(&self) -> Result<QuicStream, QuicError> {
        let (send, recv) = self
            .connection
            .accept_bi()
            .await
            .map_err(|e| QuicError::ConnectionClosed(e.to_string()))?;
        Ok(QuicStream::new(
            send,
            recv,
            self.bytes_sent.clone(),
            self.bytes_received.clone(),
        ))
    }

    /// Get connection statistics
    pub fn stats(&self) -> ConnectionStats {
        let stats = self.connection.stats();
        ConnectionStats {
            active_bi_streams: 0, // Not available in quinn stats
            active_uni_streams: 0,
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            rtt_ms: stats.path.rtt.as_millis() as f64,
        }
    }

    /// Close the connection.
    ///
    /// `error_code` is clamped to [`VarInt::MAX`] (2^62 − 1) if it exceeds
    /// the QUIC variable-length integer range, rather than panicking. Values
    /// within range are forwarded unchanged.
    pub fn close(&self, error_code: u64, reason: &[u8]) {
        // VarInt::from_u64 returns None for values > 2^62-1. Clamp rather
        // than panic so callers passing a raw OS error code can't crash the
        // process.
        let code = VarInt::from_u64(error_code).unwrap_or(VarInt::MAX);
        self.connection.close(code, reason);
    }

    /// Get the remote address
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }
}

/// Bidirectional QUIC stream
pub struct QuicStream {
    send: SendStream,
    recv: RecvStream,
    priority: StreamPriority,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
}

impl QuicStream {
    fn new(
        send: SendStream,
        recv: RecvStream,
        bytes_sent: Arc<AtomicU64>,
        bytes_received: Arc<AtomicU64>,
    ) -> Self {
        Self {
            send,
            recv,
            priority: StreamPriority::default(),
            bytes_sent,
            bytes_received,
        }
    }

    /// Send data on the stream
    pub async fn send(&mut self, data: &[u8]) -> Result<usize, QuicError> {
        self.send.write_all(data).await?;
        self.bytes_sent
            .fetch_add(data.len() as u64, Ordering::Relaxed);
        Ok(data.len())
    }

    /// Receive data from the stream
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, QuicError> {
        let n = self.recv.read(buf).await?.unwrap_or(0);
        self.bytes_received.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }

    /// Finish sending on this stream
    pub async fn finish(&mut self) -> Result<(), QuicError> {
        self.send
            .finish()
            .map_err(|e| QuicError::StreamError(format!("Failed to finish stream: {:?}", e)))?;
        Ok(())
    }

    /// Set stream priority
    pub fn set_priority(&mut self, priority: StreamPriority) {
        self.priority = priority;
        // Note: quinn doesn't directly expose priority setting
        // This would typically be handled at the application level
    }

    /// Get current priority
    pub fn priority(&self) -> StreamPriority {
        self.priority
    }
}

/// Unidirectional send-only stream
pub struct QuicSendStream {
    send: SendStream,
    bytes_sent: Arc<AtomicU64>,
}

impl QuicSendStream {
    fn new(send: SendStream, bytes_sent: Arc<AtomicU64>) -> Self {
        Self { send, bytes_sent }
    }

    /// Send data on the stream
    pub async fn send(&mut self, data: &[u8]) -> Result<usize, QuicError> {
        self.send.write_all(data).await?;
        self.bytes_sent
            .fetch_add(data.len() as u64, Ordering::Relaxed);
        Ok(data.len())
    }

    /// Finish sending on this stream
    pub async fn finish(&mut self) -> Result<(), QuicError> {
        self.send
            .finish()
            .map_err(|e| QuicError::StreamError(format!("Failed to finish stream: {:?}", e)))?;
        Ok(())
    }
}

/// Build the `rustls::ClientConfig` used by `QuicConnection::connect`.
///
/// In default builds this returns a config wired to the OS platform trust
/// store via `rustls-platform-verifier` (per ADR-0011). When the
/// `insecure-dev-only-skip-server-verification` cargo feature is enabled
/// the config substitutes a verifier that accepts every certificate and
/// emits a runtime warning via `tracing::warn!`. That mode is intended for
/// self-signed bench/test setups only and MUST NEVER be used in production
/// builds.
#[cfg(not(feature = "insecure-dev-only-skip-server-verification"))]
fn build_client_tls_config() -> Result<quinn::rustls::ClientConfig, QuicError> {
    use rustls_platform_verifier::BuilderVerifierExt;

    quinn::rustls::ClientConfig::builder()
        .with_platform_verifier()
        .map(|builder| builder.with_no_client_auth())
        .map_err(|e| QuicError::TlsError(format!("platform verifier init failed: {e:?}")))
}

#[cfg(feature = "insecure-dev-only-skip-server-verification")]
fn build_client_tls_config() -> Result<quinn::rustls::ClientConfig, QuicError> {
    tracing::warn!(
        target: "midstreamer_quic",
        "TLS server certificate verification is DISABLED \
         (feature `insecure-dev-only-skip-server-verification`). \
         This build accepts any certificate from any peer. \
         NEVER ship a release built with this feature enabled."
    );

    Ok(quinn::rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(crate::insecure::SkipServerVerification::new())
        .with_no_client_auth())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_stats_tracking() {
        let bytes_sent = Arc::new(AtomicU64::new(100));
        let bytes_received = Arc::new(AtomicU64::new(200));

        assert_eq!(bytes_sent.load(Ordering::Relaxed), 100);
        assert_eq!(bytes_received.load(Ordering::Relaxed), 200);

        bytes_sent.fetch_add(50, Ordering::Relaxed);
        assert_eq!(bytes_sent.load(Ordering::Relaxed), 150);
    }

    #[test]
    fn test_priority_values() {
        assert_eq!(StreamPriority::default(), StreamPriority::Normal);
        assert!(StreamPriority::Critical < StreamPriority::High);
    }
}
