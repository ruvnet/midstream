# midstream Architecture

- **Status:** current as of 2026-05-13
- **Authoritative for:** what `midstream` is, how its pieces fit
  together, and where each component lives.
- **Supersedes:** the pre-2026-05-13 docs archived under
  [`docs/archive/2026-pre-cleanup/`](archive/2026-pre-cleanup/README.md)
  (`ARCHITECTURE_SUMMARY.md`, `DEEP_CODE_ANALYSIS.md`,
  `DEPENDENCY_GRAPH.md`, `GAP_ANALYSIS.md`, `ARCHITECTURE_VALIDATION*.md`,
  `EXECUTIVE_SUMMARY.md`).

This document describes what midstream looks like *today* and the
ADR pointers that explain why. For the historical record see the
archive linked above. For the decision history that drove the
current shape see [docs/adr/](adr/README.md).

---

## 1. What midstream is

midstream is a real-time LLM streaming platform whose distinguishing
trait is **inflight analysis**: tokens are pattern-matched,
metric-tagged, and (with the AIMDS layer) safety-checked *as they
arrive*, not after the response completes.

The platform has three audiences:

1. **Library consumers** who want to embed midstream's primitives —
   `temporal-compare`, `nanosecond-scheduler`, the QUIC multi-stream
   transport — into their own services. These take individual
   `midstreamer-*` crates from crates.io.
2. **Binary consumers** who want a running `midstream` service that
   ingests an LLM stream and emits metrics + dashboard events. These
   take the top-level `midstream` crate.
3. **Browser / Node consumers** who want a subset of the analytical
   primitives in WebAssembly. These take `@midstream/wasm` from npm.

## 2. Repository layout

```
.
├── crates/                              # Published workspace crates
│   ├── temporal-compare/                # midstreamer-temporal-compare
│   ├── nanosecond-scheduler/            # midstreamer-scheduler
│   ├── temporal-attractor-studio/       # midstreamer-attractor
│   ├── temporal-neural-solver/          # midstreamer-neural-solver
│   ├── strange-loop/                    # midstreamer-strange-loop
│   └── quic-multistream/                # midstreamer-quic
├── src/                                 # Top-level `midstream` binary + lib
│   ├── lib.rs
│   ├── midstream.rs                     # streaming pipeline core
│   ├── hypr_service.rs                  # in-memory metric service
│   ├── config.rs                        # MidstreamConfig (figment, pending)
│   ├── bin/main.rs                      # `midstream` binary entry
│   └── lean_agentic/                    # LEGACY — off-by-default (ADR-0005)
├── AIMDS/                               # AI Manipulation Defense System
│   └── crates/                          # 4 aimds-* crates (separate workspace)
├── wasm-bindings/                       # Canonical WASM bindings (ADR-0003)
├── wasm/                                # Legacy WASM crate (ADR-0003 retires)
├── npm-wasm/                            # @midstream/wasm npm package
├── npm/                                 # midstream-cli (TypeScript)
├── lean-agentic-js/                     # @midstream/lean-agentic (TS client)
├── integrations/                        # @midstream/agentic-flow-bridge
├── examples/                            # Rust examples
├── benches/                             # criterion benches
├── tests/                               # integration tests
├── deploy/                              # Docker + Helm (ADR-0035, pending)
└── docs/
    ├── adr/                             # 41 architectural decisions
    ├── ARCHITECTURE.md                  # this file
    └── archive/                         # pre-2026-05-13 doc archive
```

## 3. The Rust layer

### 3.1 Workspace shape

