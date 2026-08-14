use std::collections::VecDeque;

use crate::ShellEvent;

pub const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueDomain {
    Desktop,
    Window,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueuePush {
    Enqueued,
    Coalesced,
    Overflowed(QueueDomain),
    Backpressure(ShellEvent),
}

#[derive(Clone, Debug)]
pub struct BoundedEventQueue {
    capacity: usize,
    events: VecDeque<ShellEvent>,
    desktop_overflow_pending: bool,
    window_overflow_pending: bool,
    max_depth: usize,
}

impl BoundedEventQueue {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "event queue capacity must be non-zero");
        Self {
            capacity,
            events: VecDeque::with_capacity(capacity),
            desktop_overflow_pending: false,
            window_overflow_pending: false,
            max_depth: 0,
        }
    }

    pub fn push(&mut self, event: ShellEvent) -> QueuePush {
        if let Some(domain) = coalescing_domain(&event)
            && let Some(existing) = self
                .events
                .iter_mut()
                .rev()
                .find(|queued| coalescing_domain(queued) == Some(domain))
        {
            *existing = event;
            return QueuePush::Coalesced;
        }
        if self.events.len() < self.capacity {
            self.events.push_back(event);
            self.max_depth = self.max_depth.max(self.events.len());
            return QueuePush::Enqueued;
        }
        if let Some(domain) = coalescing_domain(&event) {
            self.mark_overflow(domain);
            QueuePush::Overflowed(domain)
        } else {
            QueuePush::Backpressure(event)
        }
    }

    pub fn pop(&mut self) -> Option<ShellEvent> {
        if self.desktop_overflow_pending {
            self.desktop_overflow_pending = false;
            return Some(ShellEvent::DesktopOverflow);
        }
        if self.window_overflow_pending {
            self.window_overflow_pending = false;
            return Some(ShellEvent::WindowOverflow);
        }
        self.events.pop_front()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty() && !self.desktop_overflow_pending && !self.window_overflow_pending
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub const fn max_depth(&self) -> usize {
        self.max_depth
    }

    fn mark_overflow(&mut self, domain: QueueDomain) {
        match domain {
            QueueDomain::Desktop => self.desktop_overflow_pending = true,
            QueueDomain::Window => self.window_overflow_pending = true,
        }
    }
}

fn coalescing_domain(event: &ShellEvent) -> Option<QueueDomain> {
    match event {
        ShellEvent::DesktopItemsChanged { .. } => Some(QueueDomain::Desktop),
        ShellEvent::WindowsChanged { .. } => Some(QueueDomain::Window),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::{CorrelationId, Generation, RequestId, TerminalKind};

    fn desktop(generation: u64) -> ShellEvent {
        ShellEvent::DesktopItemsChanged {
            request_id: RequestId(generation),
            generation: Generation(generation),
            items: BTreeSet::new(),
        }
    }

    fn windows(generation: u64) -> ShellEvent {
        ShellEvent::WindowsChanged {
            request_id: RequestId(generation),
            generation: Generation(generation),
            windows: BTreeMap::new(),
        }
    }

    #[test]
    fn coalescing_is_deterministic_and_identity_domain_specific() {
        let mut queue = BoundedEventQueue::new(4);
        assert_eq!(queue.push(desktop(1)), QueuePush::Enqueued);
        assert_eq!(queue.push(windows(1)), QueuePush::Enqueued);
        assert_eq!(queue.push(desktop(2)), QueuePush::Coalesced);
        assert_eq!(queue.len(), 2);
        assert!(matches!(
            queue.pop(),
            Some(ShellEvent::DesktopItemsChanged {
                generation: Generation(2),
                ..
            })
        ));
        assert!(matches!(
            queue.pop(),
            Some(ShellEvent::WindowsChanged {
                generation: Generation(1),
                ..
            })
        ));
    }

    #[test]
    fn terminal_is_never_dropped_when_queue_is_full() {
        let mut queue = BoundedEventQueue::new(1);
        queue.push(ShellEvent::DesktopOverflow);
        let terminal = ShellEvent::RequestTerminal {
            request_id: RequestId(1),
            correlation_id: CorrelationId(1),
            generation: Generation(1),
            terminal: TerminalKind::Succeeded,
        };
        assert_eq!(
            queue.push(terminal.clone()),
            QueuePush::Backpressure(terminal)
        );
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn storm_never_exceeds_bound_and_emits_explicit_overflow() {
        let mut queue = BoundedEventQueue::new(8);
        for generation in 0..10_000 {
            let event = if generation % 2 == 0 {
                desktop(generation)
            } else {
                windows(generation)
            };
            queue.push(event);
            assert!(queue.len() <= queue.capacity());
        }
        assert!(queue.max_depth() <= 8);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn full_protected_queue_turns_dropped_delta_into_overflow_event() {
        let mut queue = BoundedEventQueue::new(1);
        assert_eq!(queue.push(ShellEvent::DesktopOverflow), QueuePush::Enqueued);
        assert_eq!(
            queue.push(windows(1)),
            QueuePush::Overflowed(QueueDomain::Window)
        );
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop(), Some(ShellEvent::WindowOverflow));
        assert_eq!(queue.pop(), Some(ShellEvent::DesktopOverflow));
    }
}
