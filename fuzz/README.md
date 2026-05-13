# midstream-fuzz

cargo-fuzz scaffold per [ADR-0038](../docs/adr/0038-fuzz-and-property-tests.md).

Standalone crate (excluded from the root workspace) hosting
libfuzzer-sys targets that exercise the workspace's parser /
algorithmic / state-machine surfaces beyond what the proptest
baselines reach.

## Targets

| Target | Asserts | Notes |
|---|---|---|
| `temporal_compare_compare` | `TemporalComparator::compare` does not panic / OOM on any (sequence_a, sequence_b, algorithm) tuple | Algorithmic; ADR-0009 |
| `scheduler_event_loop` | `RealtimeScheduler` does not panic / OOM under random `(schedule, next_task, clear)` interleavings | State-machine; ADR-0033 |

Each target asserts only the absence of panics + OOM. The
algebraic / structural invariants live in the per-crate proptest
baselines under `crates/*/tests/proptest_*.rs`.

## Running

```bash
# One-time tool install
cargo install cargo-fuzz

# Run a target (libfuzzer requires nightly):
cargo +nightly fuzz run temporal_compare_compare
cargo +nightly fuzz run scheduler_event_loop

# Bounded run for CI (60 seconds):
cargo +nightly fuzz run temporal_compare_compare -- -max_total_time=60
```

## Layout

```
fuzz/
├── Cargo.toml             # not a workspace member; declares libfuzzer-sys + target bins
├── fuzz_targets/
│   ├── temporal_compare_compare.rs
│   └── scheduler_event_loop.rs
├── artifacts/             # crashes go here; committed under .gitignore exception
└── corpus/                # seed corpus + libfuzzer's discovered inputs
```

Both `artifacts/` and `corpus/` are runtime directories; the seed
corpus (`corpus/<target>/*`) is committed so CI replays prior
findings, while libfuzzer-discovered new inputs accumulate locally.

## CI

A `fuzz.yml` workflow is the natural next step (per ADR-0038's
implementation notes): nightly-toolchain matrix entry that runs
each target with `-max_total_time=300s` on a schedule + on PR
label `fuzz`. Crashes upload as workflow artifacts.

## Follow-up targets

Per ADR-0038's table, the priority list is:

  * sse_parser — SSE event stream → frame
  * quic_frame_decode — bytes → QuicFrame
  * mcp_jsonrpc_dispatch — JSON-RPC request → tool dispatch
  * figment_config_load — TOML/JSON bytes → MidstreamConfig

These land as separate PRs as each underlying surface stabilises.
