# ADR 0040: Separate Realtime Reflex Control From Deliberative Reasoning

Status: proposed

Date: 2026-09-09

Issue: #106

## Context

Realtime agents have two incompatible latency regimes. Interaction control such as interruption acknowledgement, backchannel, pause, and cancellation must react quickly. Planning, tool selection, and complex reasoning can take much longer. Running both through one deliberative loop increases interruption latency and wastes inference when work should have been cancelled earlier.

## Decision

Add a small MidStream reflex controller in front of the deliberative agent.

The reflex controller may only acknowledge interaction, pause reasoning, request cancellation, and forward observations to the reasoner. It cannot invoke privileged tools, grant capabilities, mutate external state, or declare the task complete.

Ruflo remains responsible for deliberative reasoning. RVM remains responsible for privileged external effects.

## Invariants

1. Every reflex receipt has `authority: none`.
2. Event sequence numbers are monotonic. Stale events fail closed.
3. Queues are bounded.
4. Adjacent repeated interruptions are coalesced to avoid interrupt storms.
5. Cancellation accounts for abandoned work rather than hiding it.
6. The reasoner handoff preserves event order and cancellation provenance.
7. Reflex processing cannot execute network, filesystem, tool, credential, or capability operations.

## Benchmark

Compare a monolithic controller and the split controller on matched event traces containing normal speech, interruptions, repeated interruptions, cancellation, delayed observations, stale events, queue pressure, and deliberative work of varying duration.

Report acknowledgement latency, completion quality proxy, cancelled work units, wasted work, queue depth, stale drops, overflow, p50 and p99 reflex processing latency, and total model cost.

## Promotion gate

1. interruption acknowledgement latency improves by at least 40 percent
2. task quality remains within 2 absolute percentage points
3. wasted deliberative work after cancellation remains below 10 percent
4. stale and out of order events never trigger state changing reflexes
5. p99 reflex processing remains below the declared realtime budget
6. no reflex receipt carries execution authority

## Security

Reflex events are untrusted observations. Even a valid interrupt or cancellation request does not grant external effect authority. RVM remains the only privileged enforcement boundary.

## Rollback

The module is additive. Removing `src/reflex.rs` and its export returns MidStream to the current monolithic path. No persisted state migration is introduced.

## Cross stack

MidStream owns reflex arbitration. Ruflo owns deliberation. MetaHarness independently benchmarks split versus monolithic behavior. RVM owns authority. Core Memory may retain receipts. Cognitum can use the split in voice, robotics, ambient, and realtime customer agent products.
