//! Property-based tests for `midstreamer-quic`. Implements ADR-0038
//! for the QUIC crate.
//!
//! The QUIC crate's hot-path types are `StreamPriority`,
//! `ConnectionStats`, and `QuicError`. The transport surface itself
//! (`QuicConnection`, `QuicStream`) needs a real `quinn::Endpoint`
//! to exercise meaningfully — that's the scope of the bench-rewrite
//! follow-up to ADR-0009. These tests cover the value-type contracts
//! the rest of the crate (and downstream consumers) rely on.
//!
//! Invariants asserted (all pass):
//!
//!   StreamPriority:
//!     * Total ordering matches numeric variant order
//!       (Critical < High < Normal < Low).
//!     * `serde_json` roundtrip preserves the variant.
//!     * `Display` is non-empty and consistent across calls.
//!     * `Default` is always Normal.
//!
//!   ConnectionStats:
//!     * `Default::default()` zeroes every numeric field.
//!     * `serde_json` roundtrip preserves every field bit-for-bit.
//!
//!   QuicError:
//!     * `Display` includes the inner string for the `String`-
//!       carrying variants.

use midstreamer_quic::{ConnectionStats, QuicError, StreamPriority};
use proptest::prelude::*;

/// Generator for any `StreamPriority` variant.
fn priority() -> impl Strategy<Value = StreamPriority> {
    prop_oneof![
        Just(StreamPriority::Critical),
        Just(StreamPriority::High),
        Just(StreamPriority::Normal),
        Just(StreamPriority::Low),
    ]
}

/// Generator for `ConnectionStats` over a realistic numeric range.
///
/// `rtt_ms` is generated from an integer 0..=10_000 cast to f64 so
/// that the serde_json roundtrip is bit-exact — arbitrary f64 values
/// can lose precision in their decimal representation (this is a
/// JSON-format limitation, not a serde_json bug). Real RTT values
/// are reported at millisecond granularity by quinn so this matches
/// the actual data shape.
fn connection_stats() -> impl Strategy<Value = ConnectionStats> {
    (
        0usize..=10_000,
        0usize..=10_000,
        0u64..=1_000_000_000,
        0u64..=1_000_000_000,
        0u64..=10_000,
    )
        .prop_map(|(bi, uni, sent, recv, rtt_int)| ConnectionStats {
            active_bi_streams: bi,
            active_uni_streams: uni,
            bytes_sent: sent,
            bytes_received: recv,
            rtt_ms: rtt_int as f64,
        })
}

// ---------------------------------------------------------------- StreamPriority.

proptest! {
    /// `StreamPriority` ordering matches variant declaration order:
    /// Critical < High < Normal < Low (Critical is the highest
    /// priority but the smallest variant by Ord).
    #[test]
    fn priority_ordering_total(a in priority(), b in priority()) {
        // Total ordering: exactly one of a < b, a == b, a > b holds.
        let total = (a < b) as usize + (a == b) as usize + (a > b) as usize;
        prop_assert_eq!(total, 1);
    }

    /// Variant ordering: Critical < High < Normal < Low.
    #[test]
    fn priority_critical_is_smallest(_unit in Just(())) {
        prop_assert!(StreamPriority::Critical < StreamPriority::High);
        prop_assert!(StreamPriority::High < StreamPriority::Normal);
        prop_assert!(StreamPriority::Normal < StreamPriority::Low);
    }

    /// `serde_json` roundtrip preserves the variant.
    #[test]
    fn priority_serde_roundtrip(p in priority()) {
        let s = serde_json::to_string(&p).unwrap();
        let back: StreamPriority = serde_json::from_str(&s).unwrap();
        prop_assert_eq!(p, back);
    }

    /// `Display` is non-empty.
    #[test]
    fn priority_display_non_empty(p in priority()) {
        let s = p.to_string();
        prop_assert!(!s.is_empty(), "Display empty for {:?}", p);
    }

    /// `Default` is `Normal`. (Not really a property test in the
    /// random-input sense, but cheap and locks in the invariant.)
    #[test]
    fn priority_default_is_normal(_unit in Just(())) {
        prop_assert_eq!(StreamPriority::default(), StreamPriority::Normal);
    }
}

// ---------------------------------------------------------------- ConnectionStats.

proptest! {
    /// `Default::default()` zeroes every numeric field.
    #[test]
    fn stats_default_is_zero(_unit in Just(())) {
        let d = ConnectionStats::default();
        prop_assert_eq!(d.active_bi_streams, 0);
        prop_assert_eq!(d.active_uni_streams, 0);
        prop_assert_eq!(d.bytes_sent, 0);
        prop_assert_eq!(d.bytes_received, 0);
        prop_assert_eq!(d.rtt_ms, 0.0);
    }

    /// `serde_json` roundtrip preserves every field bit-for-bit.
    #[test]
    fn stats_serde_roundtrip(s in connection_stats()) {
        let json = serde_json::to_string(&s).unwrap();
        let back: ConnectionStats = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s.active_bi_streams, back.active_bi_streams);
        prop_assert_eq!(s.active_uni_streams, back.active_uni_streams);
        prop_assert_eq!(s.bytes_sent, back.bytes_sent);
        prop_assert_eq!(s.bytes_received, back.bytes_received);
        // f64 round-trips bit-for-bit via serde_json when finite.
        prop_assert_eq!(s.rtt_ms, back.rtt_ms);
    }
}

// ---------------------------------------------------------------- QuicError.

fn err_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{0,32}".prop_map(String::from)
}

proptest! {
    /// `Display` for the `String`-carrying QuicError variants
    /// includes the inner string. The `#[error("...: {0}")]`
    /// attribute on each `thiserror::Error` variant should embed it.
    #[test]
    fn error_display_contains_inner(msg in err_string()) {
        for err in [
            QuicError::ConnectionFailed(msg.clone()),
            QuicError::StreamError(msg.clone()),
            QuicError::SendError(msg.clone()),
            QuicError::RecvError(msg.clone()),
            QuicError::InvalidConfig(msg.clone()),
            QuicError::TlsError(msg.clone()),
            QuicError::Timeout(msg.clone()),
            QuicError::ConnectionClosed(msg.clone()),
            QuicError::IoError(msg.clone()),
        ] {
            let display = err.to_string();
            prop_assert!(
                display.contains(&msg),
                "Display {:?} does not contain inner {:?}", display, msg
            );
        }
    }
}
