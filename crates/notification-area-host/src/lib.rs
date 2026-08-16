use std::collections::{BTreeMap, BTreeSet, VecDeque};

use shell_provider_protocol::{
    IconKey, NotificationEvent, NotificationEventKind, NotificationHostHealth,
    NotificationHostResponse, NotificationMutation, NotificationSnapshot, RegisteredIcon, Validate,
};

pub const MAX_CLIENTS: usize = 64;
pub const MAX_ICONS: usize = 256;
pub const MAX_EVENTS: usize = 512;

#[derive(Clone, Debug, Default)]
pub struct NotificationRegistry {
    clients: BTreeSet<String>,
    icons: BTreeMap<IconKey, RegisteredIcon>,
    generation: u64,
}

impl NotificationRegistry {
    pub fn apply(&mut self, mutation: NotificationMutation) -> NotificationHostResponse {
        if let Err(error) = mutation.validate() {
            return NotificationHostResponse::Rejected(error.to_string());
        }
        match mutation {
            NotificationMutation::RegisterClient { client_id } => {
                if !self.clients.contains(&client_id) && self.clients.len() >= MAX_CLIENTS {
                    return NotificationHostResponse::Rejected("client-capacity".into());
                }
                let changed = self.clients.insert(client_id);
                self.accepted(changed)
            }
            NotificationMutation::Add { icon } => {
                if !self.clients.contains(&icon.key.client_id) {
                    return NotificationHostResponse::Rejected("client-not-registered".into());
                }
                if !self.icons.contains_key(&icon.key) && self.icons.len() >= MAX_ICONS {
                    return NotificationHostResponse::Rejected("icon-capacity".into());
                }
                let changed = self
                    .icons
                    .get(&icon.key)
                    .is_none_or(|current| icon.generation > current.generation);
                if changed {
                    self.icons.insert(icon.key.clone(), icon);
                }
                self.accepted(changed)
            }
            NotificationMutation::Modify { icon } => {
                let changed = self
                    .icons
                    .get(&icon.key)
                    .is_some_and(|current| icon.generation > current.generation);
                if changed {
                    self.icons.insert(icon.key.clone(), icon);
                }
                self.accepted(changed)
            }
            NotificationMutation::Delete { key, generation } => {
                let changed = self
                    .icons
                    .get(&key)
                    .is_some_and(|current| generation >= current.generation);
                if changed {
                    self.icons.remove(&key);
                }
                self.accepted(changed)
            }
            NotificationMutation::Focus { key, generation } => {
                let changed = self
                    .icons
                    .get(&key)
                    .is_some_and(|current| generation >= current.generation);
                self.accepted(changed)
            }
            NotificationMutation::Disconnect { client_id } => {
                let client = self.clients.remove(&client_id);
                let before = self.icons.len();
                self.icons.retain(|key, _| key.client_id != client_id);
                self.accepted(client || before != self.icons.len())
            }
            NotificationMutation::Snapshot => NotificationHostResponse::Snapshot(self.snapshot()),
            NotificationMutation::Health => {
                NotificationHostResponse::Health(NotificationHostHealth {
                    healthy: true,
                    clients: self.clients.len(),
                    icons: self.icons.len(),
                    capacity: MAX_ICONS,
                })
            }
        }
    }

    pub fn snapshot(&self) -> NotificationSnapshot {
        NotificationSnapshot {
            generation: self.generation,
            icons: self.icons.values().cloned().collect(),
        }
    }

    fn accepted(&mut self, changed: bool) -> NotificationHostResponse {
        if changed {
            self.generation = self.generation.saturating_add(1);
        }
        NotificationHostResponse::Accepted {
            changed,
            generation: self.generation,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NotificationEventQueue {
    events: VecDeque<NotificationEvent>,
    capacity: usize,
}

impl Default for NotificationEventQueue {
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
            capacity: MAX_EVENTS,
        }
    }
}

impl NotificationEventQueue {
    pub fn push(&mut self, event: NotificationEvent) -> bool {
        if event.kind == NotificationEventKind::Hover
            && let Some(existing) = self.events.iter_mut().rev().find(|existing| {
                existing.key == event.key && existing.kind == NotificationEventKind::Hover
            })
        {
            *existing = event;
            return true;
        }
        if self.events.len() >= self.capacity {
            if matches!(
                event.kind,
                NotificationEventKind::Activate | NotificationEventKind::Context
            ) {
                if let Some(index) = self
                    .events
                    .iter()
                    .position(|existing| existing.kind == NotificationEventKind::Hover)
                {
                    self.events.remove(index);
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        self.events.push_back(event);
        true
    }

    pub fn pop(&mut self) -> Option<NotificationEvent> {
        self.events.pop_front()
    }
    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_provider_protocol::NotificationIcon;

    fn icon(generation: u64) -> RegisteredIcon {
        RegisteredIcon {
            key: IconKey {
                client_id: "client".into(),
                icon_id: 1,
            },
            generation,
            icon: NotificationIcon {
                owner_id: "client".into(),
                icon_id: 1,
                tooltip: "Tip".into(),
                visible: true,
                icon: None,
            },
            always_visible: false,
        }
    }

    #[test]
    fn lifecycle_stale_generation_disconnect_and_snapshot_are_deterministic() {
        let mut registry = NotificationRegistry::default();
        registry.apply(NotificationMutation::RegisterClient {
            client_id: "client".into(),
        });
        registry.apply(NotificationMutation::Add { icon: icon(2) });
        assert!(matches!(
            registry.apply(NotificationMutation::Modify { icon: icon(1) }),
            NotificationHostResponse::Accepted { changed: false, .. }
        ));
        assert_eq!(registry.snapshot().icons[0].generation, 2);
        registry.apply(NotificationMutation::Disconnect {
            client_id: "client".into(),
        });
        assert!(registry.snapshot().icons.is_empty());
    }
}
