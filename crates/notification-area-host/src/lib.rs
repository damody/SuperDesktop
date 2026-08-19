use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use platform_win::common::notify_icon_compat::NotifyIconIngress;
use platform_win::common::windows_notification_events::{
    WindowsNotificationEventSource, parse_windows_notification_id,
};

use shell_provider_protocol::{
    IconKey, MAX_TEXT_BYTES, NotificationEvent, NotificationEventKind, NotificationHostHealth,
    NotificationHostResponse, NotificationIcon, NotificationMutation, NotificationSnapshot,
    NotifyIconCompatibilityTerminal, NotifyIconIdentity, NotifyIconTerminalKind, OwnedNotification,
    OwnedNotificationContent, RegisteredIcon, Validate, WindowsNotificationAccess,
    WindowsNotificationChange, WindowsNotificationEventStatus,
};

pub const MAX_CLIENTS: usize = 64;
pub const MAX_ICONS: usize = 256;
pub const MAX_EVENTS: usize = 512;
pub const MAX_NOTIFICATION_HISTORY: usize = 100;
pub const MAX_COMPLETED_CALLBACKS: usize = 512;
pub const MAX_CALLBACK_AGE_MS: u64 = 5_000;
const WINDOWS_NOTIFICATION_RECONCILE_INTERVAL: Duration = Duration::from_secs(5);

pub struct NativeCompatibilityRegistry {
    pub registry: NotificationRegistry,
    host_generation: u64,
    native: BTreeMap<IconKey, shell_provider_protocol::OwnedNotifyIcon>,
    completed_callbacks: VecDeque<String>,
    windows_events: Option<WindowsNotificationEventSource>,
    last_windows_reconcile: Option<Instant>,
    windows_events_enabled: bool,
}

impl Default for NativeCompatibilityRegistry {
    fn default() -> Self {
        Self {
            registry: NotificationRegistry::default(),
            host_generation: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |value| value.as_millis().max(1) as u64),
            native: BTreeMap::new(),
            completed_callbacks: VecDeque::new(),
            windows_events: None,
            last_windows_reconcile: None,
            windows_events_enabled: false,
        }
    }
}

impl NativeCompatibilityRegistry {
    /// Production constructor. `Default` deliberately remains isolated for deterministic tests.
    pub fn with_windows_notification_events() -> Self {
        let mut result = Self {
            windows_events_enabled: true,
            ..Self::default()
        };
        match WindowsNotificationEventSource::new() {
            Ok(source) => {
                result.registry.windows_events = source.access_status();
                result.windows_events = Some(source);
                result.reconcile_windows_notifications(true);
            }
            Err(reason) => result.registry.windows_events = unavailable_windows_status(reason),
        }
        result
    }

    pub const fn host_generation(&self) -> u64 {
        self.host_generation
    }

    pub fn apply_mutation(
        &mut self,
        mutation: NotificationMutation,
        now_unix_ms: u64,
    ) -> NotificationHostResponse {
        self.reconcile_windows_notifications(false);
        match mutation {
            NotificationMutation::DismissNotification {
                ref notification_id,
                expected_generation,
            } if parse_windows_notification_id(notification_id).is_some() => {
                self.dismiss_windows_notification(notification_id, expected_generation)
            }
            NotificationMutation::ClearNotifications {
                expected_generation,
            } if self.registry.has_windows_notifications() => {
                self.clear_windows_notifications(expected_generation)
            }
            NotificationMutation::Event { event } => self.deliver_event(event, now_unix_ms),
            NotificationMutation::CancelEvent { correlation_id } => {
                if !self.completed_callbacks.contains(&correlation_id) {
                    if self.completed_callbacks.len() >= MAX_COMPLETED_CALLBACKS {
                        self.completed_callbacks.pop_front();
                    }
                    self.completed_callbacks.push_back(correlation_id.clone());
                }
                self.registry
                    .apply(NotificationMutation::CancelEvent { correlation_id })
            }
            NotificationMutation::Disconnect { client_id } => {
                self.native.retain(|key, _| key.client_id != client_id);
                self.registry
                    .apply(NotificationMutation::Disconnect { client_id })
            }
            other => self.registry.apply(other),
        }
    }

