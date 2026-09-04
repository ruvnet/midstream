//! Public result types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Threat severity, ordered so that `Critical > High > Medium > Low`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational; often legitimate framing.
    Low,
    /// Suspicious; review recommended.
    Medium,
    /// Likely attack.
    High,
    /// Definite attack indicator.
    Critical,
}

impl Severity {
    /// Lower-case name matching the TypeScript `Severity` union.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" => Ok(Severity::Low),
            "medium" => Ok(Severity::Medium),
            "high" => Ok(Severity::High),
            "critical" => Ok(Severity::Critical),
            other => Err(format!("unknown severity: {other}")),
        }
    }
}

/// One matched threat pattern (mirrors `InjectionThreat` in the TypeScript engine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Threat {
    /// Pattern id from the pack, e.g. `CORE-001`.
    pub id: String,
    /// Pack the pattern belongs to, e.g. `core`.
    pub pack: String,
    /// Threat type (`ThreatKind` in the TypeScript engine).
    pub kind: String,
    /// Severity declared by the pattern.
    pub severity: Severity,
    /// Confidence in `[0.1, 1]`: base confidence minus the variant penalty
    /// (and 0.05 when found in decoded content), rounded to two decimals.
    pub confidence: f64,
    /// Byte span `(start, end)` of the match inside the text variant that matched.
    /// For decoded hits `start` is the byte offset of the encoded blob in the
    /// normalized text (whole-text rot13/reverse candidates report 0).
    pub span: (usize, usize),
    /// Matched text, truncated to 160 characters.
    pub matched: String,
    /// Which text variant matched: `base`, `separators`, `leet` or `compact`.
    pub variant: String,
    /// Set when the match was found in decoded content: `base64`, `hex`, `url`,
    /// `rot13` or `reverse`.
    pub decoded_from: Option<String>,
    /// Human-readable description from the pack.
    pub description: String,
}

/// One PII match found in the input text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiiHit {
    /// PII kind: `email`, `ssn`, `credit_card`, `api_key`, `password`.
    pub kind: String,
    /// Byte span `(start, end)` inside the input text (not the normalized text).
    pub span: (usize, usize),
    /// Replacement that [`crate::sanitize`] would insert for this match.
    pub masked: String,
}

/// Full detection result (mirrors `InjectionReport` plus PII).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    /// `true` when no threat reached the configured unsafe threshold (default: any
    /// threat makes the input unsafe, as in the TypeScript engine). PII does not affect it.
    pub safe: bool,
    /// One threat per pattern id (highest confidence), sorted by severity then confidence.
    pub threats: Vec<Threat>,
    /// PII hits (empty when PII scanning is disabled).
    pub pii: Vec<PiiHit>,
    /// The normalized text (base variant).
    pub normalized: String,
    /// Number of text variants scanned.
    pub scanned_variants: usize,
    /// Number of decoded candidates rescanned.
    pub decoded_candidates: usize,
    /// Number of compiled patterns in the enabled packs.
    pub pattern_count: usize,
    /// Wall-clock time spent in `detect`, in microseconds (always 0 on wasm32).
    pub elapsed_us: u64,
    /// `true` when the input exceeded `max_input_len` and was truncated before scanning.
    pub truncated: bool,
}

impl Detection {
    /// Highest severity among the detected threats, if any.
    pub fn max_severity(&self) -> Option<Severity> {
        self.threats.iter().map(|t| t.severity).max()
    }
}
