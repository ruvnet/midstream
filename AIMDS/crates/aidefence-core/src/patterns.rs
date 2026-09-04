//! Pattern packs: the shared JSON schema, JavaScript-to-Rust regex translation,
//! compilation, and the pack registry.
//!
//! The packs are the files in `AIMDS/patterns/*.json`, shared with the TypeScript
//! engine in `AIMDS/src/detection`. They are embedded at build time with
//! `include_str!` and can also be loaded at runtime with [`Registry::from_dir`].

use crate::types::Severity;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Pack id: the 25 patterns of `@claude-flow/aidefence` 3.0.2.
pub const PACK_CORE: &str = "core";
/// Pack id: tool-invocation directives.
pub const PACK_TOOL_INVOCATION: &str = "tool_invocation";
/// Pack id: exfiltration URL shapes.
pub const PACK_EXFIL_URL: &str = "exfil_url";
/// Pack id: encoded / obfuscated instructions.
pub const PACK_ENCODED_INSTRUCTION: &str = "encoded_instruction";
/// Pack id: Slack markup forgery.
pub const PACK_SLACK_MARKUP_FORGERY: &str = "slack_markup_forgery";
/// Pack id: instruction override in other languages.
pub const PACK_INSTRUCTION_OVERRIDE_I18N: &str = "instruction_override_i18n";

/// Pack files embedded from `AIMDS/patterns/` at build time, in file-name order
/// (the TypeScript loader sorts file names the same way).
pub const EMBEDDED_PACKS: &[(&str, &str)] = &[
    ("core.json", include_str!("../../../patterns/core.json")),
    (
        "encoded_instruction.json",
        include_str!("../../../patterns/encoded_instruction.json"),
    ),
    (
        "exfil_url.json",
        include_str!("../../../patterns/exfil_url.json"),
    ),
    (
        "instruction_override_i18n.json",
        include_str!("../../../patterns/instruction_override_i18n.json"),
    ),
    (
        "slack_markup_forgery.json",
        include_str!("../../../patterns/slack_markup_forgery.json"),
    ),
    (
        "tool_invocation.json",
        include_str!("../../../patterns/tool_invocation.json"),
    ),
];

/// Upper bound on the compiled NFA size per pattern (`RegexBuilder::size_limit`).
/// This is the gate every shared pattern must pass; it is the `regex` crate's own
/// default (4 MiB) made explicit so the threshold is a documented choice.
pub const REGEX_SIZE_LIMIT: usize = 4 << 20;
/// Upper bound on the lazy DFA cache per pattern (`RegexBuilder::dfa_size_limit`),
/// the `regex` crate default (2 MiB) made explicit.
pub const DFA_SIZE_LIMIT: usize = 2 << 20;
/// Largest `size_limit` tried when reporting how big an over-limit pattern actually is.
const SIZE_HINT_CEILING: usize = 64 << 20;
/// Bounded repeats with an upper bound at or above this are relaxed to unbounded ones.
/// The bounds exist to keep JavaScript's backtracking engine safe; the `regex` crate is
/// linear regardless, and a counted repeat of 2048 compiles to thousands of NFA states.
const RELAX_REPEAT_AT: u32 = 1024;
/// Base confidence when a pattern declares none (TypeScript: `spec.confidence ?? 0.7`).
const DEFAULT_CONFIDENCE: f64 = 0.7;

/// One pattern as written in the JSON packs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternSpec {
    /// Stable id, e.g. `CORE-001`.
    pub id: String,
    /// Pack the pattern belongs to.
    pub pack: String,
    /// Severity reported for a hit.
    pub severity: Severity,
    /// Threat type reported as [`crate::Threat::kind`].
    #[serde(rename = "type")]
    pub kind: String,
    /// JavaScript-flavoured regex source.
    pub regex: String,
    /// JavaScript flags; `i`, `s`, `m` are honoured, `u` is ignored.
    #[serde(default)]
    pub flags: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Inputs that must trigger this pattern (checked by the test suite).
    #[serde(default)]
    pub examples: Vec<String>,
    /// Base confidence; `0.7` when absent.
    #[serde(default)]
    pub confidence: Option<f64>,
    /// `"portable"` (default) compiles in the `regex` crate; `"js"` may need lookaround.
    #[serde(default)]
    pub engine: Option<String>,
    /// Free-form note from the pack author.
    #[serde(default)]
    pub note: Option<String>,
    /// Rust-side extension: anchored regex tested at the match end; rejects the hit
    /// when it matches. Filled automatically from a trailing `(?!...)`.
    #[serde(default)]
    pub not_followed_by: Option<String>,
    /// Rust-side extension: regex tested against the matched text; rejects the hit
    /// when it matches.
    #[serde(default)]
    pub exclude_match: Option<String>,
}

