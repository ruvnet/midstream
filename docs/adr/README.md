# Architecture Decision Records

This directory holds the Architecture Decision Records (ADRs) for the
**midstream** repository. ADRs capture *why* an architectural decision was
made — not the implementation, which lives in code and lib docs.

We use the [MADR 4.0](https://adr.github.io/madr/) format. Each ADR is a
short, dated, immutable document. To change a decision you write a new ADR
that *supersedes* the old one (the old file stays, with its Status updated).

## Index

### Foundational (workspace, deduplication)

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0000 | [Use ADRs to record architectural decisions](0000-use-adrs.md)     | Accepted |
| 0001 | [Single Cargo workspace for all Rust crates](0001-single-cargo-workspace.md) | Proposed |
| 0002 | [Stop vendoring hyprstream, depend on it externally](0002-unvendor-hyprstream.md) | Accepted (#13) |
| 0003 | [One canonical WASM crate, three publish targets](0003-wasm-consolidation.md) | Proposed |
| 0004 | [AIMDS as a first-class workspace member](0004-aimds-workspace-member.md) | Proposed |
| 0005 | [Remove src/lean_agentic duplication of workspace crates](0005-deduplicate-lean-agentic.md) | Accepted (#14) |

### Performance and SOTA

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0006 | [Zero-copy `bytes::Bytes` end-to-end streaming pipeline](0006-zero-copy-bytes-streaming.md) | Accepted (#7) |
| 0007 | [Bounded backpressure: mpsc + Semaphore, never unbounded](0007-bounded-backpressure.md) | Proposed |
| 0008 | [Lock-free scheduler and cache rewrite](0008-lock-free-scheduler-cache.md) | Proposed |
| 0009 | [Benchmarks against real workloads, not mocks](0009-honest-benchmarks.md) | Proposed |
| 0010 | [Allocator and observability baseline (mimalloc + OTLP)](0010-allocator-observability.md) | Proposed (partial: arrow drop only) |

### Security and supply chain

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0011 | [QUIC TLS verification: secure-by-default](0011-quic-tls-verification.md) | Accepted (#8) |
| 0012 | [Streaming input bounds (size, rate, buffer)](0012-streaming-input-bounds.md) | Proposed |
| 0013 | [AIMDS integration contract](0013-aimds-integration-contract.md)   | Proposed |
| 0014 | [Supply-chain hygiene: cargo audit + cargo deny in CI](0014-supply-chain-pinning.md) | Accepted (#10, #55) |
| 0015 | [WASM network egress allowlist](0015-wasm-egress-allowlist.md)     | Proposed |

### API, release, configuration, hygiene

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0016 | [Redesign the `LLMClient` provider trait](0016-llm-provider-trait-redesign.md) | Proposed |
| 0017 | [Release model: release-plz + git-cliff, drop hand-written scripts](0017-release-and-publishing.md) | Accepted (#51) |
| 0018 | [Error-handling policy: `thiserror` for libs, `anyhow` for binaries](0018-error-policy.md) | Proposed |
| 0019 | [Configuration system: replace `config` with `figment`](0019-config-system.md) | Accepted (#52) |
| 0020 | [Documentation triage: retire the `*_REPORT.md` sprawl](0020-docs-triage.md) | Accepted (#17, #18, #19, #20, #50) |

### Transport, persistence, policy

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0021 | [QUIC implementation: quinn default + optional s2n-quic](0021-quic-implementation-quinn.md) | Proposed |
| 0022 | [Persistence: delete ruvector.db; redb when needed](0022-persistence-layer.md) | Proposed |
| 0023 | [MSRV policy: declared, N-3, CI-enforced](0023-msrv-policy.md)     | Accepted (#11) |
| 0024 | [Semver discipline + stability tiers (alpha/beta/stable)](0024-semver-and-api-stability.md) | Proposed |
| 0025 | [Feature-flag policy: additive, off-by-default, prefixed](0025-feature-flags-policy.md) | Proposed (partial: #16 — npm-wasm only) |

### TypeScript / JS surface

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0026 | [TypeScript monorepo via pnpm workspaces](0026-typescript-monorepo.md) | Proposed |
| 0027 | [Rust ↔ JS boundary: WASM in-process, MCP cross-process](0027-rust-js-boundary.md) | Proposed |
| 0028 | [Multi-modal scope: trim README to what code backs](0028-multimodal-scope.md) | Proposed |
| 0029 | [JS/TS CI matrix: lint, typecheck, test on every PR](0029-js-ci-matrix.md) | Accepted (#56) |
| 0030 | [`integrations/` is a package, not a stray file](0030-integrations-directory.md) | Proposed |

### Dashboard, MCP surface, SLO, lints, deployment

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0031 | [Console dashboard: ratatui, split rendering from state](0031-dashboard-architecture.md) | Proposed |
| 0032 | [MCP tool surface: namespaced, versioned, lifecycle-split](0032-mcp-tool-surface.md) | Proposed |
| 0033 | [Real-time scheduler SLO contract](0033-scheduler-slo-contract.md) | Proposed |
| 0034 | [Workspace-wide lint policy via `[workspace.lints]`](0034-workspace-lints.md) | Accepted (#15, #49) |
| 0035 | [Deployment shape: one Dockerfile + Helm chart](0035-deployment-docker-k8s.md) | Proposed |

### Licence, automation, testing, governance, deferred

| #    | Title                                                              | Status   |
|------|--------------------------------------------------------------------|----------|
| 0036 | [Licence reconciliation: dual MIT OR Apache-2.0](0036-license-reconciliation.md) | Accepted (#9) |
| 0037 | [`xtask` build automation: replace shell scripts with Rust](0037-xtask-build-automation.md) | Accepted (#57) |
| 0038 | [Fuzz + property tests for the parsing/streaming surface](0038-fuzz-and-property-tests.md) | Accepted (#58–#67) |
| 0039 | [Governance: CONTRIBUTING, CoC, SECURITY, CODEOWNERS](0039-governance.md) | Accepted (#12) |
| 0040 | [Web dashboard: deferred behind a stable event stream](0040-web-dashboard-deferred.md) | Proposed (deferred) |

ADRs 0041+ will be added as further iterations of `/loop deep review and
optimization of midstream` surface additional concrete decisions
(`cargo-flamegraph` profiling baseline, `tokio-console` policy, telemetry
PII redaction, prompt-injection regression suite, Lean theorem-prover
sidecar, the Rust ↔ Python interop story, etc.).

## Process

1. Copy [`TEMPLATE.md`](TEMPLATE.md) to `NNNN-kebab-case-title.md` (next
   free number, zero-padded to 4 digits).
2. Fill in **Context**, **Decision Drivers**, **Considered Options**, the
   chosen **Decision Outcome**, and **Consequences** (both positive and
   negative — both are mandatory).
3. Open a PR. The ADR ships in the same PR as (or just before) the code
   that implements it.
4. Once merged, the Status moves from `Proposed` → `Accepted`. Update the
   index table above.
5. To change a decision later: write a new ADR, set its `Supersedes` link
   to the old ADR, and change the old ADR's Status to `Superseded by
   NNNN-…`. **Never rewrite history in an Accepted ADR.**

## Why MADR

- Explicit "Decision Drivers" and "Considered Options" sections force you
  to name what you *didn't* pick, which is where most of the value is.
- Concise (target: 1–2 screens). ADRs that read like essays don't get
  read.
- Tooling-friendly (the [adr-tools](https://github.com/npryce/adr-tools)
  CLI and the IDE plugins all understand the format).

## Related conventions

- ADRs are scoped to **architecture** — module boundaries, dependency
  decisions, transport choices, build/release model, deployment shape.
  Implementation choices that only affect one module (which trait to
  pick, which algorithm) belong in code comments or design docs under
  `docs/`, not here.
- One decision per ADR. Bundle-ADRs become unmaintainable.
- Date ADRs with the merge date, not the draft date.
