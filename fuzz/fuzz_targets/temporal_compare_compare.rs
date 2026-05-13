//! Fuzz target: temporal-compare's `compare()` over the three algorithms.
//!
//! Asserts only "did not panic / did not OOM". Algebraic invariants
//! (metric, symmetry, etc.) are covered by the proptest baseline in
//! `crates/temporal-compare/tests/proptest_metrics.rs`. Fuzz adds
//! the *structured-random bytes* axis: arbitrary input shapes,
//! including the long-tail of edge cases proptest's generators don't
//! reach (empty sequences, single elements, max_sequence_length
//! boundary, etc.).
//!
//! Run with:
//!
//!   cargo +nightly fuzz run temporal_compare_compare
//!
//! A panic or libfuzzer-detected OOM crashes the target with a
//! reproducer dropped under `fuzz/artifacts/temporal_compare_compare/`.

#![no_main]

use libfuzzer_sys::fuzz_target;
use midstreamer_temporal_compare::{
    ComparisonAlgorithm, Sequence, TemporalComparator,
};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    // First byte selects the algorithm; second byte sets the split
    // between the two sequences; remaining bytes are the values.
    let algorithm = match data[0] % 3 {
        0 => ComparisonAlgorithm::DTW,
        1 => ComparisonAlgorithm::LCS,
        _ => ComparisonAlgorithm::EditDistance,
    };

    let split = (data[1] as usize) % data.len().max(1);
    let (a_bytes, b_bytes) = data[2..].split_at(split.min(data.len() - 2));

    let mut seq_a = Sequence::<i32>::new();
    for (i, b) in a_bytes.iter().take(256).enumerate() {
        seq_a.push(*b as i32, i as u64);
    }
    let mut seq_b = Sequence::<i32>::new();
    for (i, b) in b_bytes.iter().take(256).enumerate() {
        seq_b.push(*b as i32, i as u64);
    }

    let comp = TemporalComparator::<i32>::new(128, 256);
    // We don't care about the result — only that `compare` doesn't
    // panic or OOM on any input shape.
    let _ = comp.compare(&seq_a, &seq_b, algorithm);
});