/// A pack file: `{"pack", "version", "enabledByDefault", "description", "patterns": [...]}`.
/// A bare array is also accepted (enabled by default, pack name taken from the entries).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PackFile {
    /// Wrapped object with pack metadata.
    Wrapped {
        /// Pack id declared by the file.
        pack: String,
        /// Pack version.
        #[serde(default)]
        version: String,
        /// Whether the pack is on unless overridden in `Config::packs`.
        #[serde(default = "default_true", rename = "enabledByDefault")]
        enabled_by_default: bool,
        /// Pack description.
        #[serde(default)]
        description: String,
        /// The patterns.
        patterns: Vec<PatternSpec>,
    },
    /// Bare array.
    List(Vec<PatternSpec>),
}

fn default_true() -> bool {
    true
}

impl PackFile {
    /// Parse a pack file in either shape.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Pack metadata and patterns, regardless of shape.
    pub fn into_parts(self) -> (PackMeta, Vec<PatternSpec>) {
        match self {
            PackFile::Wrapped {
                pack,
                version,
                enabled_by_default,
                description,
                patterns,
            } => (
                PackMeta {
                    name: pack,
                    version,
                    enabled_by_default,
                    description,
                },
                patterns,
            ),
            PackFile::List(patterns) => (
                PackMeta {
                    name: patterns.first().map(|p| p.pack.clone()).unwrap_or_default(),
                    version: String::new(),
                    enabled_by_default: true,
                    description: String::new(),
                },
                patterns,
            ),
        }
    }
}

/// Pack-level metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMeta {
    /// Pack id.
    pub name: String,
    /// Pack version string.
    pub version: String,
    /// Whether the pack is on unless overridden.
    pub enabled_by_default: bool,
    /// Description.
    pub description: String,
}

/// A compiled pattern.
#[derive(Debug)]
pub struct CompiledPattern {
    /// The source spec (with any trailing lookahead already split off).
    pub spec: PatternSpec,
    /// Compiled main regex.
    pub regex: Regex,
    /// Same regex with `\s+` / `\s*` removed, for the whitespace-free text variant.
    /// `None` when the source has no such token or the stripped form does not compile.
    pub compact: Option<Regex>,
    /// Anchored regex rejecting matches by what follows them.
    pub not_followed_by: Option<Regex>,
    /// Regex rejecting matches by their own text.
    pub exclude_match: Option<Regex>,
    /// Effective base confidence.
    pub confidence: f64,
}

/// How a pattern differs from its JavaScript form after loading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DivergenceKind {
    /// The regex was rewritten (e.g. a trailing negative lookahead moved to a post-check).
    Rewritten {
        /// Original source.
        from: String,
        /// Source actually compiled.
        to: String,
    },
    /// The pattern could not be compiled in the `regex` crate and is not matched.
    Skipped {
        /// Compiler error.
        reason: String,
    },
}

/// One recorded divergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Divergence {
    /// Pattern id.
    pub id: String,
    /// What happened.
    pub kind: DivergenceKind,
}

/// A compiled set of packs.
#[derive(Debug)]
pub struct Registry {
    /// Pack metadata in load order.
    pub packs: Vec<PackMeta>,
    /// Compiled patterns in load order.
    pub patterns: Vec<CompiledPattern>,
    /// Patterns that were rewritten or skipped.
    pub divergences: Vec<Divergence>,
    /// Hard errors: invalid JSON, schema violations, or a portable pattern that does not
    /// compile. Empty in a healthy build.
    pub errors: Vec<String>,
}

