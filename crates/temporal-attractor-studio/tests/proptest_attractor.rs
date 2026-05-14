//! Property-based tests for `midstreamer-attractor`. Implements
//! ADR-0038 for the attractor crate.
//!
//! These properties exercise `AttractorAnalyzer::analyze` over random
//! phase-space trajectories and assert the structural invariants the
//! returned `AttractorInfo` must satisfy regardless of input.
//!
//! Invariants asserted (all pass):
//!
//!   * Lyapunov exponents are finite (no NaN/Inf leak).
//!   * Classification is total over the four variants (PointAttractor /
//!     LimitCycle / StrangeAttractor / Unknown) — `analyze` never
//!     panics on any well-formed trajectory.
//!   * `is_stable == lyapunov_exponents.iter().all(|l| l < 0.0)` — the
//!     impl's own derivation, asserted to remain consistent.
//!   * `confidence ∈ [0.0, 1.0]` — confidence is a probability, not
//!     an unbounded score.
//!   * `dimension == embedding_dimension passed to ::new`.
//!   * `is_chaotic()` is `true` iff `attractor_type == StrangeAttractor`.
//!   * `max_lyapunov_exponent()` returns `None` iff `lyapunov_exponents`
//!     is empty; otherwise it returns the actual max.
//!   * Constant trajectory (same point repeated) is never classified as
//!     chaotic — a degenerate trajectory cannot be a strange attractor.

use midstreamer_attractor::{AttractorAnalyzer, AttractorType, PhasePoint};
use proptest::prelude::*;

/// Bounded f64 generator: excludes NaN, Inf, subnormals, and values
/// large enough to overflow Lyapunov accumulators.
fn bounded_coord() -> impl Strategy<Value = f64> {
    (-10.0_f64..10.0).prop_filter("finite", |v| v.is_finite() && !v.is_subnormal())
}

/// Generator for a phase-space trajectory: 50..=200 points of
/// `embedding_dim` coordinates each. The lower bound matches the
/// analyzer's `min_points_for_analysis` default.
fn trajectory_strategy(
    embedding_dim: usize,
    min_points: usize,
    max_points: usize,
) -> impl Strategy<Value = Vec<Vec<f64>>> {
    proptest::collection::vec(
        proptest::collection::vec(bounded_coord(), embedding_dim..=embedding_dim),
        min_points..=max_points,
    )
}

fn analyze_trajectory(
    embedding_dim: usize,
    points: Vec<Vec<f64>>,
) -> Result<midstreamer_attractor::AttractorInfo, midstreamer_attractor::AttractorError> {
    let mut analyzer = AttractorAnalyzer::new(embedding_dim, 1024);
    for (i, coords) in points.into_iter().enumerate() {
        analyzer.add_point(PhasePoint::new(coords, i as u64))?;
    }
    analyzer.analyze()
}

// ---------------------------------------------------------------- Lyapunov.

proptest! {
    /// Every Lyapunov exponent returned by `analyze` is finite.
    #[test]
    fn lyapunov_exponents_finite(points in trajectory_strategy(3, 100, 200)) {
        let info = analyze_trajectory(3, points).unwrap();
        for (i, &le) in info.lyapunov_exponents.iter().enumerate() {
            prop_assert!(
                le.is_finite(),
                "lyapunov_exponents[{}] = {} is not finite", i, le
            );
        }
    }

    /// `is_stable == lyapunov_exponents.all(< 0.0)` — keeps the
    /// derived field consistent with its inputs across all generated
    /// trajectories.
    #[test]
    fn is_stable_matches_lyapunov(points in trajectory_strategy(2, 100, 150)) {
        let info = analyze_trajectory(2, points).unwrap();
        let all_negative = info.lyapunov_exponents.iter().all(|&l| l < 0.0);
        prop_assert_eq!(info.is_stable, all_negative);
    }

    /// `max_lyapunov_exponent()` returns the actual maximum.
    #[test]
    fn max_lyapunov_is_max(points in trajectory_strategy(3, 100, 200)) {
        let info = analyze_trajectory(3, points).unwrap();
        match info.max_lyapunov_exponent() {
            None => prop_assert!(info.lyapunov_exponents.is_empty()),
            Some(m) => {
                prop_assert!(!info.lyapunov_exponents.is_empty());
                let actual_max = info.lyapunov_exponents.iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                prop_assert_eq!(m, actual_max);
            }
        }
    }
}

// ---------------------------------------------------------------- Classification.

proptest! {
    /// `analyze` returns one of the 4 documented variants — never
    /// panics, never returns garbage. The match-arm exhaustiveness
    /// is the property: every code path must produce a valid variant.
    #[test]
    fn classification_is_total(points in trajectory_strategy(3, 100, 200)) {
        let info = analyze_trajectory(3, points).unwrap();
        match info.attractor_type {
            AttractorType::PointAttractor
            | AttractorType::LimitCycle
            | AttractorType::StrangeAttractor
            | AttractorType::Unknown => {} // ok
        }
    }

    /// `is_chaotic()` is exactly `attractor_type == StrangeAttractor`.
    #[test]
    fn is_chaotic_iff_strange(points in trajectory_strategy(3, 100, 200)) {
        let info = analyze_trajectory(3, points).unwrap();
        prop_assert_eq!(
            info.is_chaotic(),
            info.attractor_type == AttractorType::StrangeAttractor
        );
    }

    /// Constant trajectory (same point repeated N times) is never
    /// classified as a strange attractor — a degenerate trajectory
    /// has no chaotic dynamics by definition.
    #[test]
    fn constant_trajectory_is_not_chaotic(coord in bounded_coord(), n in 100usize..=200) {
        let constant_point = vec![coord, coord, coord];
        let points: Vec<Vec<f64>> = (0..n).map(|_| constant_point.clone()).collect();
        let info = analyze_trajectory(3, points).unwrap();
        prop_assert!(
            !info.is_chaotic(),
            "constant trajectory classified as chaotic: {:?}", info.attractor_type
        );
    }
}

// ---------------------------------------------------------------- Confidence + dimension.

proptest! {
    /// Confidence is a probability: 0.0 <= confidence <= 1.0.
    #[test]
    fn confidence_is_probability(points in trajectory_strategy(3, 100, 200)) {
        let info = analyze_trajectory(3, points).unwrap();
        prop_assert!(
            (0.0..=1.0).contains(&info.confidence),
            "confidence {} outside [0.0, 1.0]", info.confidence
        );
    }

    /// AttractorInfo.dimension == embedding_dimension passed to ::new.
    #[test]
    fn dimension_matches_embedding(
        embedding_dim in 1usize..=4,
        n in 100usize..=150,
        coord in bounded_coord(),
    ) {
        let points: Vec<Vec<f64>> = (0..n)
            .map(|_| vec![coord; embedding_dim])
            .collect();
        let info = analyze_trajectory(embedding_dim, points).unwrap();
        prop_assert_eq!(info.dimension, embedding_dim);
    }
}
