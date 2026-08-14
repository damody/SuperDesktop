use std::collections::{BTreeMap, VecDeque};

use shell_core::{Generation, ShellItemId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatcherKind {
    Created,
    Removed,
    Renamed,
    Metadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatcherDelta {
    pub identity: ShellItemId,
    pub generation: Generation,
    pub kind: WatcherKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WatcherPush {
    Enqueued,
    Coalesced,
    Overflowed,
}

#[derive(Clone, Debug)]
pub struct DesktopWatcherQueue {
    capacity: usize,
    queue: VecDeque<ShellItemId>,
    latest: BTreeMap<ShellItemId, WatcherDelta>,
    overflow_pending: bool,
    max_depth: usize,
}

impl DesktopWatcherQueue {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            queue: VecDeque::new(),
            latest: BTreeMap::new(),
            overflow_pending: false,
            max_depth: 0,
        }
    }
    pub fn push(&mut self, delta: WatcherDelta) -> WatcherPush {
        if let Some(existing) = self.latest.get_mut(&delta.identity) {
            *existing = delta;
            return WatcherPush::Coalesced;
        }
        if self.queue.len() >= self.capacity {
            self.overflow_pending = true;
            return WatcherPush::Overflowed;
        }
        self.queue.push_back(delta.identity.clone());
        self.latest.insert(delta.identity.clone(), delta);
        self.max_depth = self.max_depth.max(self.queue.len());
        WatcherPush::Enqueued
    }
    pub fn take_overflow(&mut self) -> bool {
        std::mem::take(&mut self.overflow_pending)
    }
    pub fn pop(&mut self, current_generation: Generation) -> Option<WatcherDelta> {
        while let Some(identity) = self.queue.pop_front() {
            let delta = self.latest.remove(&identity)?;
            if delta.generation == current_generation {
                return Some(delta);
            }
        }
        None
    }
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty() && !self.overflow_pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn delta(id: &str, g: u64, kind: WatcherKind) -> WatcherDelta {
        WatcherDelta {
            identity: ShellItemId::new(id).unwrap(),
            generation: Generation(g),
            kind,
        }
    }
    #[test]
    fn rename_storm_coalesces_and_stays_bounded() {
        let mut queue = DesktopWatcherQueue::new(8);
        for _ in 0..10_000 {
            queue.push(delta("same", 1, WatcherKind::Renamed));
        }
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.max_depth(), 1);
        assert_eq!(queue.pop(Generation(1)).unwrap().kind, WatcherKind::Renamed);
    }
    #[test]
    fn overflow_is_single_flight_and_stale_deltas_are_suppressed() {
        let mut queue = DesktopWatcherQueue::new(1);
        queue.push(delta("old", 1, WatcherKind::Created));
        assert_eq!(
            queue.push(delta("new", 2, WatcherKind::Created)),
            WatcherPush::Overflowed
        );
        assert!(queue.take_overflow());
        assert!(!queue.take_overflow());
        assert!(queue.pop(Generation(2)).is_none());
    }
}
