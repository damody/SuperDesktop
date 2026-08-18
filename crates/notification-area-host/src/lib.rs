use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::{SystemTime, UNIX_EPOCH},
};

use platform_win::common::notify_icon_compat::NotifyIconIngress;

use shell_provider_protocol::{
    IconKey, NotificationEvent, NotificationEventKind, NotificationHostHealth,
    NotificationHostResponse, NotificationIcon, NotificationMutation, NotificationSnapshot,
    NotifyIconCompatibilityTerminal, NotifyIconIdentity, NotifyIconTerminalKind, RegisteredIcon,
    Validate,
};

pub const MAX_CLIENTS: usize = 64;
pub const MAX_ICONS: usize = 256;
pub const MAX_EVENTS: usize = 512;

pub struct NativeCompatibilityRegistry {
    pub registry: NotificationRegistry,
    host_generation: u64,
    native: BTreeMap<IconKey, shell_provider_protocol::OwnedNotifyIcon>,
}

impl Default for NativeCompatibilityRegistry {
    fn default() -> Self {
        Self {
            registry: NotificationRegistry::default(),
            host_generation: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |value| value.as_millis().max(1) as u64),
            native: BTreeMap::new(),
        }
    }
}

impl NativeCompatibilityRegistry {
    pub fn apply_ingress(&mut self, ingress: NotifyIconIngress) -> NotifyIconCompatibilityTerminal {
        let generation = self.registry.generation.saturating_add(1).max(1);
        let identity = NotifyIconIdentity {
            numeric_id: ingress.input.numeric_id,
            guid: ingress.input.guid,
        };
        if identity.validate().is_err()
            || platform_win::common::notify_icon_compat::validate_window_owner(
                ingress.input.window_identity,
                ingress.input.process_id,
                ingress.input.session_id,
            )
            .is_err()
        {
            return self.terminal(
                ingress,
                NotifyIconTerminalKind::InvalidRequest,
                None,
                "notify-icon-owner-or-identity-invalid",
            );
        }
        let client_id = format!(
            "native:{}:{}:{}",
            ingress.input.session_id, ingress.input.process_id, ingress.input.window_identity
        );
        let key = IconKey {
            client_id: client_id.clone(),
            icon_id: stable_icon_id(&identity),
        };
        let terminal = match ingress.message {
            0 | 1 => {
                let icon = match platform_win::common::notify_icon_compat::copy_notify_icon(
                    &ingress.input,
                    generation,
                    false,
                ) {
                    Ok(icon) => icon,
                    Err(message) => {
                        return self.terminal(
                            ingress,
                            NotifyIconTerminalKind::InvalidRequest,
                            None,
                            message,
                        );
                    }
                };
                if ingress.message == 0 {
                    let _ = self
                        .registry
                        .apply(NotificationMutation::RegisterClient { client_id });
                    let registered = registered_icon(&key, &icon);
                    let response = self
                        .registry
                        .apply(NotificationMutation::Add { icon: registered });
                    self.native.insert(key, icon);
                    terminal_kind(response)
                } else {
                    let response = self.registry.apply(NotificationMutation::Modify {
                        icon: registered_icon(&key, &icon),
                    });
                    if matches!(
                        response,
                        NotificationHostResponse::Accepted { changed: true, .. }
                    ) {
                        self.native.insert(key, icon);
                    }
                    terminal_kind(response)
                }
            }
            2 => {
                let response = self.registry.apply(NotificationMutation::Delete {
                    key: key.clone(),
                    generation,
                });
                if matches!(
                    response,
                    NotificationHostResponse::Accepted { changed: true, .. }
                ) {
                    self.native.remove(&key);
                }
                terminal_kind(response)
            }
            3 => terminal_kind(
                self.registry
                    .apply(NotificationMutation::Focus { key, generation }),
            ),
            4 => {
                if let Some(current) = self.native.get_mut(&key) {
                    if let Some(version) = ingress.input.requested_version {
                        current.callback.negotiated_version = version;
                    }
                    NotifyIconTerminalKind::Applied
                } else {
                    NotifyIconTerminalKind::NoChange
                }
            }
            _ => NotifyIconTerminalKind::InvalidRequest,
        };
        self.terminal(ingress, terminal, Some(generation), "")
    }

