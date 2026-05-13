# 0016 — Redesign the `LLMClient` provider trait

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** api, provider, streaming

## Context and Problem Statement

The current provider trait is too thin to be usable in any realistic
deployment. From `src/midstream.rs:52`:

```rust
pub trait LLMClient: Send + Sync {
    fn stream(&self) -> BoxStream<'static, String>;
}
```

There is:

- **No prompt parameter.** The prompt is baked into the implementation
  (`examples/openrouter.rs:30-38` hardcodes `"Tell me a short story
  about a robot learning to paint…"` inside `stream()`).
- **No model parameter.** Model selection lives in `env::var
  ("OPENROUTER_MODEL")` (`examples/openrouter.rs:32`).
- **No message history or context.** Multi-turn conversations cannot
  be expressed.
- **No error type.** The stream yields `String`; failures are silently
  swallowed (cf. `examples/openrouter.rs:68-90` which `println!`'s
  parse errors and emits an empty string).
- **No abort signal / cancellation.** A misbehaving upstream cannot be
  cleanly stopped from the consumer side.
- **No tool-call surface.** The trait is text-only; modern providers
  (OpenAI, Anthropic, Gemini) all support structured tool calls.
- **No usage / metering.** Token counts, finish reason, and rate-limit
  headers are unrecoverable.

`StreamProcessor::process_stream` (`src/midstream.rs:171-183`) just
loops over `stream.next().await` with no way to distinguish "stream
ended normally" from "stream errored out".

## Decision Drivers

- **Provider parity.** The same trait must back OpenAI Realtime, the
  Chat Completions API, Anthropic, Gemini, OpenRouter, vLLM, and local
  Ollama — without ad-hoc shims.
- **Error transparency.** Stream events must distinguish content,
  errors, end-of-stream, and rate-limit signals.
- **Cancellation.** Callers must be able to abort a running stream.
- **Forward compatibility.** Tool calls and multi-modal (image/audio
  in, image/audio out) are imminent for every provider; the trait
  should not be re-broken six months from now.

## Considered Options

1. **Keep `BoxStream<String>`, add a separate `prompt()` setter.**
   Minimal change; doesn't fix the missing error type, abort signal, or
   tool calls.
2. **New trait `LlmProvider` with an associated `Request`/`Response`
   pair and a `BoxStream<Event>` of typed events.** Modelled on
   `genai` / `rust-genai` / `async-openai`'s emerging shape.
3. **Adopt `genai` as the upstream trait directly** and wrap it.
   Lowest API-design cost; couples us to a third-party trait's
   semver.
4. **Define our own trait** but make it convertible to/from `genai`
   via `From` impls in a feature-gated adapter crate. Best of both:
   our public surface is ours, but plugging existing providers is free.

## Decision Outcome

**Chosen option: Option 4 — define our own `LlmProvider` trait, ship a
feature-gated adapter to `genai` and an adapter to OpenAI Realtime
WS.**

```rust
pub trait LlmProvider: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn complete(
        &self,
        req: LlmRequest,
        cancel: CancellationToken,
    ) -> Result<BoxStream<'static, Result<LlmEvent, Self::Error>>, Self::Error>;
}

pub struct LlmRequest {
    pub model: SmolStr,
    pub messages: Vec<LlmMessage>,
    pub tools: Vec<ToolSpec>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stop: Vec<SmolStr>,
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub enum LlmEvent {
    ContentDelta { text: Bytes },
    ToolCallDelta { id: SmolStr, name: SmolStr, args_delta: Bytes },
    Usage(UsageStats),
    Finish(FinishReason),
}
```

The existing `LLMClient` trait becomes a deprecated re-export that
forwards to `LlmProvider` with a fixed `LlmRequest::default()`.

### Positive consequences

- Multi-turn conversations, tool calls, and abort signals are now
  first-class.
- Errors are typed and propagated through the stream as
  `Result<LlmEvent, Self::Error>`, not silently dropped.
- `Bytes` content matches [ADR-0006](0006-zero-copy-bytes-streaming.md);
  no per-token `String` allocation.
- A single trait swap unifies the four current example providers
  (`openrouter.rs`, `lean_agentic_streaming.rs`, OpenAI Realtime, the
  test `SimulatedLLMClient`) and any future ones.

### Negative consequences

- Breaking change to the published `midstream` crate's public API. We
  ship as v0.2 with a deprecated shim for the old `LLMClient`.
- The `async fn` in trait approach requires Rust 1.75+ (already our
  MSRV target; cf. ADR-0023 future) or `async_trait::async_trait`
  meanwhile.
- Tool-call schema is shaped by today's OpenAI/Anthropic
  semantics; future providers may not fit. Mitigation: `extra:
  serde_json::Map` escape hatch.

## Implementation notes

- New module `src/provider.rs` with `LlmProvider`, `LlmRequest`,
  `LlmEvent`, `LlmMessage`, `ToolSpec`, `FinishReason`, `UsageStats`.
- Replace `examples/openrouter.rs` with one that takes the prompt from
  argv and the model from CLI flag; the API key still comes from env.
- Replace `examples/lean_agentic_streaming.rs::SimulatedLLMClient`
  with a `SimulatedLlmProvider` that streams a canned conversation.
- Behind feature `provider-genai`, add an adapter `GenAiProvider` that
  wraps `genai::Client`.
- Behind feature `provider-openai-realtime`, ship a WebSocket-backed
  provider that exposes the OpenAI Realtime API as `LlmProvider`.
- Deprecate `LLMClient`: leave a `pub trait LLMClient` that auto-impls
  for any `T: LlmProvider` with a default `LlmRequest`.

## Links

- Related: [ADR-0006](0006-zero-copy-bytes-streaming.md),
  [ADR-0013](0013-aimds-integration-contract.md).
- `genai` crate: https://docs.rs/genai/
- `async-openai`: https://docs.rs/async-openai/
- `tokio_util::sync::CancellationToken`:
  https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html