impl Registry {
    /// The packs embedded at build time, compiled once.
    pub fn embedded() -> &'static Arc<Registry> {
        static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let files = EMBEDDED_PACKS
                .iter()
                .map(|(name, json)| ((*name).to_string(), (*json).to_string()))
                .collect();
            Arc::new(Registry::from_sources(files))
        })
    }

    /// Load every `*.json` pack file in `dir` (sorted by file name), like the TypeScript
    /// `loadPacks`. Fails only when the directory cannot be read; per-file problems are
    /// reported in [`Registry::errors`].
    pub fn from_dir(dir: &Path) -> Result<Registry, String> {
        let mut files = Vec::new();
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            if path.extension().is_some_and(|x| x == "json") {
                let json = std::fs::read_to_string(&path)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                files.push((path.display().to_string(), json));
            }
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Registry::from_sources(files))
    }

    /// Build a registry from already-parsed pack files.
    pub fn from_packs(packs: Vec<PackFile>) -> Registry {
        let parts = packs.into_iter().map(PackFile::into_parts).collect();
        Registry::from_parts(parts, Vec::new())
    }

    /// Build a registry from `(source name, json)` pairs.
    pub fn from_sources(files: Vec<(String, String)>) -> Registry {
        let mut errors = Vec::new();
        let mut parts = Vec::new();
        for (name, json) in files {
            match PackFile::parse(&json) {
                Ok(pack) => parts.push(pack.into_parts()),
                Err(e) => errors.push(format!("{name}: invalid pack JSON: {e}")),
            }
        }
        Registry::from_parts(parts, errors)
    }

    fn from_parts(parts: Vec<(PackMeta, Vec<PatternSpec>)>, mut errors: Vec<String>) -> Registry {
        let mut packs = Vec::new();
        let mut patterns = Vec::new();
        let mut divergences = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for (meta, specs) in parts {
            if packs.iter().any(|p: &PackMeta| p.name == meta.name) {
                errors.push(format!("duplicate pack {}", meta.name));
                continue;
            }
            for spec in specs {
                if spec.pack != meta.name {
                    errors.push(format!(
                        "pattern {}: declares pack {:?} inside pack {:?}",
                        spec.id, spec.pack, meta.name
                    ));
                }
                if !ids.insert(spec.id.clone()) {
                    errors.push(format!("duplicate pattern id {}", spec.id));
                }
                let is_js = spec.engine.as_deref() == Some("js");
                let id = spec.id.clone();
                match compile_spec(spec) {
                    Ok((compiled, divergence)) => {
                        divergences.extend(divergence);
                        patterns.push(compiled);
                    }
                    Err(reason) if is_js => divergences.push(Divergence {
                        id,
                        kind: DivergenceKind::Skipped { reason },
                    }),
                    Err(reason) => errors.push(reason),
                }
            }
            packs.push(meta);
        }
        Registry {
            packs,
            patterns,
            divergences,
            errors,
        }
    }

    /// Pack names in load order.
    pub fn pack_names(&self) -> Vec<&str> {
        self.packs.iter().map(|p| p.name.as_str()).collect()
    }

    /// Metadata for one pack.
    pub fn pack(&self, name: &str) -> Option<&PackMeta> {
        self.packs.iter().find(|p| p.name == name)
    }

    /// Pattern ids in load order.
    pub fn pattern_ids(&self) -> Vec<&str> {
        self.patterns.iter().map(|p| p.spec.id.as_str()).collect()
    }
}

/// Compiled patterns of the embedded registry.
pub fn patterns() -> &'static [CompiledPattern] {
    &Registry::embedded().patterns
}

/// Number of patterns in the embedded registry.
pub fn pattern_count() -> usize {
    Registry::embedded().patterns.len()
}

/// Pack names of the embedded registry.
pub fn packs() -> Vec<&'static str> {
    Registry::embedded().pack_names()
}

/// Hard load errors of the embedded registry (asserted empty by the test suite).
pub fn load_errors() -> &'static [String] {
    &Registry::embedded().errors
}

/// Divergences of the embedded registry from the JavaScript patterns.
pub fn divergences() -> &'static [Divergence] {
    &Registry::embedded().divergences
}

