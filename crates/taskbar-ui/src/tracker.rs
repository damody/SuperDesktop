use std::collections::{BTreeMap, VecDeque};

use shell_core::{ApplicationId, WindowId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Eligibility {
    Eligible,
    Invisible,
    ToolWindow,
    Cloaked,
    OwnedTransient,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowObservation {
    pub id: WindowId,
    pub application_id: ApplicationId,
    pub title: String,
    pub visible: bool,
    pub tool_window: bool,
    pub cloaked: bool,
    pub owned_transient: bool,
    pub minimized: bool,
    pub foreground: bool,
    pub attention: bool,
}

impl WindowObservation {
    pub fn eligibility(&self) -> Eligibility {
        if !self.visible {
            Eligibility::Invisible
        } else if self.tool_window {
            Eligibility::ToolWindow
        } else if self.cloaked {
            Eligibility::Cloaked
        } else if self.owned_transient {
            Eligibility::OwnedTransient
        } else {
            Eligibility::Eligible
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskWindow {
    pub observation: WindowObservation,
    pub membership_order: u64,
    pub content_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnedWindowEvent {
    Created(WindowObservation),
    Destroyed(WindowId),
    Title(WindowId, String),
    Attention(WindowId, bool),
    Overflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackerPush {
    Queued,
    Coalesced,
    OverflowRequested,
}

#[derive(Debug)]
pub struct WindowTracker {
    generation: u64,
    next_order: u64,
    capacity: usize,
    refresh_pending: bool,
    windows: BTreeMap<WindowId, TaskWindow>,
    queue: VecDeque<(u64, OwnedWindowEvent)>,
    max_depth: usize,
}

impl WindowTracker {
    pub fn new(capacity: usize) -> Result<Self, &'static str> {
        if capacity == 0 {
            return Err("window-tracker-capacity-zero");
        };
        Ok(Self {
            generation: 0,
            next_order: 0,
            capacity,
            refresh_pending: false,
            windows: BTreeMap::new(),
            queue: VecDeque::new(),
            max_depth: 0,
        })
    }
    pub fn windows(&self) -> &BTreeMap<WindowId, TaskWindow> {
        &self.windows
    }
    pub const fn max_depth(&self) -> usize {
        self.max_depth
    }
    pub fn push(&mut self, generation: u64, event: OwnedWindowEvent) -> TrackerPush {
        if generation < self.generation {
            return TrackerPush::Coalesced;
        }
        let key = event_key(&event);
        if let Some(index) = self
            .queue
            .iter()
            .position(|(_, queued)| event_key(queued) == key)
        {
            self.queue[index] = (generation, event);
            return TrackerPush::Coalesced;
        }
        if self.queue.len() >= self.capacity {
            self.queue.clear();
            if !self.refresh_pending {
                self.refresh_pending = true;
                self.queue
                    .push_back((generation, OwnedWindowEvent::Overflow));
            }
            self.max_depth = self.max_depth.max(self.queue.len());
            return TrackerPush::OverflowRequested;
        }
        self.queue.push_back((generation, event));
        self.max_depth = self.max_depth.max(self.queue.len());
        TrackerPush::Queued
    }
    pub fn drain(&mut self) {
        while let Some((generation, event)) = self.queue.pop_front() {
            if generation < self.generation {
                continue;
            }
            match event {
                OwnedWindowEvent::Created(observation) => self.upsert(observation),
                OwnedWindowEvent::Destroyed(id) => {
                    self.windows.remove(&id);
                }
                OwnedWindowEvent::Title(id, title) => {
                    if let Some(window) = self.windows.get_mut(&id) {
                        window.observation.title = title;
                        window.content_revision = window.content_revision.saturating_add(1)
                    }
                }
                OwnedWindowEvent::Attention(id, value) => {
                    if let Some(window) = self.windows.get_mut(&id) {
                        window.observation.attention = value;
                        window.content_revision = window.content_revision.saturating_add(1)
                    }
                }
                OwnedWindowEvent::Overflow => {}
            }
        }
    }
    pub fn reconcile(
        &mut self,
        generation: u64,
        snapshot: impl IntoIterator<Item = WindowObservation>,
    ) -> bool {
        if generation < self.generation {
            return false;
        }
        self.generation = generation;
        let observations: Vec<_> = snapshot
            .into_iter()
            .filter(|o| o.eligibility() == Eligibility::Eligible)
            .collect();
        let mut next = BTreeMap::new();
        for observation in observations {
            let window = if let Some(existing) = self.windows.remove(&observation.id) {
                TaskWindow {
                    observation,
                    membership_order: existing.membership_order,
                    content_revision: existing.content_revision.saturating_add(1),
                }
            } else {
                self.next_order = self.next_order.saturating_add(1);
                TaskWindow {
                    observation,
                    membership_order: self.next_order,
                    content_revision: 0,
                }
            };
            next.insert(window.observation.id.clone(), window);
        }
        self.windows = next;
        self.refresh_pending = false;
        self.queue.retain(|(g, _)| *g >= generation);
        true
    }
    fn upsert(&mut self, observation: WindowObservation) {
        if observation.eligibility() != Eligibility::Eligible {
            self.windows.remove(&observation.id);
            return;
        }
        if let Some(window) = self.windows.get_mut(&observation.id) {
            window.observation = observation;
            window.content_revision = window.content_revision.saturating_add(1)
        } else {
            self.next_order = self.next_order.saturating_add(1);
            self.windows.insert(
                observation.id.clone(),
                TaskWindow {
                    observation,
                    membership_order: self.next_order,
                    content_revision: 0,
                },
            );
        }
    }
}

fn event_key(event: &OwnedWindowEvent) -> String {
    match event {
        OwnedWindowEvent::Created(o) => o.id.to_string(),
        OwnedWindowEvent::Destroyed(id)
        | OwnedWindowEvent::Title(id, _)
        | OwnedWindowEvent::Attention(id, _) => id.to_string(),
        OwnedWindowEvent::Overflow => "overflow".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(id: &str, app: &str) -> WindowObservation {
        WindowObservation {
            id: WindowId::new(id).unwrap(),
            application_id: ApplicationId::new(app).unwrap(),
            title: id.into(),
            visible: true,
            tool_window: false,
            cloaked: false,
            owned_transient: false,
            minimized: false,
            foreground: false,
            attention: false,
        }
    }
    #[test]
    fn filters_all_excluded_window_classes() {
        for mutate in 0..4 {
            let mut o = observation("w", "a");
            match mutate {
                0 => o.visible = false,
                1 => o.tool_window = true,
                2 => o.cloaked = true,
                _ => o.owned_transient = true,
            };
            assert_ne!(o.eligibility(), Eligibility::Eligible)
        }
    }
    #[test]
    fn authoritative_snapshot_preserves_membership_order_across_content_churn() {
        let mut t = WindowTracker::new(3).unwrap();
        t.reconcile(1, [observation("w1", "a"), observation("w2", "b")]);
        let order = t.windows()[&WindowId::new("w1").unwrap()].membership_order;
        let mut changed = observation("w1", "a");
        changed.title = "new".into();
        t.reconcile(2, [observation("w2", "b"), changed]);
        assert_eq!(
            t.windows()[&WindowId::new("w1").unwrap()].membership_order,
            order
        )
    }
    #[test]
    fn storm_is_bounded_overflow_single_flight_and_stale_snapshot_rejected() {
        let mut t = WindowTracker::new(2).unwrap();
        for n in 0..20 {
            let _ = t.push(
                2,
                OwnedWindowEvent::Title(WindowId::new(format!("w{n}")).unwrap(), "x".into()),
            );
        }
        assert!(t.max_depth() <= 2);
        assert!(t.reconcile(3, [observation("final", "a")]));
        assert!(!t.reconcile(2, [observation("stale", "b")]));
        assert!(t.windows().contains_key(&WindowId::new("final").unwrap()))
    }
}
