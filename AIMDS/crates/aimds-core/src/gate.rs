//! `SafetyGate` — composing trait for the AIMDS 3-gate pipeline.
//!
//! This trait formalizes the canonical 3-gate flow that AIMDS
//! consumers expect:
//!
//!   1. Pre-storage PII detection (`aimds-detection`)
//!   2. Sanitization for cookies / tokens / high-entropy blobs
//!      (`aimds-detection` + `aimds-analysis`)
//!   3. Prompt-injection / role-hijack / jailbreak check
//!      (`aimds-detection` + `aimds-response`)
//!
//! Downstream consumers (notably `ruvnet/ruflo`'s
//! `ruflo-federation-peer` — see ADR-120 Step 3) embed this trait
//! to run the 3-gate inspection in-process on every federation
//! message hop. Composing the three crates' surfaces through one
//! trait lets the embedder swap or stub the gate (e.g. for tests)
//! without binding to a concrete pipeline type.
//!
//! `aimds-detection`, `-analysis`, and `-response` (v0.2.0+) provide
//! concrete implementations of this trait via a `ComposedGate` type;
//! this crate exports the trait shape only so `aimds-core` stays
//! dep-light.

use crate::types::{PromptInput, SanitizedOutput};
use crate::AimdsError;
use async_trait::async_trait;

/// The three gates surfaced as a single composing operation.
///
/// Implementors decide whether to short-circuit (the canonical
/// `aimds-response::ComposedGate` does — Block in any gate stops the
/// pipeline) or run all three in parallel for telemetry. Either is
/// valid; the trait contract only specifies the input/output shapes.
#[async_trait]
pub trait SafetyGate: Send + Sync {
    /// Run the 3-gate inspection on a prompt-shaped input. Returns a
    /// [`SafetyVerdict`] describing whether to forward, block, or
    /// forward a redacted variant.
    ///
    /// All three gates inspect the same input. PII detection (gate 1)
    /// and prompt-injection detection (gate 3) are pure functions of
    /// the content; sanitization (gate 2) can mutate the input into
    /// the `Redact` payload.
    async fn inspect(&self, input: &PromptInput) -> Result<SafetyVerdict, AimdsError>;
}

/// Result of running the 3-gate pipeline on one input.
#[derive(Debug, Clone)]
pub enum SafetyVerdict {
    /// All three gates passed. Forward the input as-is.
    Pass,

    /// At least one gate flagged the input as unsafe. The carried
    /// string is a human-readable reason describing which gate fired
    /// and (when known) what pattern triggered.
    Block(String),

    /// PII detection or sanitization rewrote the input. The carried
    /// `SanitizedOutput` holds the cleaned content + a list of
    /// redactions that were applied; forward the sanitized variant.
    Redact(SanitizedOutput),
}

impl SafetyVerdict {
    /// True when the verdict allows the message to be forwarded
    /// (either as-is or after redaction).
    pub fn is_forwardable(&self) -> bool {
        matches!(self, SafetyVerdict::Pass | SafetyVerdict::Redact(_))
    }

    /// True when the verdict requires the message to be quarantined.
    pub fn is_blocked(&self) -> bool {
        matches!(self, SafetyVerdict::Block(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SanitizedOutput;
    use chrono::Utc;
    use uuid::Uuid;

    fn fake_sanitized() -> SanitizedOutput {
        SanitizedOutput {
            original_id: Uuid::nil(),
            timestamp: Utc::now(),
            sanitized_content: String::new(),
            modifications: vec![],
            is_safe: true,
        }
    }

    #[test]
    fn pass_is_forwardable_and_not_blocked() {
        let v = SafetyVerdict::Pass;
        assert!(v.is_forwardable());
        assert!(!v.is_blocked());
    }

    #[test]
    fn redact_is_forwardable_and_not_blocked() {
        let v = SafetyVerdict::Redact(fake_sanitized());
        assert!(v.is_forwardable());
        assert!(!v.is_blocked());
    }

    #[test]
    fn block_is_blocked_and_not_forwardable() {
        let v = SafetyVerdict::Block("test rule".into());
        assert!(!v.is_forwardable());
        assert!(v.is_blocked());
    }

    /// Confirms a downstream `T: SafetyGate` bound accepts a
    /// trivially-implemented gate. Doctest-equivalent.
    #[test]
    fn safety_gate_is_object_safe() {
        fn requires_gate<T: SafetyGate>() {}
        struct AlwaysPass;
        #[async_trait]
        impl SafetyGate for AlwaysPass {
            async fn inspect(
                &self,
                _input: &PromptInput,
            ) -> Result<SafetyVerdict, AimdsError> {
                Ok(SafetyVerdict::Pass)
            }
        }
        requires_gate::<AlwaysPass>();
    }
}