    fn terminal(
        &self,
        ingress: NotifyIconIngress,
        terminal: NotifyIconTerminalKind,
        icon_generation: Option<u64>,
        message: impl Into<String>,
    ) -> NotifyIconCompatibilityTerminal {
        NotifyIconCompatibilityTerminal {
            correlation_id: format!(
                "native:{}:{}",
                ingress.input.process_id, self.registry.generation
            ),
            host_generation: self.host_generation,
            icon_generation,
            terminal,
            message: message.into(),
        }
    }
}

fn registered_icon(
    key: &IconKey,
    icon: &shell_provider_protocol::OwnedNotifyIcon,
) -> RegisteredIcon {
    RegisteredIcon {
        key: key.clone(),
        generation: icon.generation,
        icon: NotificationIcon {
            owner_id: key.client_id.clone(),
            icon_id: key.icon_id,
            tooltip: icon.tooltip.clone(),
            visible: icon.visible,
            icon: icon.pixels.clone(),
        },
        always_visible: false,
    }
}

fn terminal_kind(response: NotificationHostResponse) -> NotifyIconTerminalKind {
    match response {
        NotificationHostResponse::Accepted { changed: true, .. } => NotifyIconTerminalKind::Applied,
        NotificationHostResponse::Accepted { changed: false, .. } => {
            NotifyIconTerminalKind::NoChange
        }
        NotificationHostResponse::Rejected(reason) if reason.contains("capacity") => {
            NotifyIconTerminalKind::Capacity
        }
        NotificationHostResponse::Rejected(_) => NotifyIconTerminalKind::InvalidRequest,
        _ => NotifyIconTerminalKind::InvalidRequest,
    }
}

