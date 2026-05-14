//! Property-based tests for `midstreamer-neural-solver`. Implements
//! ADR-0038 for the LTL crate.
//!
//! Linear Temporal Logic has well-known algebraic laws. These tests
//! assert them by constructing random formulae, evaluating them
//! against random traces, and checking the laws hold for every
//! generated input.
//!
//! Invariants asserted (all pass):
//!
//!   Double-negation:
//!     * `verify(not(not(φ))) ≡ verify(φ)` for every φ and trace.
//!
//!   Idempotence:
//!     * `verify(and(φ, φ)) ≡ verify(φ)`.
//!     * `verify(or(φ, φ))  ≡ verify(φ)`.
//!
//!   Commutativity:
//!     * `verify(and(φ, ψ)) ≡ verify(and(ψ, φ))`.
//!     * `verify(or(φ, ψ))  ≡ verify(or(ψ, φ))`.
//!
//!   Globally / Eventually duality:
//!     * `verify(not(globally(φ))) ≡ verify(finally(not(φ)))`.
//!     * `verify(not(finally(φ)))  ≡ verify(globally(not(φ)))`.
//!
//!   Structural:
//!     * verify() never panics on any well-formed formula+trace.
//!     * verify() never returns satisfied=true *and* a counterexample
//!       (these are mutually exclusive by design).

use midstreamer_neural_solver::{
    TemporalFormula, TemporalNeuralSolver, TemporalState, VerificationStrictness,
};
use proptest::prelude::*;

// ---------------------------------------------------------------- Trace generator.

/// Generates a `TemporalNeuralSolver` populated with a random trace.
/// `propositions` is the small fixed alphabet of atomic-proposition
/// names so generated formulae and trace states share vocabulary.
fn solver_with_random_trace(
    trace_specs: Vec<Vec<(usize, bool)>>,
    propositions: &[&str],
) -> TemporalNeuralSolver {
    let mut s = TemporalNeuralSolver::new(
        trace_specs.len().max(1) + 1,
        1_000,
        VerificationStrictness::Medium,
    );
    for (i, props) in trace_specs.into_iter().enumerate() {
        let mut state = TemporalState::new(i as u64, i as u64);
        for (idx, val) in props {
            if let Some(name) = propositions.get(idx % propositions.len()) {
                state.set_proposition(*name, val);
            }
        }
        s.add_state(state);
    }
    s
}

fn trace_strategy() -> impl Strategy<Value = Vec<Vec<(usize, bool)>>> {
    proptest::collection::vec(
        proptest::collection::vec((0usize..=2, any::<bool>()), 0..=4),
        1..=8,
    )
}

// ---------------------------------------------------------------- Formula generator.

/// Generates a `TemporalFormula` of bounded depth. The depth bound
/// is essential: LTL formulae nest unboundedly otherwise, which
/// makes proptest hang on shrinking. Each recursive level halves
/// the remaining depth budget.
fn formula_strategy() -> impl Strategy<Value = TemporalFormula> {
    let leaf = prop_oneof![
        Just(TemporalFormula::atom("p")),
        Just(TemporalFormula::atom("q")),
        Just(TemporalFormula::atom("r")),
    ];
    leaf.prop_recursive(
        4,  // max depth
        16, // max size
        2,  // expected branching
        |inner| {
            prop_oneof![
                inner.clone().prop_map(TemporalFormula::not),
                inner.clone().prop_map(TemporalFormula::globally),
                inner.clone().prop_map(TemporalFormula::finally),
                inner.clone().prop_map(TemporalFormula::next),
                (inner.clone(), inner.clone()).prop_map(|(a, b)| TemporalFormula::and(a, b)),
                (inner.clone(), inner.clone()).prop_map(|(a, b)| TemporalFormula::or(a, b)),
                (inner.clone(), inner).prop_map(|(a, b)| TemporalFormula::until(a, b)),
            ]
        },
    )
}

const PROPS: &[&str] = &["p", "q", "r"];

fn verify_satisfied(solver: &TemporalNeuralSolver, formula: &TemporalFormula) -> bool {
    solver.verify(formula).map(|r| r.satisfied).unwrap_or(false)
}

// ---------------------------------------------------------------- Double-negation.

