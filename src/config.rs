//! Configuration loading for midstream.
//!
//! Implementation per [ADR-0019](../../../docs/adr/0019-config-system.md).
//! Layered sources (in order of precedence, last wins):
//!
//!   1. `config/default.toml`  — checked into the repo; safe defaults.
//!   2. `config/local.toml`    — gitignored local override.
//!   3. `MIDSTREAM_*` environment variables — single-`_` path
//!      separator (so `MIDSTREAM_ENGINE_ENGINE` ⇒ `engine.engine`).
//!
//! The historical public surface (`HyprSettings::new`, `Default`
//! impl, the three nested `*Settings` structs) is preserved exactly
//! so this migration is a drop-in replacement for callers.

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;
use std::path::Path;

/// Re-export of the underlying loader-error type so callers don't
/// need a direct `figment` dependency just to spell out the error.
/// The internal swap from `config 0.13` to `figment 0.10` per
/// ADR-0019 changes this from `config::ConfigError` to
/// `figment::Error` — a semver-major break by name, but the only
/// thing callers do with it is `?` it.
pub type ConfigError = figment::Error;

#[derive(Debug, Deserialize)]
pub struct HyprSettings {
    pub engine: EngineSettings,
    pub cache: CacheSettings,
}

#[derive(Debug, Deserialize)]
pub struct EngineSettings {
    pub engine: String,
    pub connection: String,
    pub options: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct CacheSettings {
    pub enabled: bool,
    pub engine: String,
    pub connection: String,
    pub max_duration_secs: u64,
}

impl HyprSettings {
    /// Load configuration from the layered sources described in the
    /// module-level docs.
    ///
    /// Returns `Err(ConfigError)` only on validation failures
    /// (malformed TOML, type mismatch). Missing files are tolerated
    /// — the only mandatory source is the env-var layer (which can
    /// itself be empty).
    #[allow(clippy::result_large_err)]
    pub fn new() -> Result<Self, ConfigError> {
        let config_dir = Path::new("config");

        Figment::new()
            // Defaults: missing file is fine.
            .merge(Toml::file(config_dir.join("default.toml")))
            // Local overrides: also missing-OK.
            .merge(Toml::file(config_dir.join("local.toml")))
            // Env vars with `MIDSTREAM_` prefix and `_` path separator.
            // `MIDSTREAM_ENGINE_ENGINE=foo` ⇒ `engine.engine = "foo"`.
            .merge(Env::prefixed("MIDSTREAM_").split("_"))
            .extract()
    }
}

impl Default for HyprSettings {
    fn default() -> Self {
        Self {
            engine: EngineSettings {
                engine: "duckdb".to_string(),
                connection: ":memory:".to_string(),
                options: std::collections::HashMap::new(),
            },
            cache: CacheSettings {
                enabled: true,
                engine: "duckdb".to_string(),
                connection: ":memory:".to_string(),
                max_duration_secs: 3600,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            // SAFETY: this is a test helper that mutates process env;
            // the `Once` guard prevents concurrent invocation.
            unsafe {
                std::env::set_var("MIDSTREAM_ENGINE_ENGINE", "test_engine");
            }
        });
    }

    #[test]
    fn test_default_settings() {
        let settings = HyprSettings::default();
        assert_eq!(settings.engine.engine, "duckdb");
        assert!(settings.cache.enabled);
    }

    #[test]
    fn test_environment_override() {
        setup();
        let settings = HyprSettings::new().unwrap();
        assert_eq!(settings.engine.engine, "test_engine");
    }
}