    fn reconcile_windows_notifications(&mut self, force: bool) {
        if !self.windows_events_enabled {
            return;
        }
        let due = self
            .last_windows_reconcile
            .is_none_or(|last| last.elapsed() >= WINDOWS_NOTIFICATION_RECONCILE_INTERVAL);
        let should_retry_access = due
            && self.windows_events.as_ref().is_none_or(|source| {
                source.access_status().access != WindowsNotificationAccess::Allowed
            });
        if should_retry_access {
            match WindowsNotificationEventSource::new_with_access_request(false) {
                Ok(source) => {
                    self.registry.windows_events = source.access_status();
                    self.windows_events = Some(source);
                }
                Err(reason) => {
                    self.registry
                        .set_windows_status(unavailable_windows_status(reason));
                    self.last_windows_reconcile = Some(Instant::now());
                    return;
                }
            }
        }
        let Some(source) = self.windows_events.as_ref() else {
            return;
        };
        if !force && !due && !source.take_dirty() {
            return;
        }
        self.last_windows_reconcile = Some(Instant::now());
        match source.snapshot() {
            Ok(batch) => self
                .registry
                .replace_windows_notifications(batch.notifications, batch.status),
            Err(reason) => self
                .registry
                .set_windows_status(unavailable_windows_status(reason)),
        }
    }

    fn dismiss_windows_notification(
        &mut self,
        notification_id: &str,
        expected_generation: u64,
    ) -> NotificationHostResponse {
        self.reconcile_windows_notifications(true);
        if expected_generation != self.registry.generation {
            return self.registry.accepted(false);
        }
        let Some(native_id) = parse_windows_notification_id(notification_id) else {
            return NotificationHostResponse::Rejected("windows-notification-id-invalid".into());
        };
        let Some(source) = self.windows_events.as_ref() else {
            return NotificationHostResponse::Rejected(
                "windows-notification-events-unavailable".into(),
            );
        };
        if !self.registry.windows_events.synchronized
            || !self
                .registry
                .notifications
                .iter()
                .any(|item| item.notification_id == notification_id)
        {
            return NotificationHostResponse::Rejected("windows-notification-not-current".into());
        }
        if let Err(reason) = source.remove(native_id) {
            return NotificationHostResponse::Rejected(reason);
        }
        self.reconcile_windows_notifications(true);
        let changed = !self
            .registry
            .notifications
            .iter()
            .any(|item| item.notification_id == notification_id);
        if changed {
            NotificationHostResponse::Accepted {
                changed: true,
                generation: self.registry.generation,
            }
        } else {
            NotificationHostResponse::Rejected("windows-notification-remove-not-confirmed".into())
        }
    }

    fn clear_windows_notifications(
        &mut self,
        expected_generation: u64,
    ) -> NotificationHostResponse {
        self.reconcile_windows_notifications(true);
        if expected_generation != self.registry.generation {
            return self.registry.accepted(false);
        }
        let Some(source) = self.windows_events.as_ref() else {
            return NotificationHostResponse::Rejected(
                "windows-notification-events-unavailable".into(),
            );
        };
        if !self.registry.windows_events.synchronized {
            return NotificationHostResponse::Rejected("windows-notification-not-current".into());
        }
        if let Err(reason) = source.clear() {
            return NotificationHostResponse::Rejected(reason);
        }
        self.reconcile_windows_notifications(true);
        if self.registry.has_windows_notifications() {
            return NotificationHostResponse::Rejected(
                "windows-notification-clear-not-confirmed".into(),
            );
        }
        self.registry
            .apply(NotificationMutation::ClearNotifications {
                expected_generation: self.registry.generation,
            })
    }

    pub fn reconcile_dead_clients(&mut self) -> usize {
        let dead = self
            .native
            .iter()
            .filter_map(|(key, icon)| {
                platform_win::common::notify_icon_compat::validate_window_owner(
                    icon.client.window_identity as isize,
                    icon.client.process_id,
                    icon.client.session_id,
                )
                .is_err()
                .then_some(key.client_id.clone())
            })
            .collect::<BTreeSet<_>>();
        for client_id in &dead {
            self.native.retain(|key, _| key.client_id != *client_id);
            let _ = self.registry.apply(NotificationMutation::Disconnect {
                client_id: client_id.clone(),
            });
        }
        dead.len()
    }

    pub fn restart_generation(&mut self) {
        self.registry = NotificationRegistry::default();
        self.native.clear();
        self.completed_callbacks.clear();
        self.host_generation = self.host_generation.saturating_add(1).max(1);
        if let Some(source) = self.windows_events.as_ref() {
            self.registry.windows_events = source.access_status();
            self.reconcile_windows_notifications(true);
        }
    }

