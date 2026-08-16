use std::collections::{BTreeMap, BTreeSet};

use settings_store::TaskbarSettings;
use shell_core::{ApplicationId, WindowId};

use crate::TaskWindow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskGroup {
    pub application_id: ApplicationId,
    pub windows: Vec<WindowId>,
    pub pinned: bool,
    pub attention: bool,
    pub order: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PinChange {
    Pinned(ApplicationId),
    Unpinned(ApplicationId),
    Reordered(Vec<ApplicationId>),
}

#[derive(Clone, Debug, Default)]
pub struct GroupModel {
    groups: BTreeMap<ApplicationId, TaskGroup>,
    pin_order: Vec<ApplicationId>,
    next_order: u64,
}

impl GroupModel {
    pub fn from_settings(settings: &TaskbarSettings) -> Self {
        Self {
            pin_order: settings
                .pins
                .iter()
                .filter_map(|id| ApplicationId::new(id.clone()).ok())
                .collect(),
            ..Self::default()
        }
    }
    pub fn groups(&self) -> Vec<&TaskGroup> {
        let mut groups = self.groups.values().collect::<Vec<_>>();
        groups.sort_by_key(|g| {
            (
                pin_index(&self.pin_order, &g.application_id).unwrap_or(usize::MAX),
                g.order,
            )
        });
        groups
    }
    pub fn reconcile<'a>(&mut self, windows: impl IntoIterator<Item = &'a TaskWindow>) {
        let mut members: BTreeMap<ApplicationId, Vec<&TaskWindow>> = BTreeMap::new();
        for window in windows {
            members
                .entry(window.observation.application_id.clone())
                .or_default()
                .push(window)
        }
        let all: BTreeSet<_> = members
            .keys()
            .cloned()
            .chain(self.pin_order.iter().cloned())
            .collect();
        self.groups.retain(|id, _| all.contains(id));
        for id in all {
            let windows = members.remove(&id).unwrap_or_default();
            let attention = windows.iter().any(|w| w.observation.attention);
            let mut ids = windows
                .into_iter()
                .map(|w| (w.membership_order, w.observation.id.clone()))
                .collect::<Vec<_>>();
            ids.sort_by_key(|(order, _)| *order);
            let pinned = self.pin_order.contains(&id);
            let old_order = self.groups.get(&id).map(|g| g.order).unwrap_or_else(|| {
                self.next_order = self.next_order.saturating_add(1);
                self.next_order
            });
            self.groups.insert(
                id.clone(),
                TaskGroup {
                    application_id: id,
                    windows: ids.into_iter().map(|(_, id)| id).collect(),
                    pinned,
                    attention,
                    order: old_order,
                },
            );
        }
    }
    pub fn pin(&mut self, id: ApplicationId) -> PinChange {
        if !self.pin_order.contains(&id) {
            self.pin_order.push(id.clone())
        }
        PinChange::Pinned(id)
    }
    pub fn unpin(&mut self, id: &ApplicationId) -> PinChange {
        self.pin_order.retain(|item| item != id);
        PinChange::Unpinned(id.clone())
    }
    pub fn reorder(&mut self, ordered: Vec<ApplicationId>) -> Result<PinChange, &'static str> {
        let current: BTreeSet<_> = self.pin_order.iter().cloned().collect();
        let incoming: BTreeSet<_> = ordered.iter().cloned().collect();
        if current != incoming {
            return Err("pin-reorder-membership-drift");
        };
        self.pin_order = ordered.clone();
        Ok(PinChange::Reordered(ordered))
    }
    pub fn persist(&self) -> TaskbarSettings {
        TaskbarSettings {
            rows: 2,
            pins: self.pin_order.iter().map(ToString::to_string).collect(),
            ..TaskbarSettings::default()
        }
    }
}
fn pin_index(pins: &[ApplicationId], id: &ApplicationId) -> Option<usize> {
    pins.iter().position(|pin| pin == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WindowObservation;
    fn window(id: &str, app: &str, order: u64) -> TaskWindow {
        TaskWindow {
            observation: WindowObservation {
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
            },
            membership_order: order,
            content_revision: 0,
        }
    }
    #[test]
    fn identity_collision_and_snapshot_reorder_do_not_merge_or_reorder() {
        let a = window("same-title-1", "a", 1);
        let b = window("same-title-2", "b", 2);
        let mut m = GroupModel::default();
        m.reconcile([&a, &b]);
        assert_eq!(m.groups().len(), 2);
        let before = m
            .groups()
            .iter()
            .map(|g| g.application_id.clone())
            .collect::<Vec<_>>();
        m.reconcile([&b, &a]);
        assert_eq!(
            before,
            m.groups()
                .iter()
                .map(|g| g.application_id.clone())
                .collect::<Vec<_>>()
        )
    }
    #[test]
    fn pin_order_round_trips_and_rejects_membership_drift() {
        let mut m = GroupModel::default();
        m.pin(ApplicationId::new("b").unwrap());
        m.pin(ApplicationId::new("a").unwrap());
        let settings = m.persist();
        let mut restored = GroupModel::from_settings(&settings);
        assert_eq!(restored.persist(), settings);
        assert!(
            restored
                .reorder(vec![ApplicationId::new("missing").unwrap()])
                .is_err()
        )
    }
}
