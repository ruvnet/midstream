//! Bounded reflex arbitration. Stop controls never compete with the data queue.
//!
//! One controller belongs to one authenticated session. Source sequence numbers
//! are monotonic; caller timestamps are telemetry, not authorization. Receipts
//! request local control only. RVM remains the external effect boundary.

use std::collections::VecDeque;

const MAX_QUEUE: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReflexEventKind {
    Interrupt,
    Backchannel,
    Cancel,
    Observation,
    CommitBoundary,
}

impl ReflexEventKind {
    fn wire_name(self) -> &'static str {
        match self {
            Self::Interrupt => "Interrupt",
            Self::Backchannel => "Backchannel",
            Self::Cancel => "Cancel",
            Self::Observation => "Observation",
            Self::CommitBoundary => "CommitBoundary",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflexEvent {
    pub sequence: u64,
    pub at_micros: u64,
    pub kind: ReflexEventKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReflexAction {
    Acknowledge,
    PauseReasoning,
    RequestCancel,
    ForwardToReasoner,
    Noop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReflexActions {
    pub first: ReflexAction,
    pub second: Option<ReflexAction>,
}

impl ReflexActions {
    const fn one(first: ReflexAction) -> Self {
        Self {
            first,
            second: None,
        }
    }

    const fn two(first: ReflexAction, second: ReflexAction) -> Self {
        Self {
            first,
            second: Some(second),
        }
    }

    pub fn contains(&self, action: ReflexAction) -> bool {
        self.first == action || self.second == Some(action)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReflexDisposition {
    Accepted,
    Stale,
    Coalesced,
    Overflow,
    /// Control accepted in its dedicated latch while the data queue was full.
    Priority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflexReceipt {
    pub sequence: u64,
    pub disposition: ReflexDisposition,
    pub actions: ReflexActions,
    pub authority: &'static str,
    pub queued: usize,
    /// Estimated obsolete work, not proof that a remote provider stopped.
    pub cancelled_work_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReasonerHandoff {
    pub latest_sequence: u64,
    pub queued_events: Vec<ReflexEvent>,
    pub cancelled_work_units: u64,
    pub interrupt_sequence: Option<u64>,
    pub cancel_sequence: Option<u64>,
    pub dropped_events: u64,
    pub authority: &'static str,
}

impl ReasonerHandoff {
    /// Versioned JSON bridge. All u64 values use decimal strings to avoid loss
    /// above JavaScript's 53-bit integer range. Call only on authenticated IPC.
    pub fn to_wire_json(&self, session_id: &str) -> Result<String, &'static str> {
        if session_id.is_empty()
            || session_id.len() > 128
            || !session_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._:".contains(&c))
        {
            return Err("invalid session identity");
        }
        if self.authority != "none" || self.queued_events.len() > MAX_QUEUE {
            return Err("invalid handoff");
        }
        let events = self
            .queued_events
            .iter()
            .map(|event| {
                format!(
                    "{{\"sequence\":\"{}\",\"at_micros\":\"{}\",\"kind\":\"{}\"}}",
                    event.sequence,
                    event.at_micros,
                    event.kind.wire_name()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let latch =
            |value: Option<u64>| value.map_or_else(|| "null".to_string(), |v| format!("\"{v}\""));
        Ok(format!(
            "{{\"version\":1,\"authority\":\"none\",\"session_id\":\"{}\",\"latest_sequence\":\"{}\",\"queued_events\":[{}],\"interrupt_sequence\":{},\"cancel_sequence\":{},\"cancelled_work_units\":\"{}\",\"dropped_events\":\"{}\"}}",
            session_id, self.latest_sequence, events, latch(self.interrupt_sequence),
            latch(self.cancel_sequence), self.cancelled_work_units, self.dropped_events
        ))
    }
}

#[derive(Clone, Debug)]
pub struct ReflexController {
    max_queue: usize,
    latest_sequence: u64,
    queue: VecDeque<ReflexEvent>,
    cancelled_work_units: u64,
    interrupt_sequence: Option<u64>,
    cancel_sequence: Option<u64>,
    dropped_events: u64,
    cancellation_accounted: bool,
}

impl ReflexController {
    /// Preallocates once. Legacy constructor bounds invalid capacities to 1..4096.
    pub fn new(max_queue: usize) -> Self {
        let max_queue = max_queue.clamp(1, MAX_QUEUE);
        Self {
            max_queue,
            latest_sequence: 0,
            queue: VecDeque::with_capacity(max_queue),
            cancelled_work_units: 0,
            interrupt_sequence: None,
            cancel_sequence: None,
            dropped_events: 0,
            cancellation_accounted: false,
        }
    }

    /// Called by the trusted host only after the preceding generation is quiescent.
    pub fn begin_reasoning(&mut self) {
        self.cancellation_accounted = false;
    }

    pub fn observe(&mut self, event: ReflexEvent, in_flight_work_units: u64) -> ReflexReceipt {
        if event.sequence <= self.latest_sequence {
            return self.receipt(
                event.sequence,
                ReflexDisposition::Stale,
                ReflexActions::one(ReflexAction::Noop),
            );
        }
        self.latest_sequence = event.sequence;
        let control = matches!(
            event.kind,
            ReflexEventKind::Interrupt | ReflexEventKind::Cancel
        );
        // Apply controls BEFORE bounded data admission. These latches survive drain.
        let actions = match event.kind {
            ReflexEventKind::Interrupt => {
                self.interrupt_sequence = Some(event.sequence);
                ReflexActions::two(ReflexAction::Acknowledge, ReflexAction::PauseReasoning)
            }
            ReflexEventKind::Cancel => {
                self.cancel_sequence = Some(event.sequence);
                ReflexActions::two(ReflexAction::Acknowledge, ReflexAction::RequestCancel)
            }
            ReflexEventKind::Backchannel => ReflexActions::one(ReflexAction::Acknowledge),
            ReflexEventKind::Observation | ReflexEventKind::CommitBoundary => {
                ReflexActions::one(ReflexAction::ForwardToReasoner)
            }
        };
        if control && !self.cancellation_accounted {
            self.cancelled_work_units = self
                .cancelled_work_units
                .saturating_add(in_flight_work_units);
            self.cancellation_accounted = true;
        }
        // Only replace a still-pending adjacent interrupt, never one already drained.
        if event.kind == ReflexEventKind::Interrupt {
            if let Some(tail) = self.queue.back_mut() {
                if tail.kind == ReflexEventKind::Interrupt
                    && tail.sequence.checked_add(1) == Some(event.sequence)
                {
                    let sequence = event.sequence;
                    *tail = event;
                    return self.receipt(sequence, ReflexDisposition::Coalesced, actions);
                }
            }
        }
        if self.queue.len() == self.max_queue {
            if control {
                return self.receipt(event.sequence, ReflexDisposition::Priority, actions);
            }
            self.dropped_events = self.dropped_events.saturating_add(1);
            return self.receipt(
                event.sequence,
                ReflexDisposition::Overflow,
                ReflexActions::one(ReflexAction::Noop),
            );
        }
        let sequence = event.sequence;
        self.queue.push_back(event);
        self.receipt(sequence, ReflexDisposition::Accepted, actions)
    }

    pub fn drain_for_reasoner(&mut self) -> ReasonerHandoff {
        ReasonerHandoff {
            latest_sequence: self.latest_sequence,
            queued_events: self.queue.drain(..).collect(),
            cancelled_work_units: self.cancelled_work_units,
            interrupt_sequence: self.interrupt_sequence,
            cancel_sequence: self.cancel_sequence,
            dropped_events: self.dropped_events,
            authority: "none",
        }
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    fn receipt(
        &self,
        sequence: u64,
        disposition: ReflexDisposition,
        actions: ReflexActions,
    ) -> ReflexReceipt {
        ReflexReceipt {
            sequence,
            disposition,
            actions,
            authority: "none",
            queued: self.queue.len(),
            cancelled_work_units: self.cancelled_work_units,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(sequence: u64, kind: ReflexEventKind) -> ReflexEvent {
        ReflexEvent {
            sequence,
            at_micros: sequence,
            kind,
        }
    }

    #[test]
    fn interrupt_acknowledges_without_authority() {
        let r = ReflexController::new(8).observe(event(1, ReflexEventKind::Interrupt), 12);
        assert_eq!(r.authority, "none");
        assert!(r.actions.contains(ReflexAction::PauseReasoning));
    }

    #[test]
    fn stale_events_fail_closed() {
        let mut c = ReflexController::new(8);
        c.observe(event(2, ReflexEventKind::Observation), 0);
        assert_eq!(
            c.observe(event(1, ReflexEventKind::Cancel), 50).disposition,
            ReflexDisposition::Stale
        );
        assert_eq!(c.drain_for_reasoner().cancel_sequence, None);
    }

    #[test]
    fn adjacent_interrupts_coalesce_to_latest_pending_event() {
        let mut c = ReflexController::new(8);
        c.observe(event(1, ReflexEventKind::Interrupt), 0);
        assert_eq!(
            c.observe(event(2, ReflexEventKind::Interrupt), 0)
                .disposition,
            ReflexDisposition::Coalesced
        );
        let h = c.drain_for_reasoner();
        assert_eq!(h.queued_events.len(), 1);
        assert_eq!(h.queued_events[0].sequence, 2);
    }

    #[test]
    fn cancellation_is_accounted_once_per_generation() {
        let mut c = ReflexController::new(8);
        c.observe(event(1, ReflexEventKind::Cancel), 7);
        assert_eq!(
            c.observe(event(2, ReflexEventKind::Cancel), 7)
                .cancelled_work_units,
            7
        );
        c.begin_reasoning();
        assert_eq!(
            c.observe(event(3, ReflexEventKind::Cancel), 2)
                .cancelled_work_units,
            9
        );
    }

    #[test]
    fn queue_overflow_is_explicit_and_bounded() {
        let mut c = ReflexController::new(1);
        c.observe(event(1, ReflexEventKind::Observation), 0);
        assert_eq!(
            c.observe(event(2, ReflexEventKind::Backchannel), 0)
                .disposition,
            ReflexDisposition::Overflow
        );
        assert_eq!(c.queued(), 1);
        assert_eq!(c.drain_for_reasoner().dropped_events, 1);
    }

    #[test]
    fn handoff_preserves_event_order() {
        let mut c = ReflexController::new(8);
        c.observe(event(1, ReflexEventKind::Observation), 0);
        c.observe(event(2, ReflexEventKind::Cancel), 3);
        let h = c.drain_for_reasoner();
        assert_eq!(
            h.queued_events
                .iter()
                .map(|e| e.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(h.authority, "none");
        assert_eq!(c.queued(), 0);
    }

    #[test]
    fn saturated_queue_never_loses_cancel() {
        let mut c = ReflexController::new(1);
        c.observe(event(1, ReflexEventKind::Observation), 0);
        let r = c.observe(event(2, ReflexEventKind::Cancel), 9);
        assert_eq!(r.disposition, ReflexDisposition::Priority);
        assert!(r.actions.contains(ReflexAction::RequestCancel));
        let h = c.drain_for_reasoner();
        assert_eq!(h.cancel_sequence, Some(2));
        assert_eq!(h.dropped_events, 0);
    }

    #[test]
    fn saturated_queue_never_loses_interrupt() {
        let mut c = ReflexController::new(1);
        c.observe(event(1, ReflexEventKind::Observation), 0);
        let r = c.observe(event(2, ReflexEventKind::Interrupt), 9);
        assert!(r.actions.contains(ReflexAction::PauseReasoning));
        assert_eq!(c.drain_for_reasoner().interrupt_sequence, Some(2));
    }

    #[test]
    fn draining_does_not_coalesce_future_interrupts() {
        let mut c = ReflexController::new(1);
        c.observe(event(1, ReflexEventKind::Interrupt), 0);
        c.drain_for_reasoner();
        assert_eq!(
            c.observe(event(2, ReflexEventKind::Interrupt), 0)
                .disposition,
            ReflexDisposition::Accepted
        );
        assert_eq!(c.queued(), 1);
    }

    #[test]
    fn u64_boundary_does_not_wrap() {
        let mut c = ReflexController::new(2);
        c.observe(event(u64::MAX - 1, ReflexEventKind::Interrupt), 0);
        c.observe(event(u64::MAX, ReflexEventKind::Interrupt), 0);
        assert_eq!(
            c.observe(event(0, ReflexEventKind::Cancel), 0).disposition,
            ReflexDisposition::Stale
        );
        assert_eq!(c.drain_for_reasoner().interrupt_sequence, Some(u64::MAX));
    }

    #[test]
    fn invalid_capacity_is_hard_bounded() {
        let mut c = ReflexController::new(usize::MAX);
        for n in 1..=5000 {
            c.observe(event(n, ReflexEventKind::Observation), 0);
        }
        assert_eq!(c.queued(), MAX_QUEUE);
        assert_eq!(c.drain_for_reasoner().dropped_events, 904);
    }

    #[test]
    fn saturated_burst_preserves_both_controls() {
        let mut c = ReflexController::new(1);
        c.observe(event(1, ReflexEventKind::Observation), 0);
        for n in 2..=1001 {
            let kind = if n % 2 == 0 {
                ReflexEventKind::Interrupt
            } else {
                ReflexEventKind::Cancel
            };
            c.observe(event(n, kind), 8);
        }
        let h = c.drain_for_reasoner();
        assert_eq!(h.interrupt_sequence, Some(1000));
        assert_eq!(h.cancel_sequence, Some(1001));
        assert_eq!(h.cancelled_work_units, 8);
        assert_eq!(h.queued_events.len(), 1);
    }

    #[test]
    fn wire_is_versioned_and_uses_decimal_strings() {
        let mut c = ReflexController::new(1);
        c.observe(event(1, ReflexEventKind::Observation), 0);
        c.observe(event(2, ReflexEventKind::Cancel), 8);
        let wire = c.drain_for_reasoner().to_wire_json("s").unwrap();
        assert_eq!(wire, "{\"version\":1,\"authority\":\"none\",\"session_id\":\"s\",\"latest_sequence\":\"2\",\"queued_events\":[{\"sequence\":\"1\",\"at_micros\":\"1\",\"kind\":\"Observation\"}],\"interrupt_sequence\":null,\"cancel_sequence\":\"2\",\"cancelled_work_units\":\"8\",\"dropped_events\":\"0\"}");
    }

    #[test]
    fn wire_rejects_session_injection() {
        let h = ReflexController::new(1).drain_for_reasoner();
        for s in ["", "../other", "x\"", "a\nb"] {
            assert!(h.to_wire_json(s).is_err());
        }
    }
}