fn stable_icon_id(identity: &NotifyIconIdentity) -> u32 {
    if identity.numeric_id != 0 {
        return identity.numeric_id;
    }
    identity
        .guid
        .unwrap_or([0; 16])
        .chunks_exact(4)
        .fold(0u32, |value, chunk| {
            value ^ u32::from_le_bytes(chunk.try_into().unwrap())
        })
        .max(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityAdmission {
    Preview,
    CommittedShell,
}

impl CompatibilityAdmission {
    pub const fn owns_shell_identity(self) -> bool {
        matches!(self, Self::CommittedShell)
    }

    pub fn from_process_args(args: impl IntoIterator<Item = String>) -> Self {
        if args.into_iter().any(|arg| arg == "--shell-notifyicon") {
            Self::CommittedShell
        } else {
            Self::Preview
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct NotificationRegistry {
    clients: BTreeSet<String>,
    icons: BTreeMap<IconKey, RegisteredIcon>,
    pub(crate) generation: u64,
    events: NotificationEventQueue,
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
            NotificationMutation::Event { event } => {
                if !self.icons.contains_key(&event.key) {
                    return NotificationHostResponse::Rejected("event-icon-not-registered".into());
                }
                if self.events.push(event) {
                    self.accepted(false)
                } else {
                    NotificationHostResponse::Rejected("event-capacity".into())
                }
            }
            NotificationMutation::DrainEvents { client_id } => {
                if !self.clients.contains(&client_id) {
                    return NotificationHostResponse::Rejected("client-not-registered".into());
                }
                NotificationHostResponse::Events(self.events.drain_client(&client_id))
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

    pub fn drain_client(&mut self, client_id: &str) -> Vec<NotificationEvent> {
        let mut retained = VecDeque::with_capacity(self.events.len());
        let mut drained = Vec::new();
        while let Some(event) = self.events.pop_front() {
            if event.key.client_id == client_id {
                drained.push(event);
            } else {
                retained.push_back(event);
            }
        }
        self.events = retained;
        drained
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_win::common::notify_icon_compat::{
        NotifyIconCopyInput, NotifyIconIngress, NotifyIconLayoutMatrix,
    };
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

    #[test]
    fn protected_events_round_trip_only_to_the_owning_client() {
        let mut registry = NotificationRegistry::default();
        registry.apply(NotificationMutation::RegisterClient {
            client_id: "client".into(),
        });
        registry.apply(NotificationMutation::Add { icon: icon(1) });
        let event = NotificationEvent {
            correlation_id: "activation".into(),
            key: IconKey {
                client_id: "client".into(),
                icon_id: 1,
            },
            kind: NotificationEventKind::Activate,
            admitted_unix_ms: 42,
        };
        assert!(matches!(
            registry.apply(NotificationMutation::Event {
                event: event.clone()
            }),
            NotificationHostResponse::Accepted { changed: false, .. }
        ));
        assert_eq!(
            registry.apply(NotificationMutation::DrainEvents {
                client_id: "client".into()
            }),
            NotificationHostResponse::Events(vec![event])
        );
        assert!(matches!(
            registry.apply(NotificationMutation::DrainEvents {
                client_id: "unknown".into()
            }),
            NotificationHostResponse::Rejected(_)
        ));
    }

    #[test]
    fn compatibility_identity_is_explicitly_shell_only() {
        assert!(!CompatibilityAdmission::Preview.owns_shell_identity());
        assert!(CompatibilityAdmission::CommittedShell.owns_shell_identity());
        assert_eq!(
            CompatibilityAdmission::from_process_args(Vec::<String>::new()),
            CompatibilityAdmission::Preview
        );
        assert_eq!(
            CompatibilityAdmission::from_process_args(["--shell-notifyicon".into()]),
            CompatibilityAdmission::CommittedShell
        );
    }

    #[test]
    fn native_add_modify_version_focus_delete_lifecycle_is_monotonic() {
        let Some((process_id, session_id, window_identity)) =
            platform_win::common::notify_icon_compat::current_console_owner()
        else {
            return;
        };
        let input = NotifyIconCopyInput {
            cb_size: NotifyIconLayoutMatrix::current().v4_size,
            flags: 1 | 4,
            process_id,
            session_id,
            window_identity,
            numeric_id: 55,
            guid: None,
            callback_message: 0x501,
            requested_version: Some(shell_provider_protocol::NotifyIconLayoutVersion::V4),
            tooltip_utf16: "Native fixture".encode_utf16().collect(),
            visible: true,
            borrowed_hicon: 0,
        };
        let mut compatibility = NativeCompatibilityRegistry::default();
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress {
                    message: 0,
                    input: input.clone()
                })
                .terminal,
            NotifyIconTerminalKind::Applied
        );
        assert_eq!(compatibility.registry.snapshot().icons.len(), 1);
        let mut modified = input.clone();
        modified.tooltip_utf16 = "Modified fixture".encode_utf16().collect();
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress {
                    message: 1,
                    input: modified
                })
                .terminal,
            NotifyIconTerminalKind::Applied
        );
        assert_eq!(
            compatibility.registry.snapshot().icons[0].icon.tooltip,
            "Modified fixture"
        );
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress {
                    message: 4,
                    input: input.clone()
                })
                .terminal,
            NotifyIconTerminalKind::Applied
        );
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress {
                    message: 3,
                    input: input.clone()
                })
                .terminal,
            NotifyIconTerminalKind::Applied
        );
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress {
                    message: 2,
                    input: input.clone()
                })
                .terminal,
            NotifyIconTerminalKind::Applied
        );
        assert!(compatibility.registry.snapshot().icons.is_empty());
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress { message: 2, input })
                .terminal,
            NotifyIconTerminalKind::NoChange
        );
    }
}
