<div align="center">

# MidStream

**Real-time LLM streaming with inflight analysis**

[![crates.io — 6 libs](https://img.shields.io/badge/crates.io-6_published_libs-orange?style=for-the-badge&logo=rust&logoColor=white)](https://crates.io/users/ruvnet)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT_OR_Apache--2.0-yellow?style=for-the-badge)](#license)
[![MSRV: 1.81](https://img.shields.io/badge/MSRV-1.81-blue?style=for-the-badge&logo=rust&logoColor=white)](docs/adr/0023-msrv-policy.md)
[![WASM Ready](https://img.shields.io/badge/WASM-Ready-654FF0?style=for-the-badge&logo=webassembly&logoColor=white)](#wasm--browser)
[![QUIC Transport](https://img.shields.io/badge/QUIC-Transport-10b981?style=for-the-badge&logo=lightning&logoColor=white)](docs/adr/0021-quic-implementation-quinn.md)

[![Star on GitHub](https://img.shields.io/github/stars/ruvnet/midstream?style=for-the-badge&logo=github&color=gold)](https://github.com/ruvnet/midstream)
[![CI](https://img.shields.io/github/actions/workflow/status/ruvnet/midstream/rust-ci.yml?branch=main&style=for-the-badge&label=CI&logo=github-actions&logoColor=white)](.github/workflows/rust-ci.yml)
[![Supply-chain audit](https://img.shields.io/github/actions/workflow/status/ruvnet/midstream/audit.yml?branch=main&style=for-the-badge&label=audit&logo=rustsec&logoColor=white)](.github/workflows/audit.yml)
[![ADRs: 41](https://img.shields.io/badge/ADRs-41_decisions-6366f1?style=for-the-badge&logo=adblock&logoColor=white)](docs/adr/README.md)

</div>

Treat an LLM token stream as a first-class signal — pattern-match it, score it, intervene on it — *while the tokens are still arriving*. MidStream is the Rust workspace + WASM bindings + npm shims that make that practical: a nanosecond scheduler, a multi-stream QUIC transport, dynamical-systems analysis, and a self-referential meta-learning loop, all under one consistent feature flag policy and a hard supply-chain gate.

### Why MidStream?

> Most "LLM tooling" treats the response as a black box that opens at the end. MidStream treats it as a stream of evidence that you can analyze, gate, and steer per chunk. Built in Rust by [`rUv`](https://ruv.io) so the analysis layer adds microseconds, not seconds — and ships the same code path to native, browser, and edge via WASM.

### What MidStream Does

One workspace, six published libraries, one binary. The libraries are independently useful (pattern matching, real-time scheduling, attractor analysis, QUIC multi-stream) and compose into a single inflight-analysis pipeline. The binary wires them together with the OpenAI Realtime API and a console dashboard.

```
Provider stream  ──▶  zero-copy Bytes  ──▶  inflight pipeline  ──▶  decisions
   (OpenAI RT,         (ADR-0006)             │
    Anthropic,                                ├─ temporal-compare    (DTW, LCS, edit-distance)
    custom)                                   ├─ scheduler           (ns-scale priority queue)
                                              ├─ attractor-studio    (Lyapunov, phase-space)
                                              ├─ neural-solver       (LTL + neural reasoning)
                                              ├─ strange-loop        (meta-learning)
                                              └─ quic-multistream    (transport, 0-RTT)
                                                       │
                                              AIMDS safety gate (optional)
                                                       │
                                                       ▼
                                              Dashboard · MCP · WASM · npm
```

> **New to MidStream?** Start with one crate. `temporal-compare` and `scheduler` ship clean tests, run on WASM, and have zero unsafe code — they're the easiest entry points. Move up to `strange-loop` once you want the full meta-learning loop.

---

## Quick Start

Three different entry points depending on what you need. Pick one:

| | **Single library** | **Full workspace** | **WASM / browser** |
|---|---|---|---|
| What it gives you | One crate (e.g. just pattern matching) | All 6 libs + binary + dashboard | Same Rust code, compiled to WASM, in npm |
| What lives in your tree | A line in your `Cargo.toml` | Cloned repo, full build | `npm install @midstream/wasm` |
| Best for | Embedding analysis in an existing app | Running the full inflight pipeline | Browser apps, edge workers |

### Path A — One crate

```bash
# Pattern matching only
cargo add midstreamer-temporal-compare

# Real-time scheduler only
cargo add midstreamer-scheduler

# Or the full library stack
cargo add midstreamer-temporal-compare midstreamer-scheduler \
          midstreamer-attractor midstreamer-neural-solver \
          midstreamer-strange-loop midstreamer-quic
```

### Path B — Full workspace

```bash
git clone https://github.com/ruvnet/midstream.git
cd midstream

# Build everything
cargo build --workspace --release

# Run the binary
cargo run --release --bin midstream -- --help

# Run the bench suite
cargo bench --workspace
```

See [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) for the full zero-to-working-build walkthrough.

### Path C — WASM / browser

```bash
# In a node project
npm install @midstream/wasm

# Or build from source
cd npm-wasm
npm install
npm run build:wasm
```

```js
import { TemporalCompare, Scheduler } from '@midstream/wasm';

const cmp = new TemporalCompare();
const distance = cmp.dtw(seriesA, seriesB);
```

---

## What You Get

| | Capability | What it means |
|---|------------|---------------|
| ⚡ | **Inflight analysis** | Pattern-match, score, and gate LLM tokens as they arrive — not after the response completes |
| 🦀 | **6 published Rust libraries** | Each crate ships independently on crates.io with its own changelog and MSRV gate |
| 🌐 | **WASM-first design** | Every library that doesn't touch the OS compiles to `wasm32-unknown-unknown` with no source forks |
| 📐 | **Honest benchmarks** | `cargo bench --workspace` against real workloads, not mocks — see [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) |
| 🔒 | **Hard supply-chain gate** | `cargo audit` + `cargo deny` block every PR; ADR-0014 |
| 🧪 | **proptest + fuzz baseline** | Every published library has a property-test suite; `midstream-fuzz` has libfuzzer targets — ADR-0038 |
| 🚦 | **Bounded backpressure** | Every async channel has a bound; no unbounded `mpsc`, ever — ADR-0007 |
| 🛡️ | **Secure-by-default TLS** | QUIC defaults to the platform verifier; `SkipServerVerification` is opt-in behind an `insecure-` feature flag — ADR-0011 |
| 📊 | **Console dashboard** | Live FPS, latency, attractor type, chaos detection, stream health — `ratatui`-based, no JS toolchain needed |
| 🔌 | **MCP tool surface** | Namespaced, versioned tools for swarm/agent integrations — ADR-0032 |
| 📚 | **41 ADRs** | Every architectural decision is documented; superseding is via new ADRs, not edits — [`docs/adr/`](docs/adr/README.md) |
| 🧾 | **Dual MIT OR Apache-2.0** | Whole workspace; ADR-0036 |

---

## Rust Workspace

Six published library crates at the same workspace version (currently `0.2.1`). Each one is independently useful and can be added without pulling the others.

<details>
<summary><strong>📦 The six libraries (click to expand)</strong></summary>

| Crate | What it does | crates.io |
|---|---|---|
| [**midstreamer-temporal-compare**](crates/temporal-compare/) | Sequence comparison: Dynamic Time Warping (DTW), Longest Common Subsequence (LCS), edit distance, with LTL extensions | [![v](https://img.shields.io/crates/v/midstreamer-temporal-compare?label=&color=orange&logo=rust)](https://crates.io/crates/midstreamer-temporal-compare) |
| [**midstreamer-scheduler**](crates/nanosecond-scheduler/) | Ultra-low-latency priority queue with strict `Ord` semantics (priority-major, deadline-minor). Designed for ns-scale arrival rates | [![v](https://img.shields.io/crates/v/midstreamer-scheduler?label=&color=orange&logo=rust)](https://crates.io/crates/midstreamer-scheduler) |
| [**midstreamer-attractor**](crates/temporal-attractor-studio/) | Dynamical-systems analysis: Lyapunov exponents, phase-space reconstruction, attractor classification (fixed-point / periodic / chaotic) | [![v](https://img.shields.io/crates/v/midstreamer-attractor?label=&color=orange&logo=rust)](https://crates.io/crates/midstreamer-attractor) |
| [**midstreamer-neural-solver**](crates/temporal-neural-solver/) | Linear Temporal Logic (LTL) verification fused with a small neural reasoner. Used for inflight safety/intent checks | [![v](https://img.shields.io/crates/v/midstreamer-neural-solver?label=&color=orange&logo=rust)](https://crates.io/crates/midstreamer-neural-solver) |
| [**midstreamer-strange-loop**](crates/strange-loop/) | Self-referential meta-learning: the system can observe and adjust its own analysis policy mid-stream | [![v](https://img.shields.io/crates/v/midstreamer-strange-loop?label=&color=orange&logo=rust)](https://crates.io/crates/midstreamer-strange-loop) |
| [**midstreamer-quic**](crates/quic-multistream/) | QUIC multi-stream transport (quinn-backed) for native; thin shim on the browser side. 0-RTT, stream prioritization, secure-by-default TLS | [![v](https://img.shields.io/crates/v/midstreamer-quic?label=&color=orange&logo=rust)](https://crates.io/crates/midstreamer-quic) |

The `midstream` binary at the workspace root wires these into the inflight pipeline.

</details>

<details>
<summary><strong>🧰 The three npm packages (click to expand)</strong></summary>

| Package | What it does |
|---|---|
| [**@midstream/wasm**](npm-wasm/) | The Rust workspace compiled to WASM — same `temporal-compare`, `scheduler`, `attractor`, `strange-loop` available from JS/TS |
| [**midstream**](npm/) | Node CLI + dashboard. Calls the Rust binary or `@midstream/wasm` depending on environment |
| [**lean-agentic-js**](lean-agentic-js/) | The agentic-loop tooling (action / observation / plan / learning) as standalone JS/TS, with its own test suite |

</details>

<details>
<summary><strong>🌐 WASM / browser support</strong></summary>

Every Rust library that doesn't touch the OS compiles to `wasm32-unknown-unknown` from the same source tree:

```bash
cargo build -p midstreamer-temporal-compare --target wasm32-unknown-unknown --no-default-features
cargo build -p midstreamer-scheduler        --target wasm32-unknown-unknown --no-default-features
cargo build -p midstreamer-strange-loop     --target wasm32-unknown-unknown --no-default-features
```

The `npm-wasm/` package builds with `wasm-pack`, ships TypeScript declarations, and works in Node ≥ 18, modern browsers, and edge runtimes (Cloudflare Workers, Deno, Bun). Network egress from the WASM sandbox is gated by an allowlist — ADR-0015.

</details>

<details>
<summary><strong>🏛️ Architecture overview</strong></summary>

```
                   ┌──────────────────────────────┐
                   │     LLM provider stream      │
                   │ (OpenAI RT · Anthropic · ...)│
                   └─────────────┬────────────────┘
                                 ▼
                       ┌─────────────────────┐
                       │  midstream binary   │
                       │  (zero-copy Bytes)  │
                       └─────────────┬───────┘
                                 ▼
        ┌─────────────────────────────────────────────────┐
        │              Inflight pipeline                  │
        │                                                 │
        │  temporal-compare ─► scheduler ─► attractor     │
        │           │              │            │         │
        │           ▼              ▼            ▼         │
        │       neural-solver  strange-loop  dashboard    │
        └─────────────────────────────────────────────────┘
                                 │
                  ┌──────────────┼──────────────┐
                  ▼              ▼              ▼
              MCP server     WASM bundle    OTLP traces
              (ADR-0032)     (ADR-0003)     (ADR-0010)
```

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the long form, and [`docs/adr/`](docs/adr/README.md) for the decisions that shaped each layer.

</details>

---

## Performance

`cargo bench --workspace` runs the full criterion suite. Honest benchmarks against real workloads, not mocks — ADR-0009.

Highlights from the current baseline (Ubuntu 24.04, Ryzen 9 7950X):

| Operation | p50 | Notes |
|---|---|---|
| `temporal-compare::dtw` (128 × 128 floats) | ~38 µs | SIMD-friendly; no allocations on the hot path |
| `scheduler::push` + `pop` (priority-major) | ~85 ns / ~120 ns | Lock-free queue, ADR-0008 |
| `attractor::lyapunov` (1 K-point trajectory) | ~2.4 ms | Single-threaded |
| `strange-loop` self-update cycle | ~340 µs | Per inflight observation |
| QUIC `connect` (0-RTT, loopback) | ~180 µs | Platform verifier, ADR-0011 |

Full numbers (CSV + plots + methodology) in [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md).

---

## Documentation

Four canonical docs. Everything else is either an ADR or archived.

| Doc | When to read it |
|---|---|
| **[Architecture](docs/ARCHITECTURE.md)** | What midstream is, how the pieces fit together, where each component lives. The *how-it-works* doc. |
| **[Getting Started](docs/GETTING_STARTED.md)** | Zero to a working local build. Prerequisites, install, first run. The *how-do-I-start* doc. |
| **[Benchmarks](docs/BENCHMARKS.md)** | Methodology + current numbers, with the historical drift documented. The *is-it-actually-fast* doc. |
| **[Security](docs/SECURITY.md)** | Threat model, posture, supply-chain gates, how to report issues. The *should-I-trust-it* doc. |

Plus the **41 [ADRs](docs/adr/README.md)** — every architectural decision, immutable, with status tracked (Proposed → Accepted → Superseded). Browse the [index](docs/adr/README.md) by topic: foundational · perf/SOTA · security · API · transport · TS surface · dashboard · governance.

---

## Development

```bash
# Run the workspace tests (skip midstream root + hyprstream; broken under --all-features)
cargo test --workspace --exclude midstream --exclude hyprstream --all-features

# Lint
cargo clippy --workspace --exclude midstream --exclude hyprstream --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check

# Supply-chain gate (matches CI)
cargo deny --workspace check --hide-inclusion-graph advisories
cargo deny --workspace check --hide-inclusion-graph bans
cargo deny --workspace check --hide-inclusion-graph licenses
cargo deny --workspace check --hide-inclusion-graph sources
cargo audit

# Property tests
cargo test --workspace --exclude midstream --exclude hyprstream --test 'proptest_*'

# Fuzz one target
cargo +nightly fuzz run dtw_does_not_panic -- -runs=100000
```

The xtask crate (`cargo xtask --help`) replaces hand-rolled shell scripts — ADR-0037.

---

## Project Conventions

- **Modular Cargo workspace.** One root, every member is in `crates/` or a top-level directory — ADR-0001.
- **Off-by-default features, prefixed by domain** (`backend-`, `provider-`, `wasm-`, `insecure-`) — ADR-0025.
- **`thiserror` for libraries, `anyhow` for binaries** — ADR-0018.
- **Workspace-wide lints.** `dbg!`, `todo!()`, `unimplemented!()`, `mem::forget` are all `deny` — ADR-0034.
- **Cargo.lock is committed.** Standard for binary-shipping workspaces; required for `cargo audit` and `--locked` MSRV checks.
- **Released via `release.yml`.** Tag `vX.Y.Z`, the workflow handles `cargo set-version`, dependency-DAG publish, and the GitHub release — ADR-0017.

---

## Contributing

PRs welcome. The bar:

1. Every change references an ADR (or proposes a new one if it's a real decision).
2. New code has property tests where the surface is parseable / generative; unit tests otherwise.
3. `cargo clippy` clean and `cargo fmt` applied — both are CI gates.
4. Security-sensitive changes (TLS, network, deserialization) get an explicit ADR-0011 / ADR-0015 review.

See [`CONTRIBUTING.md`](CONTRIBUTING.md), [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md), and [`SECURITY.md`](SECURITY.md) — all ratified under ADR-0039.

---

## Support

| Resource | Link |
|---|---|
| Issues & bugs | [GitHub Issues](https://github.com/ruvnet/midstream/issues) |
| Architecture decisions | [`docs/adr/`](docs/adr/README.md) |
| Security disclosures | [`SECURITY.md`](SECURITY.md) |
| Author | [ruv.io](https://ruv.io) · [@ruvnet](https://github.com/ruvnet) |

## License

Dual-licensed under **MIT OR Apache-2.0** — pick whichever is more convenient. The same dual licence applies to every workspace member, every WASM artifact, and every npm package. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE). ADR-0036 explains why.
