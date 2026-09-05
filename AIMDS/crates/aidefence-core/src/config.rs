//! Detector configuration (mirrors `InjectionDetectorConfig` + `DecoderOptions`).

use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Default maximum input length in bytes; longer inputs are truncated, never rejected.
pub const DEFAULT_MAX_INPUT_LEN: usize = 100_000;

/// Bounds for the encoded-blob decoders (defaults match the TypeScript engine).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DecoderOptions {
    /// Maximum decoded candidates per `detect` call.
    pub max_candidates: usize,
    /// Maximum characters kept per decoded candidate; the total is `max_bytes * max_candidates`.
    pub max_bytes: usize,
    /// Minimum encoded blob length in characters.
    pub min_blob: usize,
    /// Also rescan the rot13 of the whole (capped) input.
    pub rot13: bool,
    /// Also rescan the character-reversed whole (capped) input.
    pub reverse: bool,
    /// Whole-text variants (rot13 / reverse) are only produced up to this many characters.
    pub whole_text_limit: usize,
}

impl Default for DecoderOptions {
    fn default() -> Self {
        Self {
            max_candidates: 8,
            max_bytes: 4096,
            min_blob: 16,
            rot13: true,
            reverse: true,
            whole_text_limit: 16384,
        }
    }
}

/// Runtime configuration. All fields have defaults; use the builder methods or
/// struct-update syntax.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Inputs longer than this many bytes are truncated at a char boundary before scanning.
    pub max_input_len: usize,
    /// Per-pack enable overrides. Unlisted packs use the pack file's `enabledByDefault`.
    pub packs: BTreeMap<String, bool>,
    /// Scan obfuscation variants (separators, leet, compact) in addition to the base text.
    pub variants: bool,
    /// Decode base64 / hex / url blobs (and rot13 / reverse of the whole text) and rescan
    /// once. `None` means "when the `encoded_instruction` pack is enabled".
    pub decode: Option<bool>,
    /// Decoder bounds.
    pub decoder: DecoderOptions,
    /// Scan for PII and report [`crate::PiiHit`]s.
    pub pii: bool,
    /// A detection is unsafe when any threat has at least this severity.
    /// `Low` mirrors the TypeScript engine (any threat makes the input unsafe).
    pub unsafe_threshold: Severity,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_input_len: DEFAULT_MAX_INPUT_LEN,
            packs: BTreeMap::new(),
            variants: true,
            decode: None,
            decoder: DecoderOptions::default(),
            pii: true,
            unsafe_threshold: Severity::Low,
        }
    }
}

impl Config {
    /// Is the named pack enabled, given the pack file's `enabledByDefault`?
    pub fn pack_enabled(&self, pack: &str, enabled_by_default: bool) -> bool {
        self.packs.get(pack).copied().unwrap_or(enabled_by_default)
    }

    /// Force a pack off (builder style).
    pub fn disable_pack(mut self, pack: impl Into<String>) -> Self {
        self.packs.insert(pack.into(), false);
        self
    }

    /// Force a pack on (builder style).
    pub fn enable_pack(mut self, pack: impl Into<String>) -> Self {
        self.packs.insert(pack.into(), true);
        self
    }

    /// Set the truncation limit (builder style).
    pub fn with_max_input_len(mut self, len: usize) -> Self {
        self.max_input_len = len;
        self
    }

    /// Set the unsafe threshold (builder style).
    pub fn with_unsafe_threshold(mut self, severity: Severity) -> Self {
        self.unsafe_threshold = severity;
        self
    }

    /// Enable or disable PII scanning (builder style).
    pub fn with_pii(mut self, enabled: bool) -> Self {
        self.pii = enabled;
        self
    }

    /// Enable or disable obfuscation variants (builder style).
    pub fn with_variants(mut self, enabled: bool) -> Self {
        self.variants = enabled;
        self
    }

    /// Force decode-and-rescan on or off (builder style).
    pub fn with_decode(mut self, enabled: bool) -> Self {
        self.decode = Some(enabled);
        self
    }

    /// Replace the decoder bounds (builder style).
    pub fn with_decoder(mut self, decoder: DecoderOptions) -> Self {
        self.decoder = decoder;
        self
    }
}
