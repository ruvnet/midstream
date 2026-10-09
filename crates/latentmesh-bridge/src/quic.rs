//! LatentFrame framing over MidStream QUIC. `QuicTransport` (the published
//! embedding trait) provides streams; this module owns the framing on top —
//! the trait deliberately has no framing of its own (raw `send`/`recv`), so
//! the codec here is the contract both sides speak.

use crate::codec::{encode_frame, FrameDecoder};
use crate::frame::{BridgeError, LatentFrameView};
use midstreamer_quic::{QuicError, QuicStream, QuicTransport};

fn quic_err(e: QuicError) -> BridgeError {
    BridgeError::Transport(e.to_string())
}

/// Frame-oriented wrapper around one QUIC bidirectional stream.
pub struct QuicFrameIo {
    stream: QuicStream,
    decoder: FrameDecoder,
    read_buf: Vec<u8>,
}

impl QuicFrameIo {
    pub fn new(stream: QuicStream) -> Self {
        QuicFrameIo {
            stream,
            decoder: FrameDecoder::new(),
            read_buf: vec![0u8; 16 * 1024],
        }
    }

    /// Encode and send one frame.
    pub async fn send_frame(&mut self, frame: &LatentFrameView) -> Result<(), BridgeError> {
        let bytes = encode_frame(frame)?;
        self.stream.send(&bytes).await.map(|_| ()).map_err(quic_err)
    }

    /// Receive the next complete frame. `Ok(None)` is a clean end of stream;
    /// a stream that ends mid-frame is a transport error.
    pub async fn recv_frame(&mut self) -> Result<Option<LatentFrameView>, BridgeError> {
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(Some(frame));
            }
            let n = self
                .stream
                .recv(&mut self.read_buf)
                .await
                .map_err(quic_err)?;
            if n == 0 {
                if self.decoder.buffered() > 0 {
                    return Err(BridgeError::Transport("peer closed mid-frame".into()));
                }
                return Ok(None);
            }
            self.decoder.push(&self.read_buf[..n])?;
        }
    }

    /// Finish the send half gracefully.
    pub async fn finish(&mut self) -> Result<(), BridgeError> {
        self.stream.finish().await.map_err(quic_err)
    }
}

/// Open a latent stream on any [`QuicTransport`] implementor.
pub async fn open_latent_stream<T: QuicTransport + ?Sized>(
    transport: &T,
) -> Result<QuicFrameIo, BridgeError> {
    let stream = transport.open_bi_stream().await.map_err(quic_err)?;
    Ok(QuicFrameIo::new(stream))
}

/// Accept the peer's next latent stream.
pub async fn accept_latent_stream<T: QuicTransport + ?Sized>(
    transport: &T,
) -> Result<QuicFrameIo, BridgeError> {
    let stream = transport.accept_bi_stream().await.map_err(quic_err)?;
    Ok(QuicFrameIo::new(stream))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time bound assertion, matching midstreamer-quic's own
    /// convention: the helpers accept the concrete published connection type.
    #[test]
    fn quic_connection_satisfies_the_transport_bounds() {
        fn accepts<T: QuicTransport>() {
            let _ = open_latent_stream::<T>;
            let _ = accept_latent_stream::<T>;
        }
        let _ = accepts::<midstreamer_quic::QuicConnection>;
    }
}
