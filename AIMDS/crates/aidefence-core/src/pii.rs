//! PII detection and masking, ported from the `PII_PATTERNS` table in
//! `@claude-flow/aidefence`. Pattern data lives in `patterns/pii.json`.

use crate::patterns::translate_js_regex;
use crate::types::PiiHit;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const SOURCE: &str = include_str!("../patterns/pii.json");

/// Mask value meaning "keep the first character of the local part".
const EMAIL_MASK: &str = "@email";

/// One PII pattern as written in `pii.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PiiSpec {
    /// Stable id.
    pub id: String,
    /// PII kind reported as [`PiiHit::kind`].
    pub kind: String,
    /// JavaScript-flavoured regex source.
    pub regex: String,
    /// JavaScript flags (`i` honoured).
    #[serde(default)]
    pub flags: String,
    /// Literal replacement, or `@email` for the email-specific mask.
    pub mask: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Inputs that must trigger this pattern (checked by the test suite).
    #[serde(default)]
    pub examples: Vec<String>,
}

/// A compiled PII pattern.
#[derive(Debug)]
pub struct CompiledPii {
    /// The source spec.
    pub spec: PiiSpec,
    /// Compiled regex.
    pub regex: Regex,
}

struct PiiRegistry {
    patterns: Vec<CompiledPii>,
    errors: Vec<String>,
}

fn registry() -> &'static PiiRegistry {
    static REGISTRY: OnceLock<PiiRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut patterns = Vec::new();
        let mut errors = Vec::new();
        match serde_json::from_str::<Vec<PiiSpec>>(SOURCE) {
            Ok(specs) => {
                for spec in specs {
                    match Regex::new(&translate_js_regex(&spec.regex, &spec.flags)) {
                        Ok(regex) => patterns.push(CompiledPii { spec, regex }),
                        Err(e) => errors.push(format!("pii {}: {e}", spec.id)),
                    }
                }
            }
            Err(e) => errors.push(format!("pii.json: {e}")),
        }
        PiiRegistry { patterns, errors }
    })
}

/// Compiled PII patterns in load order.
pub fn patterns() -> &'static [CompiledPii] {
    &registry().patterns
}

/// Load / compile errors for the PII pack (empty in a healthy build).
pub fn load_errors() -> &'static [String] {
    &registry().errors
}

/// Number of PII patterns bundled.
pub fn pattern_count() -> usize {
    registry().patterns.len()
}

fn mask_email(email: &str) -> String {
    match email.find('@') {
        Some(at) => {
            let local = &email[..at];
            let domain = &email[at..];
            match local.chars().next() {
                Some(first) => format!("{first}***{domain}"),
                None => format!("***{domain}"),
            }
        }
        None => "***@***.***".to_string(),
    }
}

fn mask_for(spec: &PiiSpec, matched: &str) -> String {
    if spec.mask == EMAIL_MASK {
        mask_email(matched)
    } else {
        spec.mask.clone()
    }
}

/// Find every PII match in `text`. Spans are byte offsets into `text`.
pub fn scan(text: &str) -> Vec<PiiHit> {
    let mut hits = Vec::new();
    for p in patterns() {
        for m in p.regex.find_iter(text) {
            hits.push(PiiHit {
                kind: p.spec.kind.clone(),
                span: (m.start(), m.end()),
                masked: mask_for(&p.spec, m.as_str()),
            });
        }
    }
    hits.sort_by_key(|h| h.span);
    hits
}

/// `true` when any PII pattern matches (mirrors `ThreatDetectionService.detectPII`).
pub fn has_pii(text: &str) -> bool {
    patterns().iter().any(|p| p.regex.is_match(text))
}

/// Replace every PII match with its mask, pattern by pattern.
pub fn mask_all(text: &str) -> String {
    let mut out = text.to_string();
    for p in patterns() {
        if p.regex.is_match(&out) {
            out = p
                .regex
                .replace_all(&out, |caps: &regex::Captures<'_>| {
                    mask_for(&p.spec, &caps[0])
                })
                .into_owned();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads() {
        assert!(load_errors().is_empty(), "{:?}", load_errors());
        assert_eq!(pattern_count(), 6);
    }

    #[test]
    fn masks_are_stable_under_rescan() {
        let text = "mail jane.doe@example.com ssn 123-45-6789 card 4111 1111 1111 1111 \
                    key sk-abcdefghijklmnopqrstuvwxyz123456 password: hunter2secret";
        let once = mask_all(text);
        assert!(!has_pii(&once), "{once}");
        assert_eq!(mask_all(&once), once);
        assert!(once.contains("j***@example.com"));
        assert!(once.contains("password: ***"));
    }

    #[test]
    fn scan_reports_spans_in_input() {
        let hits = scan("x 123-45-6789 y");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "ssn");
        assert_eq!(hits[0].span, (2, 13));
    }
}
