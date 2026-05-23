//! Property-based tests for the comparison algorithms in
//! `midstreamer-temporal-compare`. Implements ADR-0038.
//!
//! Each test asserts an algebraic invariant the algorithm MUST
//! satisfy regardless of input. proptest generates ~256 cases per
//! property per CI run; shrinking failures land in
//! `proptest-regressions/proptest_metrics.txt` (committed) so any
//! discovered counterexample is replayed on subsequent runs.
//!
//! Invariants asserted:
//!
//! * Edit-distance is a metric:
//!   d(x, x) = 0
//!   d(x, y) >= 0
//!   d(x, y) = d(y, x)            (symmetry)
//!   d(x, y) <= max(|x|, |y|)     (sup bound)
//!
//! * DTW:
//!   d(x, x) = 0
//!   d(x, y) >= 0
//!   d(x, y) = d(y, x)
//!
//! * LCS:
//!   d(x, y) is finite (no NaN/Inf leak)
//!   d(x, y) = d(y, x)
//!
//! `TemporalComparator<T>` requires `T: Clone + PartialEq + Debug +
//! Serialize + Hash + Eq`, so the test uses `i32` (`f64` would fail
//! the Hash+Eq bound due to NaN semantics). The Levenshtein /
//! LCS / DTW algorithms compare values only via `==`, so the choice
//! of integer-vs-float values does not change which code paths run.

use midstreamer_temporal_compare::{ComparisonAlgorithm, Sequence, TemporalComparator};
use proptest::prelude::*;

/// Build a `Sequence<i32>` from a Vec by assigning evenly-spaced
/// integer timestamps. The comparator doesn't use timestamps for
/// DTW / LCS / edit-distance distance computation, so the spacing
/// is arbitrary.
fn make_sequence(values: Vec<i32>) -> Sequence<i32> {
    let mut s = Sequence::new();
    for (i, v) in values.into_iter().enumerate() {
        s.push(v, i as u64);
    }
    s
}

/// Generator for sequences of bounded length (0..=24) over a small
/// integer alphabet (-8..=8). The small alphabet maximises the
/// probability that randomly-generated sequences share elements,
/// which exercises the equal-element branch in DTW and the match
/// branch in LCS / edit-distance.
fn bounded_sequence() -> impl Strategy<Value = Vec<i32>> {
    proptest::collection::vec(-8_i32..=8, 0..=24)
}

fn nonempty_sequence() -> impl Strategy<Value = Vec<i32>> {
    proptest::collection::vec(-8_i32..=8, 1..=24)
}

// ---------------------------------------------------------------- Edit distance.

proptest! {
    /// d(x, x) = 0.
    #[test]
    fn edit_distance_reflexive(values in bounded_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let seq = make_sequence(values);
        let result = comp.compare(&seq, &seq, ComparisonAlgorithm::EditDistance).unwrap();
        prop_assert_eq!(result.distance, 0.0);
    }

    /// d(x, y) >= 0.
    #[test]
    fn edit_distance_non_negative(a in bounded_sequence(), b in bounded_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let result = comp.compare(&make_sequence(a), &make_sequence(b), ComparisonAlgorithm::EditDistance).unwrap();
        prop_assert!(result.distance >= 0.0, "edit_distance returned negative {}", result.distance);
    }

    /// d(x, y) = d(y, x).
    #[test]
    fn edit_distance_symmetric(a in bounded_sequence(), b in bounded_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let d_ab = comp.compare(&make_sequence(a.clone()), &make_sequence(b.clone()), ComparisonAlgorithm::EditDistance).unwrap();
        let d_ba = comp.compare(&make_sequence(b), &make_sequence(a), ComparisonAlgorithm::EditDistance).unwrap();
        prop_assert_eq!(d_ab.distance, d_ba.distance);
    }

    /// d(x, y) <= max(|x|, |y|). Edit-distance is bounded by the
    /// length of the longer string — you can never need more
    /// operations than that.
    #[test]
    fn edit_distance_bounded_by_max_length(a in bounded_sequence(), b in bounded_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let upper = a.len().max(b.len()) as f64;
        let result = comp.compare(&make_sequence(a), &make_sequence(b), ComparisonAlgorithm::EditDistance).unwrap();
        prop_assert!(result.distance <= upper, "edit_distance {} > upper bound {}", result.distance, upper);
    }
}

// ---------------------------------------------------------------- DTW.

proptest! {
    /// d(x, x) = 0.
    ///
    /// DTW initialises `dtw[0][0] = 0.0` and propagates via `min` of
    /// three predecessors plus a `0/1` cost. For equal sequences,
    /// every comparison hits the equal branch (cost 0), so the
    /// optimal path stays on the diagonal with total cost 0.
    #[test]
    fn dtw_reflexive(values in nonempty_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let seq = make_sequence(values);
        let result = comp.compare(&seq, &seq, ComparisonAlgorithm::DTW).unwrap();
        prop_assert_eq!(result.distance, 0.0);
    }

    /// d(x, y) >= 0. (The DTW DP only ever adds non-negative costs
    /// from a 0.0 origin, so this should hold by construction.)
    #[test]
    fn dtw_non_negative(a in nonempty_sequence(), b in nonempty_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let result = comp.compare(&make_sequence(a), &make_sequence(b), ComparisonAlgorithm::DTW).unwrap();
        prop_assert!(result.distance >= 0.0, "DTW returned negative {}", result.distance);
    }

    /// d(x, y) = d(y, x).
    #[test]
    fn dtw_symmetric(a in nonempty_sequence(), b in nonempty_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let d_ab = comp.compare(&make_sequence(a.clone()), &make_sequence(b.clone()), ComparisonAlgorithm::DTW).unwrap();
        let d_ba = comp.compare(&make_sequence(b), &make_sequence(a), ComparisonAlgorithm::DTW).unwrap();
        prop_assert_eq!(d_ab.distance, d_ba.distance);
    }
}

// ---------------------------------------------------------------- LCS.

proptest! {
    /// LCS distance must be finite and non-NaN — no f64-arithmetic
    /// pathology can leak into the cache or onward computations.
    #[test]
    fn lcs_finite(a in bounded_sequence(), b in bounded_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let result = comp.compare(&make_sequence(a), &make_sequence(b), ComparisonAlgorithm::LCS).unwrap();
        prop_assert!(result.distance.is_finite(), "LCS returned non-finite {}", result.distance);
    }

    /// d(x, y) = d(y, x).
    #[test]
    fn lcs_symmetric(a in bounded_sequence(), b in bounded_sequence()) {
        let comp = TemporalComparator::<i32>::new(128, 256);
        let d_ab = comp.compare(&make_sequence(a.clone()), &make_sequence(b.clone()), ComparisonAlgorithm::LCS).unwrap();
        let d_ba = comp.compare(&make_sequence(b), &make_sequence(a), ComparisonAlgorithm::LCS).unwrap();
        prop_assert_eq!(d_ab.distance, d_ba.distance);
    }
}