One Cargo workspace at the repo root with six published members.
[AIMDS](#5-aimds-the-defence-layer) currently lives as a sibling
workspace under `AIMDS/`; [ADR-0004](adr/0004-aimds-workspace-member.md)
folds it into the root workspace.

```
midstream (root binary/lib)
└── midstreamer-quic
└── midstreamer-temporal-compare
    └── midstreamer-attractor
    └── midstreamer-neural-solver
        └── midstreamer-scheduler
└── midstreamer-strange-loop
    ├── midstreamer-temporal-compare
    ├── midstreamer-attractor
    ├── midstreamer-neural-solver
    └── midstreamer-scheduler
```

The DAG is acyclic. `strange-loop` is the apex (depends on the four
analytical primitives). `temporal-compare` and `scheduler` are
leaves.

### 3.2 Public-API tiers

Per [ADR-0024](adr/0024-semver-and-api-stability.md):

| Crate                              | Tier  | Note |
|------------------------------------|-------|------|
| `midstreamer-temporal-compare`     | beta  | DTW / LCS / edit-distance primitives |
| `midstreamer-scheduler`            | beta  | priority queue + deadline scheduling |
| `midstreamer-quic`                 | alpha | quinn-backed transport, TLS hardening landed in PR #8 |
| `midstreamer-attractor`            | alpha | Lyapunov / chaos analysis |
| `midstreamer-neural-solver`        | alpha | LTL verification with neural reasoning |
| `midstreamer-strange-loop`         | alpha | meta-learning / self-reference |
| `midstream` (top-level)            | alpha | streaming-pipeline binary + lib |

MSRV across the workspace is **1.81** ([ADR-0023](adr/0023-msrv-policy.md)).

### 3.3 The streaming pipeline (top-level `midstream` crate)

The hot path lives in `src/midstream.rs`:

```
LLMClient -> BoxStream<Bytes>
            -> Midstream::process_stream
               -> for each chunk:
                  -> process_message(content: Bytes)
                     -> intent / urgent detection (UTF-8 lift once)
                     -> tool integration (optional, on urgent intents)
                     -> MetricRecord assembly
                     -> hypr_service.ingest_metric
                     -> messages.push(...)
            -> Result<Vec<LLMMessage>>
```

Key invariants (per [ADR-0006](adr/0006-zero-copy-bytes-streaming.md)):

- Chunks flow through the pipeline as `bytes::Bytes` handles.
  Successive stages `Arc::clone` the buffer instead of copying.
- UTF-8 validation happens once per chunk in `process_message`, not
  per intermediate stage.
- The intent / urgent classifier reads `Cow<'_, str>` — borrowed
  when the input is already valid UTF-8.

Pending hot-path work tracked by ADR:

- [ADR-0007](adr/0007-bounded-backpressure.md) — replace the unbounded
  `Vec<LLMMessage>` accumulator with `tokio::sync::mpsc::channel(N)`.
- [ADR-0008](adr/0008-lock-free-scheduler-cache.md) — rewrite
  `RealtimeScheduler` over `crossbeam_queue::SegQueue` per priority
  level + `AtomicU64` stats.
- [ADR-0010](adr/0010-allocator-observability.md) — `mimalloc` global
  allocator, `tracing-opentelemetry` spans.

### 3.4 Provider abstraction

[ADR-0016](adr/0016-llm-provider-trait-redesign.md) redesigns the
`LLMClient` trait to carry the prompt, model, abort signal, error
type, and tool-call surface as typed events. Until that lands,
`LLMClient` is the minimal `fn stream(&self) -> BoxStream<'static, Bytes>`
from PR #7.

Provider adapters live behind feature flags (per
[ADR-0025](adr/0025-feature-flags-policy.md)):

- `provider-genai` — wraps `genai::Client` for ten providers.
- `provider-openai-realtime` — WebSocket adapter for the OpenAI
  Realtime API.

## 4. The transport layer

`crates/quic-multistream` exposes QUIC bidirectional and
unidirectional streams over `quinn` 0.11 ([ADR-0021](adr/0021-quic-implementation-quinn.md)).
An optional `backend-s2n-quic` feature swaps in AWS s2n-quic.

**TLS verification posture:**

- Default: OS platform trust store via `rustls-platform-verifier`
  (Mac Keychain, Windows cert store, system `ca-certificates` on
  Linux).
- Opt-in: `insecure-dev-only-skip-server-verification` feature for
  self-signed bench/test setups. Emits a `tracing::warn!` on every
  `connect()`. The `SkipServerVerification` type is
  `#[cfg]`-gated out of default builds entirely.

This is the [ADR-0011](adr/0011-quic-tls-verification.md) implementation
shipped in PR #8 as `midstreamer-quic 0.1.1`.

## 5. AIMDS — the defence layer

The AI Manipulation Defense System (`AIMDS/`) sits between the LLM
stream and the rest of midstream. Per [ADR-0013](adr/0013-aimds-integration-contract.md),
the integration contract is:

```rust
pub trait Sanitizer: Send + Sync + 'static {
    async fn scan(&self, chunk: &Bytes, ctx: &StreamContext)
        -> Result<Verdict, SanitizerError>;
}

pub enum Verdict {
    Allow,
    AllowWithWarning { reason: SmolStr },
    Redact { redacted: Bytes, reason: SmolStr },
    Block { reason: SmolStr },
}
```

- **Default**: `aimds_detection::default_sanitizer()` is wired in;
  blocking by default; opt out is explicit (`MidstreamBuilder::without_sanitizer()`).
- **Failure-mode**: `fail closed` by default. `fail open` is opt-in
  and noisy.
- **Workspace shape**: today AIMDS is a sibling workspace at
  `AIMDS/Cargo.toml`. [ADR-0004](adr/0004-aimds-workspace-member.md)
  folds it into the root workspace as four members under
  `AIMDS/crates/`.

## 6. The JavaScript layer

Three published packages, gradually being unified per
[ADR-0026](adr/0026-typescript-monorepo.md):

| Package                       | Today's path        | Purpose |
|-------------------------------|---------------------|---------|
| `midstream-cli`               | `npm/`              | CLI + dashboard |
| `@midstream/wasm`             | `npm-wasm/`         | WebAssembly bindings |
| `@midstream/lean-agentic`     | `lean-agentic-js/`  | TS client (legacy axios HTTP, retiring per ADR-0027) |

After the monorepo conversion they live at `packages/cli/`,
`packages/wasm/`, `packages/lean-agentic/` under one pnpm workspace
with shared tooling.

**Rust ↔ JS boundary** ([ADR-0027](adr/0027-rust-js-boundary.md)):

- **In-process**: WASM. `@midstream/wasm` is the canonical binding.
- **Cross-process**: MCP. A Rust-side
  `crates/midstreamer-mcp-server` (pending) exposes the same tools
  as the existing `npm/src/mcp-server.ts`.

The current HTTP RPC client in `lean-agentic-js` and the bespoke
QUIC framing in `npm/src/quic-integration.ts` both retire under
that ADR.

## 7. The dashboard

Per [ADR-0031](adr/0031-dashboard-architecture.md):

- **Canonical UI**: Rust `ratatui` console TUI (`midstream tui`),
  driven by a `DashboardEvent` stream emitted by the streaming
  pipeline.
- **TS dashboard**: thin MCP subscriber, ~150 LOC; renders the same
  events into HTML / console.
- **NDJSON pipe**: `midstream tui --json` for scripted use.

Multi-modal claims (audio / video / RTMP / WebRTC / HLS) were
trimmed from the README per [ADR-0028](adr/0028-multimodal-scope.md);
the dashboard's state schema reflects only what code backs (text
+ QUIC + WS/SSE).