    fn deliver_event(
        &mut self,
        event: NotificationEvent,
        now_unix_ms: u64,
    ) -> NotificationHostResponse {
        if event.validate().is_err() {
            return NotificationHostResponse::Rejected("event-invalid".into());
        }
        if self.completed_callbacks.contains(&event.correlation_id) {
            return NotificationHostResponse::Accepted {
                changed: false,
                generation: self.registry.generation,
            };
        }
        if now_unix_ms.saturating_sub(event.admitted_unix_ms) > MAX_CALLBACK_AGE_MS {
            return NotificationHostResponse::Rejected("event-timeout".into());
        }
        let Some(icon) = self.native.get(&event.key) else {
            return NotificationHostResponse::Rejected("event-icon-not-registered".into());
        };
        if let Err(reason) =
            platform_win::common::notify_icon_compat::deliver_callback(icon, event.kind)
        {
            let client_id = event.key.client_id.clone();
            self.native.retain(|key, _| key.client_id != client_id);
            let _ = self
                .registry
                .apply(NotificationMutation::Disconnect { client_id });
            return NotificationHostResponse::Rejected(reason.into());
        }
        if self.completed_callbacks.len() >= MAX_COMPLETED_CALLBACKS {
            self.completed_callbacks.pop_front();
        }
        self.completed_callbacks.push_back(event.correlation_id);
        NotificationHostResponse::Accepted {
            changed: false,
            generation: self.registry.generation,
        }
    }

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
                    let terminal = terminal_kind(response);
                    if terminal == NotifyIconTerminalKind::Applied {
                        self.native.insert(key.clone(), icon.clone());
                        self.admit_native_notification(&key, &icon, generation);
                    }
                    terminal
                } else {
                    let response = self.registry.apply(NotificationMutation::Modify {
                        icon: registered_icon(&key, &icon),
                    });
                    let terminal = terminal_kind(response);
                    if terminal == NotifyIconTerminalKind::Applied {
                        self.native.insert(key.clone(), icon.clone());
                        self.admit_native_notification(&key, &icon, generation);
                    }
                    terminal
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

    fn admit_native_notification(
        &mut self,
        key: &IconKey,
        icon: &shell_provider_protocol::OwnedNotifyIcon,
        generation: u64,
    ) {
        let Some(content) = icon.notification.as_ref() else {
            return;
        };
        let notification = owned_notification(key, icon, content, generation);
        let _ = self.registry.admit_notification(notification);
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
    notifications: VecDeque<OwnedNotification>,
    windows_events: WindowsNotificationEventStatus,
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
            NotificationMutation::CancelEvent { correlation_id } => {
                let changed = self.events.cancel(&correlation_id);
                self.accepted(changed)
            }
            NotificationMutation::DrainEvents { client_id } => {
                if !self.clients.contains(&client_id) {
                    return NotificationHostResponse::Rejected("client-not-registered".into());
                }
                NotificationHostResponse::Events(self.events.drain_client(&client_id))
            }
            NotificationMutation::DismissNotification {
                notification_id,
                expected_generation,
            } => {
                if expected_generation != self.generation {
                    return self.accepted(false);
                }
                let before = self.notifications.len();
                self.notifications
                    .retain(|notification| notification.notification_id != notification_id);
                self.accepted(before != self.notifications.len())
            }
            NotificationMutation::ClearNotifications {
                expected_generation,
            } => {
                if expected_generation != self.generation {
                    return self.accepted(false);
                }
                let changed = !self.notifications.is_empty();
                self.notifications.clear();
                self.accepted(changed)
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
            notifications: self.notifications.iter().cloned().collect(),
            windows_events: self.windows_events.clone(),
        }
    }

    fn has_windows_notifications(&self) -> bool {
        self.notifications.iter().any(|notification| {
            parse_windows_notification_id(&notification.notification_id).is_some()
        })
    }

    fn set_windows_status(&mut self, status: WindowsNotificationEventStatus) {
        if self.windows_events != status {
            self.windows_events = status;
            self.generation = self.generation.saturating_add(1);
        }
    }

    fn replace_windows_notifications(
        &mut self,
        windows: Vec<OwnedNotification>,
        status: WindowsNotificationEventStatus,
    ) {
        let mut merged = self
            .notifications
            .iter()
            .filter(|item| parse_windows_notification_id(&item.notification_id).is_none())
            .cloned()
            .chain(windows)
            .collect::<Vec<_>>();
        merged.sort_by(|left, right| {
            right
                .admitted_unix_ms
                .cmp(&left.admitted_unix_ms)
                .then_with(|| left.notification_id.cmp(&right.notification_id))
        });
        merged.dedup_by(|left, right| left.notification_id == right.notification_id);
        merged.truncate(MAX_NOTIFICATION_HISTORY);
        let merged = VecDeque::from(merged);
        if self.notifications != merged || self.windows_events != status {
            self.notifications = merged;
            self.windows_events = status;
            self.generation = self.generation.saturating_add(1);
        }
    }

    pub fn admit_notification(
        &mut self,
        notification: OwnedNotification,
    ) -> NotificationHostResponse {
        if notification.validate().is_err() {
            return NotificationHostResponse::Rejected("notification-invalid".into());
        }
        let duplicate = self.notifications.iter().any(|current| {
            current.key == notification.key
                && current.generation == notification.generation
                && current.title == notification.title
                && current.body == notification.body
        });
        if duplicate {
            return self.accepted(false);
        }
        if self.notifications.len() >= MAX_NOTIFICATION_HISTORY {
            self.notifications.pop_back();
        }
        self.notifications.push_front(notification);
        self.accepted(true)
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

fn unavailable_windows_status(reason: String) -> WindowsNotificationEventStatus {
    let mut reason = reason;
    if reason.len() > MAX_TEXT_BYTES {
        let mut end = MAX_TEXT_BYTES;
        while !reason.is_char_boundary(end) {
            end -= 1;
        }
        reason.truncate(end);
    }
    WindowsNotificationEventStatus {
        access: WindowsNotificationAccess::Unavailable,
        synchronized: false,
        last_change: WindowsNotificationChange::None,
        reason,
    }
}

fn owned_notification(
    key: &IconKey,
    icon: &shell_provider_protocol::OwnedNotifyIcon,
    content: &OwnedNotificationContent,
    generation: u64,
) -> OwnedNotification {
    let admitted_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |value| value.as_millis().max(1) as u64);
    let mut hash = 0xcbf29ce484222325u64;
    for byte in content.title.bytes().chain(content.body.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    OwnedNotification {
        notification_id: format!("notification:{generation}:{}:{hash:016x}", key.icon_id),
        key: key.clone(),
        application_label: if icon.tooltip.trim().is_empty() {
            "Application".into()
        } else {
            icon.tooltip.clone()
        },
        title: content.title.clone(),
        body: content.body.clone(),
        severity: content.severity,
        admitted_unix_ms,
        generation,
        icon: icon.pixels.clone(),
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

    pub fn cancel(&mut self, correlation_id: &str) -> bool {
        let before = self.events.len();
        self.events
            .retain(|event| event.correlation_id != correlation_id);
        self.events.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_win::common::notify_icon_compat::{
        NotifyIconCopyInput, NotifyIconIngress, NotifyIconLayoutMatrix,
    };
    use shell_provider_protocol::{NotificationIcon, NotificationSeverity};

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

    fn notification(index: u64) -> OwnedNotification {
        OwnedNotification {
            notification_id: format!("notification-{index}"),
            key: IconKey {
                client_id: "client".into(),
                icon_id: 1,
            },
            application_label: "Fixture".into(),
            title: format!("Title {index}"),
            body: format!("Body {index}"),
            severity: NotificationSeverity::Information,
            admitted_unix_ms: index + 1,
            generation: index + 1,
            icon: None,
        }
    }

    fn windows_notification(index: u64) -> OwnedNotification {
        let mut value = notification(index);
        value.notification_id = format!("windows:{index}");
        value.key.client_id = "windows-events".into();
        value
    }

    fn synchronized_windows_status() -> WindowsNotificationEventStatus {
        WindowsNotificationEventStatus {
            access: WindowsNotificationAccess::Allowed,
            synchronized: true,
            last_change: WindowsNotificationChange::Added,
            reason: String::new(),
        }
    }

    #[test]
    fn windows_reconciliation_preserves_local_history_is_bounded_and_generation_stable() {
        let mut registry = NotificationRegistry::default();
        registry.admit_notification(notification(500));
        let windows = (0..MAX_NOTIFICATION_HISTORY as u64)
            .map(windows_notification)
            .collect::<Vec<_>>();
        registry.replace_windows_notifications(windows.clone(), synchronized_windows_status());
        let first_generation = registry.generation;
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.notifications.len(), MAX_NOTIFICATION_HISTORY);
        assert!(
            snapshot
                .notifications
                .iter()
                .any(|item| item.notification_id == "notification-500")
        );
        assert!(snapshot.windows_events.synchronized);

        registry.replace_windows_notifications(windows, synchronized_windows_status());
        assert_eq!(registry.generation, first_generation);

        registry.set_windows_status(unavailable_windows_status("transient".into()));
        assert!(registry.has_windows_notifications());
        assert_eq!(
            registry.snapshot().windows_events.access,
            WindowsNotificationAccess::Unavailable
        );

        registry.replace_windows_notifications(Vec::new(), synchronized_windows_status());
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.notifications.len(), 1);
        assert_eq!(
            snapshot.notifications[0].notification_id,
            "notification-500"
        );
    }

    #[test]
    fn identity_manifests_match_and_declare_documented_notification_capability() {
        let package = include_str!("../../../packaging/windows-identity/AppxManifest.xml");
        let executable = include_str!("../resources/windows/notification-area-host.manifest.xml");
        for exact in [
            "SuperDesktop.WindowsShell",
            "CN=SuperDesktop",
            "NotificationAreaHost",
        ] {
            assert!(package.contains(exact));
            assert!(executable.contains(exact));
        }
        for required in [
            "uap3:Capability Name=\"userNotificationListener\"",
            "rescap:Capability Name=\"runFullTrust\"",
            "uap10:AllowExternalContent>true",
            "Executable=\"notification-area-host.exe\"",
        ] {
            assert!(
                package.contains(required),
                "missing identity declaration: {required}"
            );
        }
        for forbidden in ["ShellExperienceHost", "explorer.exe", "Notifications.db"] {
            assert!(!package.contains(forbidden));
            assert!(!executable.contains(forbidden));
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
    fn history_capacity_dedup_disconnect_dismiss_and_clear_are_authoritative() {
        let mut registry = NotificationRegistry::default();
        registry.apply(NotificationMutation::RegisterClient {
            client_id: "client".into(),
        });
        registry.apply(NotificationMutation::Add { icon: icon(1) });
        let first = notification(0);
        assert!(matches!(
            registry.admit_notification(first.clone()),
            NotificationHostResponse::Accepted { changed: true, .. }
        ));
        assert!(matches!(
            registry.admit_notification(first),
            NotificationHostResponse::Accepted { changed: false, .. }
        ));
        for index in 1..=MAX_NOTIFICATION_HISTORY as u64 {
            registry.admit_notification(notification(index));
        }
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.notifications.len(), MAX_NOTIFICATION_HISTORY);
        assert_eq!(
            snapshot.notifications[0].notification_id,
            "notification-100"
        );
        assert_eq!(
            snapshot.notifications.last().unwrap().notification_id,
            "notification-1"
        );

        registry.apply(NotificationMutation::Disconnect {
            client_id: "client".into(),
        });
        assert!(registry.snapshot().icons.is_empty());
        assert_eq!(
            registry.snapshot().notifications.len(),
            MAX_NOTIFICATION_HISTORY
        );

        let generation = registry.generation;
        assert!(matches!(
            registry.apply(NotificationMutation::DismissNotification {
                notification_id: "notification-100".into(),
                expected_generation: generation,
            }),
            NotificationHostResponse::Accepted { changed: true, .. }
        ));
        assert_eq!(registry.snapshot().notifications.len(), 99);
        assert!(matches!(
            registry.apply(NotificationMutation::ClearNotifications {
                expected_generation: generation,
            }),
            NotificationHostResponse::Accepted { changed: false, .. }
        ));
        let current = registry.generation;
        assert!(matches!(
            registry.apply(NotificationMutation::ClearNotifications {
                expected_generation: current,
            }),
            NotificationHostResponse::Accepted { changed: true, .. }
        ));
        assert!(registry.snapshot().notifications.is_empty());
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
            flags: 1 | 4 | 16,
            process_id,
            session_id,
            window_identity,
            numeric_id: 55,
            guid: None,
            callback_message: 0x501,
            requested_version: Some(shell_provider_protocol::NotifyIconLayoutVersion::V4),
            tooltip_utf16: "Native fixture".encode_utf16().collect(),
            info_utf16: "Native notification body".encode_utf16().collect(),
            info_title_utf16: "Native notification".encode_utf16().collect(),
            info_flags: 1,
            info_timeout_ms: 5_000,
            realtime: true,
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
        assert_eq!(compatibility.registry.snapshot().notifications.len(), 1);
        assert_eq!(
            compatibility.registry.snapshot().notifications[0].title,
            "Native notification"
        );
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
        assert!(!compatibility.registry.snapshot().notifications.is_empty());
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress { message: 2, input })
                .terminal,
            NotifyIconTerminalKind::NoChange
        );
    }

    #[test]
    fn native_callbacks_timeout_deduplicate_disconnect_and_restart_fail_closed() {
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
            numeric_id: 91,
            guid: None,
            callback_message: 0x5a1,
            requested_version: Some(shell_provider_protocol::NotifyIconLayoutVersion::V4),
            tooltip_utf16: "Callback fixture".encode_utf16().collect(),
            info_utf16: Vec::new(),
            info_title_utf16: Vec::new(),
            info_flags: 0,
            info_timeout_ms: 0,
            realtime: false,
            visible: true,
            borrowed_hicon: 0,
        };
        let mut compatibility = NativeCompatibilityRegistry::default();
        assert_eq!(
            compatibility
                .apply_ingress(NotifyIconIngress {
                    message: 0,
                    input: input.clone(),
                })
                .terminal,
            NotifyIconTerminalKind::Applied
        );
        let key = compatibility.registry.snapshot().icons[0].key.clone();
        let timed_out = NotificationEvent {
            correlation_id: "expired".into(),
            key: key.clone(),
            kind: NotificationEventKind::Activate,
            admitted_unix_ms: 1,
        };
        assert_eq!(
            compatibility.apply_mutation(
                NotificationMutation::Event { event: timed_out },
                MAX_CALLBACK_AGE_MS + 2,
            ),
            NotificationHostResponse::Rejected("event-timeout".into())
        );
        let event = NotificationEvent {
            correlation_id: "once".into(),
            key: key.clone(),
            kind: NotificationEventKind::Focus,
            admitted_unix_ms: 10,
        };
        assert!(matches!(
            compatibility.apply_mutation(
                NotificationMutation::Event {
                    event: event.clone(),
                },
                11,
            ),
            NotificationHostResponse::Accepted { changed: false, .. }
        ));
        assert!(matches!(
            compatibility.apply_mutation(NotificationMutation::Event { event }, 12),
            NotificationHostResponse::Accepted { changed: false, .. }
        ));
        assert!(matches!(
            compatibility.apply_mutation(
                NotificationMutation::CancelEvent {
                    correlation_id: "cancelled".into(),
                },
                12,
            ),
            NotificationHostResponse::Accepted { .. }
        ));
        assert!(matches!(
            compatibility.apply_mutation(
                NotificationMutation::Disconnect {
                    client_id: key.client_id,
                },
                13,
            ),
            NotificationHostResponse::Accepted { changed: true, .. }
        ));
        assert!(compatibility.registry.snapshot().icons.is_empty());
        let generation = compatibility.host_generation();
        compatibility.restart_generation();
        assert!(compatibility.host_generation() > generation);
        assert!(compatibility.registry.snapshot().icons.is_empty());
    }

    #[test]
    fn registry_capacity_and_protected_overflow_remain_bounded() {
        let mut registry = NotificationRegistry::default();
        for index in 0..MAX_CLIENTS {
            assert!(matches!(
                registry.apply(NotificationMutation::RegisterClient {
                    client_id: format!("client-{index}"),
                }),
                NotificationHostResponse::Accepted { changed: true, .. }
            ));
        }
        assert_eq!(
            registry.apply(NotificationMutation::RegisterClient {
                client_id: "overflow".into(),
            }),
            NotificationHostResponse::Rejected("client-capacity".into())
        );
        let mut queue = NotificationEventQueue {
            events: VecDeque::new(),
            capacity: 2,
        };
        let event = |id: &str, kind| NotificationEvent {
            correlation_id: id.into(),
            key: IconKey {
                client_id: "client".into(),
                icon_id: 1,
            },
            kind,
            admitted_unix_ms: 1,
        };
        assert!(queue.push(event("hover-1", NotificationEventKind::Hover)));
        assert!(queue.push(event("hover-2", NotificationEventKind::Hover)));
        assert!(queue.push(event("activate", NotificationEventKind::Activate)));
        assert!(
            queue
                .events
                .iter()
                .any(|event| event.kind == NotificationEventKind::Activate)
        );
        assert!(queue.len() <= 2);
    }
}
