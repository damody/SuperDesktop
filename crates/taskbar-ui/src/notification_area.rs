use std::collections::{BTreeMap, BTreeSet};

use shell_provider_protocol::{
    IconData, IconKey, NotificationEvent, NotificationEventKind, NotificationSnapshot,
    RegisteredIcon, Validate,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationPlacement {
    Visible,
    Overflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationAccessibleNode {
    pub key: IconKey,
    pub stable_id: String,
    pub name: String,
    pub role: &'static str,
    pub focused: bool,
    pub placement: NotificationPlacement,
    pub icon: Option<IconData>,
}

#[derive(Clone, Debug, Default)]
pub struct NotificationAreaModel {
    generation: u64,
    icons: BTreeMap<IconKey, RegisteredIcon>,
    visible: Vec<IconKey>,
    overflow: Vec<IconKey>,
    overflow_open: bool,
    focus: Option<IconKey>,
    latency_samples_ms: Vec<u64>,
    provider_available: bool,
}

impl NotificationAreaModel {
    pub fn apply_snapshot(
        &mut self,
        snapshot: NotificationSnapshot,
        visible_capacity: usize,
    ) -> bool {
        if snapshot.generation < self.generation || snapshot.validate().is_err() {
            return false;
        }
        let next_generation = snapshot.generation;
        let next_icons = snapshot
            .icons
            .into_iter()
            .map(|icon| (icon.key.clone(), icon))
            .collect();
        let changed = !self.provider_available
            || self.generation != next_generation
            || self.icons != next_icons;
        self.generation = next_generation;
        self.icons = next_icons;
        self.provider_available = true;
        self.relayout(visible_capacity);
        changed
    }

    pub fn provider_unavailable(&mut self) {
        self.provider_available = false;
        self.icons.clear();
        self.visible.clear();
        self.overflow.clear();
        self.focus = None;
        self.overflow_open = false;
    }

    pub fn provider_available(&self) -> bool {
        self.provider_available
    }
    pub fn visible(&self) -> &[IconKey] {
        &self.visible
    }
    pub fn overflow(&self) -> &[IconKey] {
        &self.overflow
    }
    pub fn overflow_open(&self) -> bool {
        self.overflow_open
    }

    pub fn open_overflow(&mut self) {
        self.overflow_open = !self.overflow.is_empty();
        self.focus = self.overflow.first().cloned();
    }

    pub fn dismiss_overflow(&mut self) {
        self.overflow_open = false;
        self.focus = None
    }

    pub fn event(
        &self,
        key: &IconKey,
        kind: NotificationEventKind,
        correlation_id: String,
        now_ms: u64,
    ) -> Option<NotificationEvent> {
        self.icons.contains_key(key).then(|| NotificationEvent {
            correlation_id,
            key: key.clone(),
            kind,
            admitted_unix_ms: now_ms,
        })
    }

    pub fn complete_event(&mut self, event: &NotificationEvent, completed_ms: u64) {
        self.latency_samples_ms
            .push(completed_ms.saturating_sub(event.admitted_unix_ms));
        if self.latency_samples_ms.len() > 1024 {
            self.latency_samples_ms.remove(0);
        }
    }

    pub fn latency_p95_ms(&self) -> Option<u64> {
        let mut samples = self.latency_samples_ms.clone();
        if samples.is_empty() {
            return None;
        }
        samples.sort_unstable();
        Some(samples[((samples.len() * 95).div_ceil(100)).saturating_sub(1)])
    }

    pub fn accessible_nodes(&self) -> Vec<NotificationAccessibleNode> {
        self.visible
            .iter()
            .map(|key| (key, NotificationPlacement::Visible))
            .chain(
                self.overflow
                    .iter()
                    .map(|key| (key, NotificationPlacement::Overflow)),
            )
            .filter_map(|(key, placement)| {
                let icon = self.icons.get(key)?;
                Some(NotificationAccessibleNode {
                    key: key.clone(),
                    stable_id: format!("notification:{}:{}", key.client_id, key.icon_id),
                    name: icon.icon.tooltip.clone(),
                    role: "button",
                    focused: self.focus.as_ref() == Some(key),
                    placement,
                    icon: icon.icon.icon.clone(),
                })
            })
            .collect()
    }

    fn relayout(&mut self, visible_capacity: usize) {
        let mut keys: Vec<_> = self.icons.keys().cloned().collect();
        keys.sort_by(|left, right| {
            let left_always = self.icons.get(left).is_some_and(|icon| icon.always_visible);
            let right_always = self
                .icons
                .get(right)
                .is_some_and(|icon| icon.always_visible);
            right_always.cmp(&left_always).then_with(|| left.cmp(right))
        });
        let mut visible_set = BTreeSet::new();
        for key in keys
            .iter()
            .filter(|key| self.icons.get(*key).is_some_and(|icon| icon.always_visible))
        {
            visible_set.insert(key.clone());
        }
        for key in &keys {
            if visible_set.len() >= visible_capacity {
                break;
            }
            if self
                .icons
                .get(key)
                .is_some_and(|icon| icon.icon.visible || icon.always_visible)
            {
                visible_set.insert(key.clone());
            }
        }
        self.visible = keys
            .iter()
            .filter(|key| visible_set.contains(*key))
            .cloned()
            .collect();
        self.overflow = keys
            .into_iter()
            .filter(|key| !visible_set.contains(key))
            .collect();
        if self
            .focus
            .as_ref()
            .is_some_and(|key| !self.icons.contains_key(key))
        {
            self.focus = None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_provider_protocol::NotificationIcon;

    fn icon(client: &str, id: u32, always: bool) -> RegisteredIcon {
        RegisteredIcon {
            key: IconKey {
                client_id: client.into(),
                icon_id: id,
            },
            generation: 1,
            icon: NotificationIcon {
                owner_id: client.into(),
                icon_id: id,
                tooltip: format!("Icon {id}"),
                visible: true,
                icon: Some(IconData {
                    width: 1,
                    height: 1,
                    rgba: vec![255, 0, 0, 255],
                }),
            },
            always_visible: always,
        }
    }

    #[test]
    fn snapshot_overflow_input_latency_and_unavailable_are_truthful() {
        let mut model = NotificationAreaModel::default();
        assert!(model.apply_snapshot(
            NotificationSnapshot {
                generation: 1,
                icons: vec![icon("a", 1, true), icon("b", 2, false)],
                notifications: Vec::new(),
                windows_events: Default::default(),
            },
            1
        ));
        assert_eq!(model.visible().len(), 1);
        assert_eq!(model.overflow().len(), 1);
        model.open_overflow();
        let nodes = model.accessible_nodes();
        assert!(nodes.iter().any(|node| node.focused));
        assert!(nodes.iter().all(|node| node.icon.is_some()));
        let key = model.overflow()[0].clone();
        let event = model
            .event(&key, NotificationEventKind::Activate, "event".into(), 1_000)
            .unwrap();
        model.complete_event(&event, 1_042);
        assert_eq!(model.latency_p95_ms(), Some(42));
        model.provider_unavailable();
        assert!(!model.provider_available());
        assert!(model.visible().is_empty());
    }

    #[test]
    fn accessible_nodes_are_a_complete_unique_snapshot_across_placements() {
        let mut model = NotificationAreaModel::default();
        let mut hidden = icon("hidden", 4, false);
        hidden.icon.visible = false;
        assert!(model.apply_snapshot(
            NotificationSnapshot {
                generation: 1,
                icons: vec![
                    icon("a", 1, true),
                    icon("b", 2, false),
                    icon("c", 3, false),
                    hidden,
                ],
                notifications: Vec::new(),
                windows_events: Default::default(),
            },
            2,
        ));
        assert_eq!(model.visible().len(), 2);
        assert_eq!(model.overflow().len(), 2);
        let nodes = model.accessible_nodes();
        assert_eq!(nodes.len(), 4);
        let unique = nodes
            .iter()
            .map(|node| node.key.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), nodes.len());
        assert!(
            nodes
                .iter()
                .any(|node| node.placement == NotificationPlacement::Visible)
        );
        assert!(
            nodes
                .iter()
                .any(|node| node.placement == NotificationPlacement::Overflow)
        );
        assert!(nodes.iter().any(|node| {
            node.key.client_id == "hidden" && node.placement == NotificationPlacement::Overflow
        }));
    }
}
