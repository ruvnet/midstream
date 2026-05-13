# crates.io yank log

This file tracks every version midstream has yanked from crates.io,
in chronological order. Yanking is observable (the version stays
*resolvable* for explicit pins but becomes *invisible* to new
dependency resolution), so each entry needs a paper trail.

A yank can always be undone (`cargo yank --undo`), but a yank
should never be casual — every entry here is a deliberate signal to
the downstream ecosystem.

## Policy

A version is yanked when **all three** are true:

1. It contains a real defect (security vulnerability, correctness
   bug, or licence error that materially harms a consumer).
2. A fix has been published as a newer compatible version.
3. The `^M.N.x` semver range that consumers typically pin will
   resolve to the fix automatically.

Documentation-only or marketing changes are never grounds for a
yank. Cosmetic bugs without a security or correctness dimension
should be left published.

## Entries

### 2026-05-13 — `midstreamer-quic 0.1.0`

- **Reason:** TLS server-certificate verification was disabled by
  default. The `QuicConnection::connect` path unconditionally
  installed a `SkipServerVerification` cert verifier (verified by
  the PR #6 deep-review agent at `crates/quic-multistream/src/native.rs:39-43`
  of the pre-fix tree). Any downstream consumer of `midstreamer-quic
  0.1.0` who called `connect()` inherited an MITM-vulnerable client.
- **Fix shipped as:** `midstreamer-quic 0.1.1` (PR #8, ADR-0011).
  Default verifier is now `rustls-platform-verifier`; the legacy
  skip-verify behaviour is gated behind a feature flag
  `insecure-dev-only-skip-server-verification`, isolated in
  `crates/quic-multistream/src/insecure.rs`, and emits a
  `tracing::warn!` on every connect.
- **Yank command:** `cargo yank --version 0.1.0 midstreamer-quic`
- **Recommended consumer action:** none — `cargo update` resolves
  `^0.1` to `0.1.1` automatically. Consumers pinning `=0.1.0`
  should rebuild against `0.1.1` (no API break).
- **ADR:** [`adr/0011-quic-tls-verification.md`](adr/0011-quic-tls-verification.md)

### 2026-05-13 — `midstreamer-scheduler 0.1.0` and `0.1.1`

- **Reason:** Two ordering bugs in `ScheduledTask::cmp` that caused
  `RealtimeScheduler::next_task()` to pop tasks in the opposite of
  the documented order:
    1. **Priority ordering inverted.** Tasks were popped lowest
       discriminant-priority first instead of highest. So Critical
       (discriminant 100) was popped *after* Background
       (discriminant 10) — exactly the opposite of the doc-comment
       claim ("Higher priority first").
    2. **Within-priority deadline ordering inverted.** For two
       equal-priority tasks, the LATER deadline popped first
       instead of the earlier one — opposite of "earlier deadline
       first".
  Discovered by the proptest baseline in PR #59 with shrunk
  counterexamples checked into the proptest-regressions corpus.
- **Fix shipped as:** `midstreamer-scheduler 0.1.2` (PR #60 fixes
  both swaps; PR #61 releases). The strict ordering invariant is
  now asserted by the proptest
  `next_task_emits_priority_desc_then_deadline_asc`.
- **Yank commands:**
    `cargo yank --version 0.1.0 midstreamer-scheduler`
    `cargo yank --version 0.1.1 midstreamer-scheduler`
- **Recommended consumer action:** none — `cargo update` resolves
  `^0.1` to `0.1.2`. Behaviour change is in the direction of the
  documented contract: tasks now pop in the order the doc-comment
  always promised. Consumers who relied on the buggy ordering have
  a workaround: don't.
- **ADR:** none required (bugfix, not architectural). Discovery
  documented in [PR #59](https://github.com/ruvnet/midstream/pull/59);
  fix in [PR #60](https://github.com/ruvnet/midstream/pull/60);
  release in [PR #61](https://github.com/ruvnet/midstream/pull/61).
