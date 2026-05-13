//! Fuzz target: nanosecond-scheduler's `schedule` + `next_task`
//! interleavings.
//!
//! Drives a randomized sequence of (schedule, pop, clear) operations
//! over a `RealtimeScheduler<u8>` and asserts only "did not panic /
//! did not OOM". Ordering invariants live in the proptest baseline
//! at `crates/nanosecond-scheduler/tests/proptest_queue_order.rs`;
//! this target chases the long tail (operation interleavings with
//! many queue-full encounters, rapid clear-then-fill cycles, etc.).
//!
//! Run with:
//!
//!   cargo +nightly fuzz run scheduler_event_loop

#![no_main]

use libfuzzer_sys::fuzz_target;
use midstreamer_scheduler::{
    Deadline, Priority, RealtimeScheduler, SchedulerConfig, SchedulingPolicy,
};

fn priority(b: u8) -> Priority {
    match b % 5 {
        0 => Priority::Critical,
        1 => Priority::High,
        2 => Priority::Medium,
        3 => Priority::Low,
        _ => Priority::Background,
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // First byte is the max_queue_size (clamped to [1, 32]).
    let cap = (data[0] as usize % 32).max(1);
    let sched = RealtimeScheduler::<u8>::new(SchedulerConfig {
        policy: SchedulingPolicy::FixedPriority,
        max_queue_size: cap,
        enable_rt_scheduling: false,
        cpu_affinity: None,
    });

    // Each subsequent byte is an op: low 2 bits = op code,
    // upper bits = data for that op.
    for &b in data[1..].iter().take(1024) {
        let op = b & 0b11;
        let arg = b >> 2;
        match op {
            // schedule(payload=arg, deadline=arg micros, priority=arg)
            0 => {
                let _ = sched.schedule(
                    arg,
                    Deadline::from_micros(arg as u64 + 1),
                    priority(arg),
                );
            }
            // pop
            1 => {
                let _ = sched.next_task();
            }
            // clear
            2 => sched.clear(),
            // queue_size (read-only; just don't panic)
            _ => {
                let _ = sched.queue_size();
            }
        }
    }
});
