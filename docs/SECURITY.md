# midstream Security Posture

- **Status:** current as of 2026-05-13
- **Audience:** integrators, operators, and security reviewers deciding
  whether midstream is safe to deploy in their environment.
- **Companion doc**: [`../SECURITY.md`](../SECURITY.md) at the repo
  root is the **reporting policy** (where to file a vulnerability,
  SLAs, disclosure timeline). *This* document is the **posture
  description** (what midstream defends against, what it explicitly
  does not).
- **Supersedes**: the pre-2026-05-13 reports archived under
  [`archive/2026-pre-cleanup/`](archive/2026-pre-cleanup/)
  (`SECURITY_ANALYSIS_REPORT.md`, `SECURITY_AUDIT_SUMMARY.md`,
  `SECURITY_VULNERABILITY_REPORT.md`). Those documents made claims
  ("10/10 security checks passed") that did not survive the
  PR #6 fresh-eyes review — see "Posture, honestly" below.

This document is intentionally honest about what is and is not
defended. If you find an inaccuracy, file an advisory per the
reporting policy.

---

## 1. Threat model

midstream sits between an LLM (treated as **partially-trusted at
best, adversarial at worst**) and downstream consumers. The threat
sources we model:

| Source | Examples | What we defend with |
|---|---|---|
| **Hostile LLM output** | Prompt injection in the model's tokens, attempts to exfiltrate caller state, instructions that try to subvert tool calls | The AIMDS layer (`Sanitizer` trait per [ADR-0013](adr/0013-aimds-integration-contract.md)) — fail-closed by default |
| **Malicious network input** | Oversized chunks, slow-loris streams, malformed SSE / WebSocket frames | Streaming input bounds ([ADR-0012](adr/0012-streaming-input-bounds.md)): per-chunk size cap, per-stream lifetime cap, rate limit, timeout. Backpressure ([ADR-0007](adr/0007-bounded-backpressure.md)) propagates fullness back to producers. |
| **MITM on QUIC transport** | Attacker substitutes a TLS certificate | Default verifier is the OS platform trust store (`rustls-platform-verifier`); skip-verify is a feature flag named verbosely (`insecure-dev-only-skip-server-verification`) and emits a runtime warning on every `connect()` ([ADR-0011](adr/0011-quic-tls-verification.md)) |
| **Supply-chain attack** | Compromised transitive crate, advisory published against a pinned version | `cargo-audit` as a hard CI gate; `cargo-deny` policy in `deny.toml`; `Cargo.lock` committed; Dependabot weekly ([ADR-0014](adr/0014-supply-chain-pinning.md)) |
| **WASM-side proxy abuse** | A page that loads `@midstream/wasm` is tricked into proxying arbitrary `fetch` / WebSocket / EventSource calls | Scheme + host allowlist on every WASM-side URL constructor; per-stream byte cap; abort signal; timeout ([ADR-0015](adr/0015-wasm-egress-allowlist.md)) |
| **Memory-safety bugs in our code** | Use-after-free, OOB, data race | Zero `unsafe ` blocks workspace-wide (verified by security review, enforced by `[workspace.lints] rust.unsafe_code = "deny"` per [ADR-0034](adr/0034-workspace-lints.md)); panic-free hot path enforced by clippy lints |
| **Bot or attacker filing a public security issue** | Accidental public disclosure of an unpatched bug | `.github/ISSUE_TEMPLATE/config.yml` blocks blank issues and routes security reports to GitHub Security Advisories ([ADR-0039](adr/0039-governance.md)) |

## 2. What we do NOT defend against

State explicitly so integrators can layer their own controls:

- **Compromise of the host process.** If an attacker has process-
  level code execution, midstream's in-memory metric store and
  any in-flight tokens are theirs. Standard OS sandboxing
  (containers, seccomp, AppArmor / SELinux) is your job.
- **Compromise of the configuration file or env vars.** If the
  attacker can write your config / `.env`, they can replace
  provider URLs and API keys. Secrets are loaded via
  `secrecy::SecretString` (redacted on `Debug`) but the loader
  trusts whatever the OS hands it.
- **Compromise of the upstream LLM provider itself.** If OpenAI /
  Anthropic / Gemini themselves are compromised, the sanitizer
  catches what it can but cannot detect a "perfectly legitimate"
  response that's been tampered with at the provider.
- **Side-channel attacks.** Timing of pattern-detection in
  `temporal-compare`, cache hits in the LRU caches, allocator
  behaviour — none of these are constant-time. Do not embed
  midstream's analytical primitives in a path that handles
  cryptographic secrets unless you understand this.
- **Denial-of-wallet attacks at the LLM provider.** If an attacker
  drives traffic that costs you provider tokens, midstream can
  rate-limit *its own* throughput but cannot see your billing.
- **Browser sandbox escapes via WASM.** WASM is a sandbox; we
  trust the host (browser / Node) sandbox. We make our public
  surface deliberately small to minimise abuse, but a sandbox
  escape in V8 or SpiderMonkey is out of scope.

## 3. Default-secure modes

Every public knob ships in its safest state. Opting into unsafe is
verbose, greppable, and emits a runtime warning:

