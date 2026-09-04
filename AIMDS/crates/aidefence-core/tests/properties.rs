//! Property tests: no panics on arbitrary UTF-8, sanitize is idempotent, spans are valid.

use aidefence_core::{decode_and_rescan, detect, normalize, sanitize, Config, Detector};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn sanitize_is_idempotent_and_total(s in "\\PC{0,400}") {
        let once = sanitize(&s);
        prop_assert_eq!(sanitize(&once), once);
    }

    #[test]
    fn sanitize_is_idempotent_on_any_string(s in any::<String>()) {
        let once = sanitize(&s);
        prop_assert_eq!(sanitize(&once), once);
    }

    #[test]
    fn normalize_is_idempotent(s in any::<String>()) {
        let once = normalize(&s);
        prop_assert_eq!(normalize(&once), once);
    }

    #[test]
    fn detect_never_panics_and_spans_are_in_bounds(s in any::<String>()) {
        let d = detect(&s);
        for t in &d.threats {
            prop_assert!(t.span.0 <= t.span.1, "{:?}", t);
            if t.variant == "base" && t.decoded_from.is_none() {
                prop_assert!(t.span.1 <= d.normalized.len(), "{:?}", t);
                prop_assert!(d.normalized.is_char_boundary(t.span.0));
                prop_assert!(d.normalized.is_char_boundary(t.span.1));
            }
            prop_assert!((0.1..=1.0).contains(&t.confidence));
        }
        for p in &d.pii {
            prop_assert!(p.span.0 <= p.span.1 && p.span.1 <= s.len());
        }
        let _ = decode_and_rescan(&s, 4096);
    }

    #[test]
    fn truncation_never_panics(s in any::<String>(), max in 0usize..64) {
        let d = Detector::new(Config::default().with_max_input_len(max)).detect(&s);
        prop_assert_eq!(d.truncated, s.len() > max);
    }

    #[test]
    fn pii_laden_text_sanitizes_idempotently(local in "[a-z]{1,8}", user in "[A-Za-z0-9]{36}") {
        let s = format!(
            "{local}@example.com ssn 123-45-6789 card 4111-1111-1111-1111 ghp_{user} password=hunter2x"
        );
        let once = sanitize(&s);
        prop_assert_eq!(sanitize(&once), once.clone());
        prop_assert!(!aidefence_core::has_pii(&once), "{}", once);
    }
}