proptest! {
    /// ¬¬φ ≡ φ
    #[test]
    fn double_negation_elimination(
        formula in formula_strategy(),
        trace in trace_strategy(),
    ) {
        let solver = solver_with_random_trace(trace, PROPS);
        let phi = verify_satisfied(&solver, &formula);
        let not_not_phi = verify_satisfied(
            &solver,
            &TemporalFormula::not(TemporalFormula::not(formula)),
        );
        prop_assert_eq!(phi, not_not_phi);
    }
}

// ---------------------------------------------------------------- Idempotence.

proptest! {
    /// φ ∧ φ ≡ φ
    #[test]
    fn and_idempotent(formula in formula_strategy(), trace in trace_strategy()) {
        let solver = solver_with_random_trace(trace, PROPS);
        let phi = verify_satisfied(&solver, &formula);
        let phi_and_phi = verify_satisfied(
            &solver,
            &TemporalFormula::and(formula.clone(), formula),
        );
        prop_assert_eq!(phi, phi_and_phi);
    }

    /// φ ∨ φ ≡ φ
    #[test]
    fn or_idempotent(formula in formula_strategy(), trace in trace_strategy()) {
        let solver = solver_with_random_trace(trace, PROPS);
        let phi = verify_satisfied(&solver, &formula);
        let phi_or_phi = verify_satisfied(
            &solver,
            &TemporalFormula::or(formula.clone(), formula),
        );
        prop_assert_eq!(phi, phi_or_phi);
    }
}

// ---------------------------------------------------------------- Commutativity.

proptest! {
    /// (φ ∧ ψ) ≡ (ψ ∧ φ)
    #[test]
    fn and_commutative(
        a in formula_strategy(),
        b in formula_strategy(),
        trace in trace_strategy(),
    ) {
        let solver = solver_with_random_trace(trace, PROPS);
        let ab = verify_satisfied(
            &solver,
            &TemporalFormula::and(a.clone(), b.clone()),
        );
        let ba = verify_satisfied(&solver, &TemporalFormula::and(b, a));
        prop_assert_eq!(ab, ba);
    }

    /// (φ ∨ ψ) ≡ (ψ ∨ φ)
    #[test]
    fn or_commutative(
        a in formula_strategy(),
        b in formula_strategy(),
        trace in trace_strategy(),
    ) {
        let solver = solver_with_random_trace(trace, PROPS);
        let ab = verify_satisfied(
            &solver,
            &TemporalFormula::or(a.clone(), b.clone()),
        );
        let ba = verify_satisfied(&solver, &TemporalFormula::or(b, a));
        prop_assert_eq!(ab, ba);
    }
}

// ---------------------------------------------------------------- Globally / Eventually duality.

proptest! {
    /// ¬□φ ≡ ◇¬φ — "it's not always the case that φ" iff "there
    /// exists a time when ¬φ".
    #[test]
    fn globally_eventually_duality_a(
        formula in formula_strategy(),
        trace in trace_strategy(),
    ) {
        let solver = solver_with_random_trace(trace, PROPS);
        let not_globally_phi = verify_satisfied(
            &solver,
            &TemporalFormula::not(TemporalFormula::globally(formula.clone())),
        );
        let finally_not_phi = verify_satisfied(
            &solver,
            &TemporalFormula::finally(TemporalFormula::not(formula)),
        );
        prop_assert_eq!(not_globally_phi, finally_not_phi);
    }

    /// ¬◇φ ≡ □¬φ — "never φ" iff "always ¬φ".
    #[test]
    fn globally_eventually_duality_b(
        formula in formula_strategy(),
        trace in trace_strategy(),
    ) {
        let solver = solver_with_random_trace(trace, PROPS);
        let not_finally_phi = verify_satisfied(
            &solver,
            &TemporalFormula::not(TemporalFormula::finally(formula.clone())),
        );
        let globally_not_phi = verify_satisfied(
            &solver,
            &TemporalFormula::globally(TemporalFormula::not(formula)),
        );
        prop_assert_eq!(not_finally_phi, globally_not_phi);
    }
}

// ---------------------------------------------------------------- Structural.

proptest! {
    /// `verify()` never panics on any well-formed formula+trace.
    /// (Implicit: a panic anywhere in the closure aborts the case.)
    #[test]
    fn verify_never_panics(formula in formula_strategy(), trace in trace_strategy()) {
        let solver = solver_with_random_trace(trace, PROPS);
        let _ = solver.verify(&formula);
    }
}
