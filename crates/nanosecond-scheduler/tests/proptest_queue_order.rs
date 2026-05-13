//! Property-based tests for `midstreamer-scheduler`. Implements
//! ADR-0038 for the scheduler crate.
//!
//! 8 properties × 256 cases each = ~2,000 generated schedule
//! interleavings exercised per CI run, plus replay of any committed
//! `proptest-regressions/proptest_queue_order.txt` counterexamples.
//!
//! Invariants asserted:
//!
//!   schedule()/queue_size:
//!     * After N schedule() calls (under max_queue_size), queue_size == N.
//!     * QueueFull is returned at exactly the (max_queue_size+1)-th call.
//!     * After clear(), queue_size == 0.
//!
//!   next_task() ordering:
//!     * `next_task` always returns the highest-priority task first.
//!     * Among equal-priority tasks, earlier-deadline wins.
//!     * Pop sequence is sorted by (priority desc, deadline asc).
//!     * After N pops, queue_size decreases by N (down to 0).
//!     * Empty queue returns None.
//!
//! ScheduledTask::cmp orders by (Priority desc) then (deadline.absolute_time
//! asc), so BinaryHeap (max-heap) yields the same order on pop.

use midstreamer_scheduler::{
    Deadline, Priority, RealtimeScheduler, SchedulerConfig, SchedulingPolicy,
};
use proptest::prelude::*;
use std::time::Duration;

/// Map a 0..=4 selector to one of the 5 declared Priority levels.
fn priority(idx: u8) -> Priority {
    match idx % 5 {
        0 => Priority::Critical,
        1 => Priority::High,
        2 => Priority::Medium,
        3 => Priority::Low,
        _ => Priority::Background,
    }
}

/// Generator: a vector of `(priority_idx, deadline_micros, payload)`.
fn task_specs() -> impl Strategy<Value = Vec<(u8, u64, u32)>> {
    proptest::collection::vec(
        (0u8..=4, 1u64..=1_000_000, any::<u32>()),
        0..=32,
    )
}

fn fresh_scheduler(max_queue_size: usize) -> RealtimeScheduler<u32> {
    RealtimeScheduler::new(SchedulerConfig {
        policy: SchedulingPolicy::FixedPriority,
        max_queue_size,
        enable_rt_scheduling: false,
        cpu_affinity: None,
    })
}

// ---------------------------------------------------------------- queue size.

proptest! {
    /// queue_size == schedule()-count under capacity.
    #[test]
    fn queue_size_equals_schedule_count(specs in task_specs()) {
        let s = fresh_scheduler(specs.len().max(1));
        for (p, micros, payload) in &specs {
            s.schedule(*payload, Deadline::from_micros(*micros), priority(*p)).unwrap();
        }
        prop_assert_eq!(s.queue_size(), specs.len());
    }

    /// `clear()` empties the queue.
    #[test]
    fn clear_empties_queue(specs in task_specs()) {
        let s = fresh_scheduler(specs.len().max(1));
        for (p, micros, payload) in &specs {
            s.schedule(*payload, Deadline::from_micros(*micros), priority(*p)).unwrap();
        }
        s.clear();
        prop_assert_eq!(s.queue_size(), 0);
    }
}

// ---------------------------------------------------------------- max_queue_size.

proptest! {
    /// The (max_queue_size+1)-th call to schedule() returns QueueFull.
    /// Calls 1..=max_queue_size succeed.
    #[test]
    fn queue_full_at_capacity(cap in 1usize..=8) {
        let s = fresh_scheduler(cap);
        // Fill exactly to capacity. Every push must succeed.
        for i in 0..cap {
            s.schedule(i as u32, Deadline::from_micros(1_000), Priority::Medium)
                .expect("under-cap schedule should not fail");
        }
        prop_assert_eq!(s.queue_size(), cap);

        // The next push must fail with QueueFull.
        let err = s.schedule(99, Deadline::from_micros(1_000), Priority::Medium);
        prop_assert!(err.is_err(), "over-cap schedule unexpectedly succeeded");
    }
}

