# 0019 — Configuration system: replace `config = "0.13"` with `figment`

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** config, dependencies, supply-chain

## Context and Problem Statement

Configuration is wired through the `config` crate (`config = "0.13"`,
root `Cargo.toml:28`), used at `src/config.rs:1-39`:

```rust
let builder = Config::builder()
    .add_source(File::from(config_dir.join("default.toml")).required(false))
    .add_source(File::from(config_dir.join("local.toml")).required(false))
    .add_source(Environment::with_prefix("MIDSTREAM").separator("_"));
```

Three problems:

1. **`config = "0.13"` is 18 months behind.** Current is `0.15`. The
   `0.13` line transitively pulls in `yaml-rust = "0.4.5"`, which is
   **unmaintained** per `cargo audit`. `0.15` migrated to `yaml-rust2`.
2. **`Environment::with_prefix("MIDSTREAM").separator("_")`** is
   ambiguous: `MIDSTREAM_ENGINE_CONNECTION` could mean
   `engine.connection` or `engine_connection`. There is a real failing
   test fixture for this (`src/config.rs:81` uses
   `MIDSTREAM_ENGINE_ENGINE`).
3. **No schema validation.** Garbage `config/default.toml` produces a
   `Deserialize` error deep in the type tree; no surfacing of "you
   meant `engine.engine` not `enginee.engine`".
4. **Single global struct (`HyprSettings`)** mixes streaming knobs
   (which want hot-reload), provider keys (which want strong typing +
   redaction), and DB settings (which want startup-only). Different
   change-rates, same struct.

The new ADRs [ADR-0007](0007-bounded-backpressure.md),
[ADR-0011](0011-quic-tls-verification.md),
[ADR-0012](0012-streaming-input-bounds.md), and
[ADR-0015](0015-wasm-egress-allowlist.md) all want configurable knobs.
Piling them onto `HyprSettings` makes the existing problems worse.

## Decision Drivers

- **Drop the `yaml-rust` advisory.** Either bump `config` or replace
  it.
- **Schema-validated, layered config.** Sources in known order
  (defaults < file < env < CLI), with clear error messages.
- **Type discipline.** Provider API keys must be a `Secret<String>`
  (redacted on `Debug`), URLs must be `url::Url`, durations must be
  `humantime`-parsed, etc.
- **One config domain per concern.** Streaming, transport, providers,
  AIMDS, and observability each get their own typed struct;
  composition lives at the binary.

## Considered Options

1. **Bump `config` to `0.15`.** Cheapest; fixes the `yaml-rust`
   advisory. Doesn't fix the env-var-separator ambiguity or schema
   validation gap.
2. **Replace with `figment`.** Layered providers (`Toml`, `Env`,
   `Json`, `Yaml`, `Serialized`), strict mode, profile-aware (dev /
   prod / test). Used by Rocket; widely adopted.
3. **Replace with `confique`.** Code-first config with derived
   documentation generation. Smaller community, very ergonomic for
   small projects.
4. **Hand-roll TOML+env merging.** No new dep. Loses every layering
   feature on offer.

## Decision Outcome

**Chosen option: Option 2 — `figment`.**

Concretely:

- `figment` with `Toml`, `Json` (for k8s ConfigMap mounts), and
  `Env::prefixed("MIDSTREAM_")` providers, joined with
  `figment::Profile::Default`.
- Top-level config struct splits into:
  - `MidstreamConfig` (loaded once at startup),
  - `StreamingConfig` (hot-reloadable; cf. [ADR-0012](0012-streaming-input-bounds.md)),
  - `ProvidersConfig` (per-provider keyed by name; uses
    `secrecy::SecretString` for tokens),
  - `TransportConfig` (QUIC, HTTP, etc.; cf. [ADR-0011](0011-quic-tls-verification.md)),
  - `AimdsConfig` (cf. [ADR-0013](0013-aimds-integration-contract.md)),
  - `ObservabilityConfig` (OTLP endpoint, log filters; cf.
    [ADR-0010](0010-allocator-observability.md)).
- Env-var naming uses `__` as the path separator (so
  `MIDSTREAM_STREAMING__MAX_CHUNK_BYTES=65536` unambiguously sets
  `streaming.max_chunk_bytes`). The previous single-`_` separator
  ambiguity goes away.
- All API keys are `secrecy::SecretString`; `Debug` redacts them.
- `MidstreamConfig::load()` returns a `Result<Self, ConfigError>`
  with a top-level `Display` impl that includes the *source* of the
  offending value (file path + line, or the env var name).
- Profiles: `default`, `dev`, `prod`, `test`. `MIDSTREAM_PROFILE=dev`
  selects.

### Positive consequences

- `yaml-rust` advisory closes (figment 0.10 uses serde-native parsers).
- Env-var ambiguity resolved (`__` separator).
- Secrets visible in Debug output stop being a footgun
  (`secrecy::SecretString`).
- Each domain owns its struct; per-domain validation lives next to the
  fields.

### Negative consequences

- One-time migration of `config/default.toml`, plus any deployments
  that set `MIDSTREAM_*` env vars. Mitigation: ship a translation
  doc and emit a `tracing::warn!` for any single-`_` env var that
  looks like an old-style key.
- `figment` is one more dep, and `secrecy` adds a small one. Trivial.
- `figment`'s error messages are good but not great; we add our own
  `Display` wrapper.

## Implementation notes

- Replace `config = "0.13"` with `figment = { version = "0.10",
  features = ["toml", "json", "env"] }` and `secrecy = "0.8"` in root
  `[workspace.dependencies]`.
- Rewrite `src/config.rs` to expose `MidstreamConfig` plus the five
  sub-structs above. Each sub-struct gets its own `Default` impl and
  `validate()` method.
- `src/hypr_service.rs::HyprServiceImpl::new` becomes `(&HyprConfig)`,
  not `(&HyprSettings)`. Old names re-exported with `#[deprecated]`.
- Add `tests/config_loading_tests.rs` covering: default-only,
  file-override, env-override, secret-redaction, bad-path error.

## Links

- Related: [ADR-0007](0007-bounded-backpressure.md),
  [ADR-0010](0010-allocator-observability.md),
  [ADR-0011](0011-quic-tls-verification.md),
  [ADR-0012](0012-streaming-input-bounds.md),
  [ADR-0013](0013-aimds-integration-contract.md),
  [ADR-0014](0014-supply-chain-pinning.md).
- `figment`: https://docs.rs/figment/
- `secrecy`: https://docs.rs/secrecy/
