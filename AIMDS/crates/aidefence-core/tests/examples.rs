//! Every shared pack loads, every pattern compiles (or is an explicit divergence),
//! and every `examples[]` entry triggers its id. Mirrors the "pattern packs" block of
//! `AIMDS/tests/unit/detection.test.ts`.

use aidefence_core::patterns::{self, DivergenceKind};
use aidefence_core::{pii, Detector};

#[test]
fn packs_load_without_errors() {
    assert!(
        patterns::load_errors().is_empty(),
        "pattern load errors: {:#?}",
        patterns::load_errors()
    );
    assert!(pii::load_errors().is_empty(), "{:#?}", pii::load_errors());
    assert_eq!(
        patterns::packs(),
        vec![
            "core",
            "encoded_instruction",
            "exfil_url",
            "instruction_override_i18n",
            "slack_markup_forgery",
            "tool_invocation"
        ]
    );
    assert_eq!(patterns::pattern_count(), 53);
    assert_eq!(pii::pattern_count(), 6);
}

#[test]
fn every_pack_is_enabled_by_default() {
    let d = Detector::default();
    assert_eq!(d.enabled_packs(), patterns::packs());
    assert_eq!(d.pattern_count(), 53);
}

#[test]
fn divergences_from_javascript_are_exactly_the_known_ones() {
    let rewritten: Vec<&str> = patterns::divergences()
        .iter()
        .filter(|d| matches!(d.kind, DivergenceKind::Rewritten { .. }))
        .map(|d| d.id.as_str())
        .collect();
    let skipped: Vec<&str> = patterns::divergences()
        .iter()
        .filter(|d| matches!(d.kind, DivergenceKind::Skipped { .. }))
        .map(|d| d.id.as_str())
        .collect();
    // CORE-005 ends in (?!going|about|ready): moved to a not_followed_by post-check.
    // (Counted repeats of 1024+ would also be relaxed and listed here; the packs no
    // longer carry any.)
    assert_eq!(
        rewritten,
        vec!["CORE-005"],
        "{:#?}",
        patterns::divergences()
    );
    assert!(skipped.is_empty(), "skipped patterns: {skipped:?}");
}

#[test]
fn every_pattern_matches_each_of_its_own_examples() {
    let detector = Detector::default();
    let mut failures = Vec::new();
    for p in patterns::patterns() {
        if p.spec.examples.is_empty() {
            failures.push(format!("{}: no examples", p.spec.id));
        }
        for example in &p.spec.examples {
            let d = detector.detect(example);
            if !d.threats.iter().any(|t| t.id == p.spec.id) {
                failures.push(format!(
                    "{}: example {:?} did not match (hits: {:?})",
                    p.spec.id,
                    example,
                    d.threats.iter().map(|t| &t.id).collect::<Vec<_>>()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_pii_example_matches_its_kind() {
    for p in pii::patterns() {
        assert!(!p.spec.examples.is_empty(), "{} has no examples", p.spec.id);
        for example in &p.spec.examples {
            let hits = pii::scan(example);
            assert!(
                hits.iter().any(|h| h.kind == p.spec.kind),
                "{}: {:?} -> {:?}",
                p.spec.id,
                example,
                hits
            );
        }
    }
}

#[test]
fn portable_patterns_are_lookaround_and_backreference_free() {
    for p in patterns::patterns() {
        if p.spec.engine.as_deref() == Some("js") {
            continue;
        }
        assert!(!p.spec.regex.contains("(?="), "{} has lookahead", p.spec.id);
        assert!(!p.spec.regex.contains("(?!"), "{} has lookahead", p.spec.id);
        assert!(
            !p.spec.regex.contains("(?<"),
            "{} has lookbehind",
            p.spec.id
        );
    }
}
