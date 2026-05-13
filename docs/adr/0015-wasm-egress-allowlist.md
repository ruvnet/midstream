# 0015 — WASM network egress: scheme + host allowlist, byte cap, abort signal

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** security, wasm

## Context and Problem Statement

`wasm-bindings/src/lib.rs:218-247` exposes a
`StreamingHTTPClient::stream` that:

- accepts an arbitrary URL from JS,
- calls `window.fetch_with_request`,
- forwards each chunk to a JS `Function` callback (`:241`),
- has **no origin check, no scheme allowlist, no abort signal, no byte
  ceiling, no timeout**.

`WebSocketClient::new(url)` (`:52`) and the `EventSource`
constructors do the same — they accept caller-supplied URLs without
validation.

This isn't a sandbox escape (browser same-origin policy still applies),
but it turns the WASM module into a transparent `fetch`/`WebSocket`
proxy that runs inside the embedding page's origin. Anyone who
embeds this WASM module ships a wildcard egress to anywhere on its own
origin's allowlist.

Adjacent risks:

- `serde_wasm_bindgen::to_value(...).unwrap()` at
  `npm-wasm/src/lib.rs:391, 484-487` panics the WASM module on
  serialization failure (DoS only; not escape).
- No abort signal means a slow-loris server can hold the WASM thread
  indefinitely.

## Decision Drivers

- **Caller-supplied URLs are untrusted input.** They must be validated
  at the WASM↔JS boundary, not deeper in the stack.
- **Egress must be a stated capability**, not an implicit one. A WASM
  embedder should be able to look at the binding's constructor and see
  what destinations are allowed.
- **WASM-side validation is cheap.** A few hundred CPU cycles per
  connect; vastly less than the TLS handshake that follows.

## Considered Options

1. **Status quo.** No validation.
2. **Scheme allowlist only (`https`, `wss`).** Trivial; closes the
   accidental-plaintext path but not the wildcard-host path.
3. **Scheme + host allowlist, byte cap per stream, abort on
   `AbortSignal`, timeout.** Full belt-and-braces. Host allowlist is
   per-instance (caller-configured), not hard-coded.
4. **Drop the WASM HTTP/WebSocket client entirely; require the
   embedder to provide a `fetch`/`WebSocket` adapter via JS callback.**
   Cleanest separation; most caller friction.

## Decision Outcome

**Chosen option: Option 3 — scheme+host allowlist, byte cap, abort
signal, timeout.** The host allowlist is required at construction
time; passing `None` is permitted but emits a `console.warn` and
records a metric (`midstream.wasm.unrestricted_egress`). The byte cap
and timeout come from `StreamingLimits` (see
[ADR-0012](0012-streaming-input-bounds.md)).

### Positive consequences

- WASM module becomes a real capability boundary: a glance at the
  constructor reveals which hosts it talks to.
- A misconfigured embed (or a compromised JS context) cannot use the
  WASM module to proxy arbitrary egress.
- Slow-loris and runaway-stream DoS classes get bounded by the byte
  cap + timeout + abort signal.

### Negative consequences

- Public WASM constructors gain an additional argument
  (`hostAllowlist?: string[]`). Existing JS callers need to opt in.
  Migration: emit a deprecation warning when `hostAllowlist` is
  `undefined` rather than `[]`/`null`.
- Allowlist enforcement adds ~100ns per connect (host parsing). Trivial.

## Implementation notes

- `wasm-bindings/src/lib.rs`: every public constructor that takes a
  URL acquires the `StreamingLimits` shared from
  [ADR-0012](0012-streaming-input-bounds.md). Validate:
  - `url.scheme() ∈ limits.wasm_url_scheme_allowlist` (default
    `["https", "wss"]`).
  - if `limits.wasm_url_host_allowlist.is_some()`:
    `url.host() ∈ allowlist`.
- `StreamingHTTPClient::stream` wires an `AbortController` from JS,
  honours `limits.max_stream_duration` as a timeout, enforces
  `limits.max_chunk_bytes` and `limits.max_stream_bytes`.
- Replace `serde_wasm_bindgen::to_value(...).unwrap()` with `?` and
  surface as a real `JsError`.
- Add `tests/wasm_egress_tests.rs` (wasm-bindgen-test) covering: HTTP
  scheme rejection, off-allowlist host rejection, byte cap, timeout,
  abort signal.

## Links

- Related: [ADR-0012](0012-streaming-input-bounds.md),
  [ADR-0011](0011-quic-tls-verification.md).
- MDN AbortController:
  https://developer.mozilla.org/en-US/docs/Web/API/AbortController