| Knob | Default | Unsafe opt-in |
|---|---|---|
| QUIC TLS verifier | OS platform trust store | `--features insecure-dev-only-skip-server-verification` (emits `tracing::warn!`) |
| Sanitizer | `aimds_detection::default_sanitizer()` | `MidstreamBuilder::without_sanitizer()` (visible call) |
| Sanitizer failure policy | fail closed (block on `SanitizerError::Timeout` / `Internal`) | `StreamingLimits.sanitizer_fail_open = true` (logs `error!` span and downgrades to `Allow`) |
| Streaming chunk size | 64 KiB max | `StreamingLimits.max_chunk_bytes` (config) |
| Stream lifetime | 16 MiB cumulative max | `StreamingLimits.max_stream_bytes` |
| Stream rate | 1,000 chunks/s max | `StreamingLimits.max_chunks_per_second` |
| Stream duration | 5 min timeout | `StreamingLimits.max_stream_duration` |
| WASM URL schemes | `["https", "wss"]` | `StreamingLimits.wasm_url_scheme_allowlist` |
| WASM URL hosts | (no allowlist; recommend `Some([…])`) | `StreamingLimits.wasm_url_host_allowlist` |
| `unsafe_code` | `deny` workspace-wide | `#[allow(unsafe_code)]` + doc-comment justification (per crate) |

## 4. Cryptography

- **TLS** via `rustls 0.23` with the platform verifier for trust
  decisions. We deliberately avoid `native-tls` / `openssl-sys` so
  the trust path is auditable from one Rust dep tree.
- **QUIC** via `quinn 0.11`. The crypto provider is `rustls-ring`.
- **No first-party crypto.** midstream does not implement
  primitives. Everything cryptographic is `rustls` /
  `webpki-roots` / `ring` / `rustls-platform-verifier`.
- **No secret-handling primitives.** Provider API keys flow through
  `secrecy::SecretString` (per [ADR-0019](adr/0019-config-system.md)),
  which redacts on `Debug` but does not zeroise. If you need
  zeroising allocations, layer that on top.

## 5. Supply chain

[ADR-0014](adr/0014-supply-chain-pinning.md) is the single source of
truth. Mechanically:

- `Cargo.lock` is committed.
- `cargo-audit` runs on every PR + nightly. Hard gate; the four
  active RustSec advisories at the time of this writing are listed
  in `deny.toml` with ADR-tracked expiries.
- `cargo-deny` runs on every PR with separate matrix entries for
  `advisories`, `bans`, `licenses`, `sources`. Currently
  `continue-on-error` during rollout while pre-existing duplicate-
  version skew (most via the un-vendored hyprstream chain in
  PR #13) clears. Promoted to a hard gate as each remediation ADR
  lands.
- The licence allowlist accepts only permissive SPDX expressions
  (MIT / Apache-2.0 / BSD-2/3 / ISC / Unicode / CC0 / Zlib / MPL-2.0
  / Apache-2.0 WITH LLVM-exception). GPL / AGPL / SSPL are
  forbidden by omission.
- Dependabot opens grouped weekly update PRs ([ADR-0039](adr/0039-governance.md)).

## 6. Posture, honestly

The pre-2026-05-13 reports archived under
[`archive/2026-pre-cleanup/`](archive/2026-pre-cleanup/) claimed
"10/10 security checks passed" and "no hardcoded credentials" while
the QUIC client was actively shipping an unconditional
`SkipServerVerification` cert verifier in production code (fixed in
PR #8). Treat any earlier claim with the same caution we now apply.

Findings that *did* survive the PR #6 fresh-eyes audit:

- **Zero `unsafe ` blocks** in workspace code. Verified by grep;
  enforced going forward by `unsafe_code = "deny"` in
  `[workspace.lints]`.
- **No hardcoded secrets** in source. The grep that did flag
  `config/default.toml:19 api_key = "default_key"` was a
  placeholder string, not a real key, but is being moved to
  `local.toml` (gitignored) under [ADR-0019](adr/0019-config-system.md).
- **TLS-skipping default in `midstreamer-quic 0.1.0`** — a real
  MITM-vulnerable default, now fixed in `0.1.1` via PR #8. **Yank
  0.1.0 from crates.io after the security release lands.**
- **4 active CVEs in `rustls-webpki < 0.103.13`** — present only
  via the orphan `rustls = "0.22"` dep, which PR #8 removes. The
  resulting lockfile contains `rustls-webpki 0.103.13` only.
- **5 unmaintained-transitive advisories** (yaml-rust, dotenv,
  bincode, paste, rustls-pemfile) — each tracked to a specific
  remediation ADR (0019 for the config stack, 0002 for the
  duckdb chain).

## 7. Reporting and disclosure

See [`../SECURITY.md`](../SECURITY.md). Summary:

- **Do not open public issues** for security reports. Use GitHub
  Security Advisories or `security@ruv.net`.
- SLOs: **72h ack / 7d triage / 30d fix / 90d disclosure** for
  high-severity issues. Misses logged publicly in
  `docs/triage-log.md`.
- Supported versions per the stability tier table in
  [ADR-0024](adr/0024-semver-and-api-stability.md). Today only
  `alpha` and `beta` crates exist; security backports are
  best-effort for both.

## 8. Where to read next

- **Reporting policy**: [`../SECURITY.md`](../SECURITY.md).
- **ADRs that drive this posture**:
  [0011](adr/0011-quic-tls-verification.md) (TLS),
  [0012](adr/0012-streaming-input-bounds.md) (input bounds),
  [0013](adr/0013-aimds-integration-contract.md) (AIMDS contract),
  [0014](adr/0014-supply-chain-pinning.md) (supply chain),
  [0015](adr/0015-wasm-egress-allowlist.md) (WASM egress),
  [0034](adr/0034-workspace-lints.md) (lint enforcement),
  [0036](adr/0036-license-reconciliation.md) (licence),
  [0038](adr/0038-fuzz-and-property-tests.md) (fuzz baseline),
  [0039](adr/0039-governance.md) (security routing).
- **Threat-modelling exercises**: not currently in tree. Inputs
  welcome via the discussion forum linked from `CONTRIBUTING.md`.
