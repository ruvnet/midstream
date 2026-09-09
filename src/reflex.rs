//! Realtime reflex control for two-speed agents.
//!
//! The reflex path can acknowledge, pause, cancel, and forward events, but it
//! cannot authorize external effects. Privileged actions remain behind RVM and
//! the deliberative reasoning path.

use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReflexEventKind {
    Interrupt,
    Backchannel,
    Cancel,
    Observation,
    CommitBoundary,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflexReceipt {
    pub sequence: u64,
    pub disposition: ReflexDisposition,
    pub actions: ReflexActions,
    pub authority: &'static str,
    pub queued: usize,
    pub cancelled_work_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReasonerHandoff {
    pub latest_sequence: u64,
    pub queued_events: Vec<ReflexEvent>,
    pub cancelled_work_units: u64,
    pub authority: &'static str,
}

#[derive(Clone, Debug)]
pub struct ReflexController {
    max_queue: usize,
    latest_sequence: u64,
    queue: VecDeque<ReflexEvent>,
    cancelled_work_units: u64,
    last_interrupt_sequence: Option<u64>,
}

impl ReflexController {
    pub fn new(max_queue: usize) -> Self {
        Self {
            max_queue: max_queue.max(1),
            latest_sequence: 0,
            queue: VecDeque::new(),
            cancelled_work_units: 0,
            last_interrupt_sequence: None,
        }
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

        if event.kind == ReflexEventKind::Interrupt
            && self
                .last_interrupt_sequence
                .is_some_and(|previous| event.sequence == previous + 1)
        {
            self.last_interrupt_sequence = Some(event.sequence);
            return self.receipt(
                event.sequence,
                ReflexDisposition::Coalesced,
                ReflexActions::two(ReflexAction::Acknowledge, ReflexAction::PauseReasoning),
            );
        }

        if self.queue.len() >= self.max_queue {
            return self.receipt(
                event.sequence,
                ReflexDisposition::Overflow,
                ReflexActions::one(ReflexAction::Noop),
            );
        }

        let actions = match event.kind {
            ReflexEventKind::Interrupt => {
                self.last_interrupt_sequence = Some(event.sequence);
                ReflexActions::two(ReflexAction::Acknowledge, ReflexAction::PauseReasoning)
            }
            ReflexEventKind::Backchannel => ReflexActions::one(ReflexAction::Acknowledge),
            ReflexEventKind::Cancel => {
                self.cancelled_work_units = self
                    .cancelled_work_units
                    .saturating_add(in_flight_work_units);
                ReflexActions::two(ReflexAction::Acknowledge, ReflexAction::RequestCancel)
            }
            ReflexEventKind::Observation | ReflexEventKind::CommitBoundary => {
                ReflexActions::one(ReflexAction::ForwardToReasoner)
            }
        };

        let sequence = event.sequence;
        self.queue.push_back(event);
        self.receipt(sequence, ReflexDisposition::Accepted, actions)
    }

    pub fn drain_for_reasoner(&mut self) -> ReasonerHandoff {
        let queued_events = self.queue.drain(..).collect();
        ReasonerHandoff {
            latest_sequence: self.latest_sequence,
            queued_events,
            cancelled_work_units: self.cancelled_work_units,
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
            at_micros: sequence * 100,
            kind,
        }
    }

    #[test]
    fn interrupt_acknowledges_without_authority() {
        let mut controller = ReflexController::new(8);
        let receipt = controller.observe(event(1, ReflexEventKind::Interrupt), 12);
        assert_eq!(receipt.disposition, ReflexDisposition::Accepted);
        assert_eq!(receipt.authority, "none");
        assert_eq!(
            receipt.actions,
            ReflexActions::two(ReflexAction::Acknowledge, ReflexAction::PauseReasoning)
        );
    }

    #[test]
    fn stale_events_fail_closed() {
        let mut controller = ReflexController::new(8);
        controller.observe(event(2, ReflexEventKind::Observation), 0);
        let stale = controller.observe(event(1, ReflexEventKind::Cancel), 50);
        assert_eq!(stale.disposition, ReflexDisposition::Stale);
        assert_eq!(stale.cancelled_work_units, 0);
    }

    #[test]
    fn adjacent_interrupts_coalesce() {
        let mut controller = ReflexController::new(8);
        controller.observe(event(1, ReflexEventKind::Interrupt), 0);
        let second = controller.observe(event(2, ReflexEventKind::Interrupt), 0);
        assert_eq!(second.disposition, ReflexDisposition::Coalesced);
        assert_eq!(controller.queued(), 1);
    }

    #[test]
    fn cancellation_accounts_wasted_work() {
        let mut controller = ReflexController::new(8);
        let receipt = controller.observe(event(1, ReflexEventKind::Cancel), 7);
        assert_eq!(receipt.cancelled_work_units, 7);
        assert!(receipt.actions.contains(ReflexAction::RequestCancel));
    }

    #[test]
    fn queue_overflow_does_not_expand_state() {
        let mut controller = ReflexController::new(1);
        controller.observe(event(1, ReflexEventKind::Observation), 0);
        let overflow = controller.observe(event(2, ReflexEventKind::Backchannel), 0);
        assert_eq!(overflow.disposition, ReflexDisposition::Overflow);
        assert_eq!(controller.queued(), 1);
    }

    #[test]
    fn handoff_preserves_event_order_and_has_no_authority() {
        let mut controller = ReflexController::new(8);
        controller.observe(event(1, ReflexEventKind::Observation), 0);
        controller.observe(event(2, ReflexEventKind::Cancel), 3);
        let handoff = controller.drain_for_reasoner();
        assert_eq!(handoff.authority, "none");
        assert_eq!(handoff.cancelled_work_units, 3);
        assert_eq!(
            handoff
                .queued_events
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(controller.queued(), 0);
    }
}