/// Split a trailing JavaScript negative lookahead `(?!alt)` off a regex source.
pub fn split_trailing_negative_lookahead(src: &str) -> Option<(&str, &str)> {
    let end = src.strip_suffix(')')?;
    let bytes = end.as_bytes();
    let mut depth = 0usize;
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        let escaped = i > 0 && bytes[i - 1] == b'\\';
        match bytes[i] {
            b')' if !escaped => depth += 1,
            b'(' if !escaped => {
                if depth == 0 {
                    return end[i..].strip_prefix("(?!").map(|alt| (&src[..i], alt));
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Compact-form source: `\s+` / `\s*` removed so joined words match
/// (mirrors `compactSource` in the TypeScript engine).
pub fn compact_source(src: &str) -> String {
    src.replace("\\s+", "").replace("\\s*", "")
}

/// Relax `{n,m}` repeats whose upper bound is at least [`RELAX_REPEAT_AT`] to `{n,}`.
pub fn relax_large_repeats(src: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\{(\d+),(\d+)\}").expect("static regex"));
    re.replace_all(src, |caps: &regex::Captures<'_>| {
        let upper: u32 = caps[2].parse().unwrap_or(0);
        if upper >= RELAX_REPEAT_AT {
            format!("{{{},}}", &caps[1])
        } else {
            caps[0].to_string()
        }
    })
    .into_owned()
}

/// Compile one spec. Two JavaScript-only shapes are rewritten and recorded as
/// divergences: a trailing negative lookahead `(?!a|b)` moves to `not_followed_by`,
/// and counted repeats with a bound of 1024 or more become unbounded.
pub fn compile_spec(
    mut spec: PatternSpec,
) -> Result<(CompiledPattern, Option<Divergence>), String> {
    let mut divergence = None;
    let relaxed = relax_large_repeats(&spec.regex);
    if relaxed != spec.regex {
        divergence = Some(Divergence {
            id: spec.id.clone(),
            kind: DivergenceKind::Rewritten {
                from: spec.regex.clone(),
                to: relaxed.clone(),
            },
        });
        spec.regex = relaxed;
    }
    if spec.not_followed_by.is_none() {
        if let Some((body, alt)) = split_trailing_negative_lookahead(&spec.regex) {
            divergence = Some(Divergence {
                id: spec.id.clone(),
                kind: DivergenceKind::Rewritten {
                    from: spec.regex.clone(),
                    to: format!("{body} + not_followed_by({alt})"),
                },
            });
            spec.not_followed_by = Some(alt.to_string());
            spec.regex = body.to_string();
        }
    }
    let regex = compile(&translate_js_regex(&spec.regex, &spec.flags))
        .map_err(|e| format!("pattern {}: {e}", spec.id))?;
    let compact_src = compact_source(&spec.regex);
    let compact = if compact_src == spec.regex {
        None
    } else {
        compile(&translate_js_regex(&compact_src, &spec.flags)).ok()
    };
    let not_followed_by = match &spec.not_followed_by {
        Some(src) => Some(
            compile(&format!("^(?:{})", translate_js_regex(src, &spec.flags)))
                .map_err(|e| format!("pattern {} not_followed_by: {e}", spec.id))?,
        ),
        None => None,
    };
    let exclude_match = match &spec.exclude_match {
        Some(src) => Some(
            compile(&translate_js_regex(src, &spec.flags))
                .map_err(|e| format!("pattern {} exclude_match: {e}", spec.id))?,
        ),
        None => None,
    };
    let confidence = spec
        .confidence
        .unwrap_or(DEFAULT_CONFIDENCE)
        .clamp(0.0, 1.0);
    Ok((
        CompiledPattern {
            regex,
            compact,
            not_followed_by,
            exclude_match,
            confidence,
            spec,
        },
        divergence,
    ))
}

fn compile(src: &str) -> Result<Regex, String> {
    RegexBuilder::new(src)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(DFA_SIZE_LIMIT)
        .build()
        .map_err(|e| match e {
            regex::Error::CompiledTooBig(limit) => format!(
                "compiled regex exceeds size_limit {limit} bytes ({})",
                compiled_size_hint(src)
            ),
            other => other.to_string(),
        })
}

/// For a pattern over the size limit, find the smallest power-of-two `size_limit`
/// (up to [`SIZE_HINT_CEILING`]) at which it compiles, so the failure line tells the
/// pack author roughly how large the compiled program is.
pub fn compiled_size_hint(src: &str) -> String {
    let mut limit = REGEX_SIZE_LIMIT;
    while limit < SIZE_HINT_CEILING {
        limit *= 2;
        if RegexBuilder::new(src).size_limit(limit).build().is_ok() {
            return format!(
                "compiles with size_limit {} MiB; shrink counted repeats or classes",
                limit >> 20
            );
        }
    }
    format!("does not compile even at {} MiB", SIZE_HINT_CEILING >> 20)
}

/// Translate a JavaScript regex into `regex`-crate syntax with JavaScript semantics:
///
/// * flags `i`, `s`, `m` become an inline `(?ism)` prefix; `g` and `u` are dropped;
/// * `\d`, `\D`, `\w`, `\W` become ASCII classes (the `regex` crate defaults to Unicode);
/// * `\b`, `\B` become ASCII word boundaries `(?-u:\b)`.
///
/// Lookaround and backreferences are not translated; the `regex` crate rejects them.
pub fn translate_js_regex(src: &str, flags: &str) -> String {
    let mut out = String::with_capacity(src.len() + 16);
    let inline: String = ['i', 's', 'm']
        .iter()
        .filter(|f| flags.contains(**f))
        .collect();
    if !inline.is_empty() {
        out.push_str("(?");
        out.push_str(&inline);
        out.push(')');
    }
    let mut in_class = false;
    let mut chars = src.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('d') => out.push_str(if in_class { "0-9" } else { "[0-9]" }),
                Some('w') => out.push_str(if in_class {
                    "0-9A-Za-z_"
                } else {
                    "[0-9A-Za-z_]"
                }),
                Some('D') if !in_class => out.push_str("[^0-9]"),
                Some('W') if !in_class => out.push_str("[^0-9A-Za-z_]"),
                Some('b') if !in_class => out.push_str(r"(?-u:\b)"),
                Some('B') if !in_class => out.push_str(r"(?-u:\B)"),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            },
            '[' if !in_class => {
                in_class = true;
                out.push(c);
            }
            ']' if in_class => {
                in_class = false;
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(regex: &str, flags: &str) -> PatternSpec {
        PatternSpec {
            id: "t".into(),
            pack: "t".into(),
            severity: Severity::High,
            kind: "t".into(),
            regex: regex.into(),
            flags: flags.into(),
            description: String::new(),
            examples: vec![],
            confidence: None,
            engine: None,
            note: None,
            not_followed_by: None,
            exclude_match: None,
        }
    }

    #[test]
    fn translates_flags_and_ascii_classes() {
        assert_eq!(
            translate_js_regex(r"\bDAN\b.*\d", "gi"),
            r"(?i)(?-u:\b)DAN(?-u:\b).*[0-9]"
        );
        assert_eq!(
            translate_js_regex(r"[\d\w-]+\]", ""),
            r"[0-90-9A-Za-z_-]+\]"
        );
        assert_eq!(translate_js_regex(r"a\\b", ""), r"a\\b");
    }

    #[test]
    fn compact_source_strips_only_whitespace_quantifiers() {
        assert_eq!(compact_source(r"a\s+b\s*c[\s\-_]?d"), r"abc[\s\-_]?d");
    }

    #[test]
    fn trailing_negative_lookahead_becomes_a_post_check() {
        let (compiled, div) =
            compile_spec(spec(r"you\s+are\s+now\s+(?!going|about|ready)", "i")).unwrap();
        assert_eq!(compiled.spec.regex, r"you\s+are\s+now\s+");
        assert!(compiled.not_followed_by.is_some());
        assert!(matches!(
            div.unwrap().kind,
            DivergenceKind::Rewritten { .. }
        ));
        assert_eq!(split_trailing_negative_lookahead(r"a(b)"), None);
        assert_eq!(split_trailing_negative_lookahead(r"a\)"), None);
    }

    #[test]
    fn over_limit_pattern_reports_its_size() {
        // Six 999-wide counted repeats stay below the relaxation threshold but exceed 4 MiB.
        let big = r"[^\s)]{0,999}[^\s)]{0,999}[^\s)]{0,999}[^\s)]{0,999}[^\s)]{0,999}[^\s)]{0,999}";
        let err = compile_spec(spec(big, "i")).expect_err("must exceed the size limit");
        assert!(err.contains("exceeds size_limit 4194304"), "{err}");
        assert!(err.contains("compiles with size_limit"), "{err}");
    }

    #[test]
    fn mid_pattern_lookaround_is_an_error() {
        assert!(compile_spec(spec("a(?!b)c", "")).is_err());
    }

    #[test]
    fn embedded_registry_is_healthy() {
        assert!(load_errors().is_empty(), "{:?}", load_errors());
        assert_eq!(
            packs(),
            vec![
                PACK_CORE,
                PACK_ENCODED_INSTRUCTION,
                PACK_EXFIL_URL,
                PACK_INSTRUCTION_OVERRIDE_I18N,
                PACK_SLACK_MARKUP_FORGERY,
                PACK_TOOL_INVOCATION
            ]
        );
        assert_eq!(pattern_count(), 53);
    }
}