// ---------------------------------------------------------------- next_task ordering.

proptest! {
    /// Popping N tasks decreases queue_size by N down to 0.
    #[test]
    fn next_task_decreases_size_monotonically(specs in task_specs()) {
        let s = fresh_scheduler(specs.len().max(1));
        for (p, micros, payload) in &specs {
            s.schedule(*payload, Deadline::from_micros(*micros), priority(*p)).unwrap();
        }

        let mut remaining = s.queue_size();
        while remaining > 0 {
            let popped = s.next_task();
            prop_assert!(popped.is_some(), "pop returned None with non-empty queue");
            remaining -= 1;
            prop_assert_eq!(s.queue_size(), remaining);
        }
        prop_assert!(s.next_task().is_none(), "empty queue must return None");
    }

    /// Every scheduled task is popped exactly once before the queue
    /// becomes empty.
    #[test]
    fn next_task_pops_every_scheduled_exactly_once(specs in task_specs()) {
        let s = fresh_scheduler(specs.len().max(1));
        let expected = specs.len();
        for (p, micros, payload) in &specs {
            s.schedule(*payload, Deadline::from_micros(*micros), priority(*p)).unwrap();
        }

        let mut popped = 0;
        while s.next_task().is_some() {
            popped += 1;
            prop_assert!(popped <= expected, "popped more tasks than scheduled");
        }
        prop_assert_eq!(popped, expected);
    }

    /// Pop sequence is sorted by (priority desc by `as_i32()`,
    /// then deadline asc by `absolute_time`).
    ///
    /// Initial drafts of this test surfaced two bugs in
    /// `ScheduledTask::cmp` (PR #59): the priority comparison was
    /// inverted, and the within-priority deadline tie-break was
    /// inverted. Both fixed in PR #60 (this commit pair); the
    /// strict ordering check is now re-enabled to lock in the
    /// documented "Higher priority first, earlier deadline first"
    /// behaviour.
    #[test]
    fn next_task_emits_priority_desc_then_deadline_asc(specs in task_specs()) {
        let s = fresh_scheduler(specs.len().max(1));
        for (p, micros, payload) in &specs {
            s.schedule(*payload, Deadline::from_micros(*micros), priority(*p)).unwrap();
        }

        let mut last: Option<(i32, std::time::Instant)> = None;
        while let Some(task) = s.next_task() {
            let curr = (task.priority.as_i32(), task.deadline.absolute_time);
            if let Some(prev) = last {
                prop_assert!(
                    prev.0 >= curr.0,
                    "priority order violated: prev={} curr={}", prev.0, curr.0
                );
                if prev.0 == curr.0 {
                    prop_assert!(
                        prev.1 <= curr.1,
                        "deadline order violated within priority {}", curr.0
                    );
                }
            }
            last = Some(curr);
        }
    }

    /// Empty queue returns None from next_task.
    #[test]
    fn empty_queue_yields_none(_unit in proptest::strategy::Just(())) {
        let s = fresh_scheduler(8);
        prop_assert!(s.next_task().is_none());
    }
}

// ---------------------------------------------------------------- Deadline arithmetic.

proptest! {
    /// `Deadline::from_now(d)` is always in the future when d > 0.
    #[test]
    fn deadline_from_now_is_future(micros in 1u64..=1_000_000) {
        let d = Deadline::from_now(Duration::from_micros(micros));
        // The deadline must be at least `micros - some_jitter` away;
        // we accept anything > 0 to allow for small wall-clock delays
        // between the call and the assertion.
        prop_assert!(!d.is_passed(), "freshly-created deadline already passed");
    }

    /// `Deadline::from_micros(0)` is in the past or exactly now.
    #[test]
    fn deadline_zero_is_passed(_unit in proptest::strategy::Just(())) {
        // Constructing a "deadline 0 microseconds from now" then
        // immediately checking it should report passed=true on any
        // realistic system (the call itself takes more than 0ns).
        let d = Deadline::from_now(Duration::from_micros(0));
        prop_assert!(d.is_passed() || d.time_until() == Some(Duration::ZERO));
    }
}
