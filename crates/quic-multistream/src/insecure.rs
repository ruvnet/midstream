//! Insecure-mode TLS verifier — opt-in via the
//! `insecure-dev-only-skip-server-verification` feature flag.
//!
//! **NEVER** enable this feature in a production build. It exists so that
//! benches and integration tests can talk to self-signed loopback QUIC
//! servers (e.g. `benches/quic_bench.rs` and downstream test harnesses)
//! without bringing up a full PKI.
//!
//! The whole module is `#[cfg]`-gated; in default builds it does not
//! exist. See `crates/quic-multistream/src/native.rs` and
//! `docs/adr/0011-quic-tls-verification.md` for the rationale.

use std::sync::Arc;

/// `ServerCertVerifier` that accepts every certificate.
///
/// Intended ONLY for self-signed bench/test setups. Anyone constructing this
/// type in production code is doing something very wrong.
#[derive(Debug)]
pub(crate) struct SkipServerVerification(Arc<quinn::rustls::crypto::CryptoProvider>);

impl SkipServerVerification {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(
            quinn::rustls::crypto::ring::default_provider(),
        )))
    }
}

impl quinn::rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &quinn::rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[quinn::rustls::pki_types::CertificateDer<'_>],
        _server_name: &quinn::rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: quinn::rustls::pki_types::UnixTime,
    ) -> Result<quinn::rustls::client::danger::ServerCertVerified, quinn::rustls::Error> {
        Ok(quinn::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &quinn::rustls::pki_types::CertificateDer<'_>,
        dss: &quinn::rustls::DigitallySignedStruct,
    ) -> Result<quinn::rustls::client::danger::HandshakeSignatureValid, quinn::rustls::Error> {
        quinn::rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &quinn::rustls::pki_types::CertificateDer<'_>,
        dss: &quinn::rustls::DigitallySignedStruct,
    ) -> Result<quinn::rustls::client::danger::HandshakeSignatureValid, quinn::rustls::Error> {
        quinn::rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<quinn::rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}
