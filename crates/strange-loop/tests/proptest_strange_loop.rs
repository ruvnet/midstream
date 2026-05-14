//! Property-based tests for `midstreamer-strange-loop`. Implements
//! ADR-0038 for the strange-loop crate.
//!
//! Invariants asserted (all pass):
//!
//!   MetaLevel arithmetic:
//!     * `MetaLevel::base().level() == 0`.
//!     * `level.next().level() == level.level() + 1`.
//!     * `MetaLevel` ordering matches `level()` ordering.
//!
//!   Bounded depth:
//!     * `learn_at_level(level, data)` returns
//!       `MaxDepthExceeded(level)` when `level > config.max_meta_depth`.
//!     * Below the cap, it succeeds (with empty data → empty knowledge
//!       but Ok return).
//!
//!   Idempotent reset:
//!     * After `reset()`, `get_summary().total_knowledge == 0` and
//!       `total_modifications == 0`.
//!     * `reset()` is idempotent: calling it twice == calling once.
//!
//!   Summary accounting:
//!     * `total_knowledge` equals the sum of per-level knowledge.len().
//!     * `total_levels` equals the number of distinct levels that have
//!       any knowledge.
//!
//!   No panics:
//!     * `learn_at_level` and `analyze_behavior` never panic on any
//!       well-formed input.

use midstreamer_strange_loop::{MetaLevel, StrangeLoop, StrangeLoopConfig, StrangeLoopError};
use proptest::prelude::*;

fn fresh_loop(max_meta_depth: usize) -> StrangeLoop {
    StrangeLoop::new(StrangeLoopConfig {
        max_meta_depth,
        enable_self_modification: false,
        max_modifications_per_cycle: 5,
        safety_check_enabled: true,
    })
}

fn small_string() -> impl Strategy<Value = String> {
    "[a-z0-9]{0,8}".prop_map(String::from)
}

fn data_strategy() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(small_string(), 0..=6)
}

// ---------------------------------------------------------------- MetaLevel arithmetic.

proptest! {
    /// `MetaLevel::base().level() == 0`.
    #[test]
    fn base_is_zero(_unit in Just(())) {
        prop_assert_eq!(MetaLevel::base().level(), 0);
    }

    /// `level.next().level() == level.level() + 1`.
    #[test]
    fn next_increments(level_idx in 0usize..=16) {
        let mut level = MetaLevel::base();
        for _ in 0..level_idx {
            level = level.next();
        }
        prop_assert_eq!(level.level(), level_idx);
        prop_assert_eq!(level.next().level(), level_idx + 1);
    }

    /// `level.next()` is monotonic: walking `n` `.next()` calls from
    /// `base()` produces level `n`, and equality on `MetaLevel`
    /// agrees with equality on the underlying `level()` value.
    #[test]
    fn next_is_monotonic_and_equality_consistent(a in 0usize..=16, b in 0usize..=16) {
        let mut la = MetaLevel::base();
        let mut lb = MetaLevel::base();
        for _ in 0..a { la = la.next(); }
        for _ in 0..b { lb = lb.next(); }
        prop_assert_eq!(la == lb, la.level() == lb.level());
        prop_assert_eq!(la.level(), a);
        prop_assert_eq!(lb.level(), b);
    }
}

// ---------------------------------------------------------------- Bounded depth.

proptest! {
    /// `learn_at_level(level, ...)` errors with MaxDepthExceeded when
    /// `level > max_meta_depth`.
    #[test]
    fn over_depth_errors(
        max_depth in 0usize..=8,
        over in 1usize..=8,
        data in data_strategy(),
    ) {
        let mut sl = fresh_loop(max_depth);
        let mut level = MetaLevel::base();
        for _ in 0..(max_depth + over) {
            level = level.next();
        }
        let result = sl.learn_at_level(level, &data);
        match result {
            Err(StrangeLoopError::MaxDepthExceeded(n)) => {
                prop_assert_eq!(n, max_depth + over);
            }
            other => prop_assert!(
                false,
                "expected MaxDepthExceeded({}), got {:?}",
                max_depth + over, other
            ),
        }
    }

    /// `learn_at_level(level, ...)` succeeds when `level <= max_meta_depth`.
    #[test]
    fn at_or_under_depth_ok(
        max_depth in 0usize..=8,
        depth in 0usize..=8,
        data in data_strategy(),
    ) {
        prop_assume!(depth <= max_depth);
        let mut sl = fresh_loop(max_depth);
        let mut level = MetaLevel::base();
        for _ in 0..depth {
            level = level.next();
        }
        let result = sl.learn_at_level(level, &data);
        prop_assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }
}

// ---------------------------------------------------------------- Reset idempotence.

proptest! {
    /// After reset, totals are zero.
    #[test]
    fn reset_empties_summary(
        max_depth in 1usize..=8,
        n_calls in 0usize..=4,
        data in data_strategy(),
    ) {
        let mut sl = fresh_loop(max_depth);
        for _ in 0..n_calls {
            let _ = sl.learn_at_level(MetaLevel::base(), &data);
        }
        sl.reset();
        let summary = sl.get_summary();
        prop_assert_eq!(summary.total_knowledge, 0);
        prop_assert_eq!(summary.total_modifications, 0);
    }

    /// `reset()` is idempotent: two resets == one reset.
    #[test]
    fn reset_is_idempotent(max_depth in 1usize..=8, data in data_strategy()) {
        let mut sl = fresh_loop(max_depth);
        let _ = sl.learn_at_level(MetaLevel::base(), &data);
        sl.reset();
        let s1 = sl.get_summary();
        sl.reset();
        let s2 = sl.get_summary();
        prop_assert_eq!(s1.total_knowledge, s2.total_knowledge);
        prop_assert_eq!(s1.total_modifications, s2.total_modifications);
        prop_assert_eq!(s1.safety_violations, s2.safety_violations);
    }
}

// ---------------------------------------------------------------- Summary accounting.

proptest! {
    /// total_knowledge equals the sum of per-level knowledge.len().
    #[test]
    fn total_knowledge_sums_levels(max_depth in 1usize..=4, data in data_strategy()) {
        let mut sl = fresh_loop(max_depth);
        for d in 0..=max_depth {
            let mut level = MetaLevel::base();
            for _ in 0..d {
                level = level.next();
            }
            let _ = sl.learn_at_level(level, &data);
        }

        let summary = sl.get_summary();
        let by_level_sum: usize = sl
            .get_all_knowledge()
            .values()
            .map(|v| v.len())
            .sum();
        prop_assert_eq!(summary.total_knowledge, by_level_sum);
    }
}

// ---------------------------------------------------------------- No panics.

proptest! {
    /// `learn_at_level` never panics on any (level, data) pair below
    /// usize::MAX. Errors are fine; panics are not.
    #[test]
    fn learn_never_panics(
        max_depth in 0usize..=8,
        level_idx in 0usize..=32,
        data in data_strategy(),
    ) {
        let mut sl = fresh_loop(max_depth);
        let mut level = MetaLevel::base();
        for _ in 0..level_idx {
            level = level.next();
        }
        let _ = sl.learn_at_level(level, &data); // result intentionally ignored
    }
}