## 8. Configuration

Today: `config = "0.13"` (pinned in root `Cargo.toml`).

Pending [ADR-0019](adr/0019-config-system.md): replace with `figment`.
The new shape splits one monolithic `HyprSettings` into:

- `MidstreamConfig` — top-level loaded at startup.
- `StreamingConfig` — hot-reloadable.
- `ProvidersConfig` — per-provider API keys (`secrecy::SecretString`,
  redacted in `Debug`).
- `TransportConfig` — QUIC etc.
- `AimdsConfig`.
- `ObservabilityConfig`.

Env var naming uses `MIDSTREAM_STREAMING__MAX_CHUNK_BYTES=…` (`__`
as path separator) to eliminate the current single-`_` ambiguity.

## 9. Observability

Pending [ADR-0010](adr/0010-allocator-observability.md):

- `mimalloc` as the global allocator in the `midstream` binary.
- `tracing-subscriber` + `tracing-opentelemetry` +
  `opentelemetry-otlp` (gRPC) — installed in `main.rs`, configurable
  via `--otlp-endpoint`.
- `console-subscriber` behind a `tokio-console` feature flag for
  runtime task introspection.
- `arrow-flight` was removed (PR #13) because no first-party code
  imported it; it was dragging in dual `rustls`/`tower`/`hyper`
  graphs.

## 10. Build, test, release

### Build / test

```bash
cargo check --workspace --exclude midstream --exclude hyprstream  # default-safe
cargo test  --workspace --exclude midstream --exclude hyprstream --lib
```

`midstream` itself compiles by default after PRs #13 + #14 land
(un-vendor hyprstream + gate legacy `lean_agentic`). The opt-in
`--features lean-agentic` build still has known errors tracked by
the dedup follow-up under [ADR-0005](adr/0005-deduplicate-lean-agentic.md).

### CI

- `.github/workflows/rust-ci.yml` — format / clippy / msrv / test
  matrix (Linux+macOS+Windows × stable+nightly) / build-crates /
  wasm / benchmarks (main only) / docs / security / coverage.
- `.github/workflows/audit.yml` — `cargo-audit` (hard gate) +
  `cargo-deny` (advisories / bans / licenses / sources, currently
  `continue-on-error` while pre-existing skew clears), nightly +
  per-PR (per [ADR-0014](adr/0014-supply-chain-pinning.md)).

### Release

[ADR-0017](adr/0017-release-and-publishing.md) replaces the four
hand-rolled `publish_*.sh` scripts with `release-plz` + `git-cliff`
+ `cargo cyclonedx` (SBOM) + cosign (signed images) + GitHub
attestations.

### Licence

Dual `MIT OR Apache-2.0` across the repo (per
[ADR-0036](adr/0036-license-reconciliation.md)). Contributors sign
the DCO on every commit; no CLA.

## 11. Known broken / out of date

These are documented honestly rather than swept under the rug:

- **`src/lean_agentic/*`** — ~4,573 LOC of legacy code that shadows
  the workspace crates and currently fails to compile. Gated behind
  `--features lean-agentic` so default `midstream` builds clean.
  ADR-0005 dedup is the follow-up.
- **Benchmark numbers in README and archived `docs/BENCHMARK_*.md`** —
  several headline claims ("QUIC throughput >1 GB/s", "schedule
  overhead <100 ns") were measured against in-memory mocks or
  included construction overhead. Real numbers regenerate via the
  criterion harness once [ADR-0009](adr/0009-honest-benchmarks.md)
  lands.
- **Three WASM crates** (`wasm/`, `wasm-bindings/`, `npm-wasm/`) —
  three attempts at the same thing; [ADR-0003](adr/0003-wasm-consolidation.md)
  collapses to one canonical crate.
- **HTTP RPC client in `lean-agentic-js`** — talks to a Rust HTTP
  server that doesn't exist in this repo. ADR-0027 deletes it.

## 12. Where to read next

- **Decision history**: [`docs/adr/README.md`](adr/README.md) — 41
  ADRs grouped by category.
- **Contributing**: [`CONTRIBUTING.md`](../CONTRIBUTING.md).
- **Security disclosure**: [`SECURITY.md`](../SECURITY.md).
- **Governance**: [`GOVERNANCE.md`](../GOVERNANCE.md).
- **Archived pre-cleanup docs**: [`docs/archive/2026-pre-cleanup/`](archive/2026-pre-cleanup/README.md).
