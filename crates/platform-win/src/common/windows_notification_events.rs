//! Documented Windows Runtime notification-listener boundary.

use std::{
    collections::BTreeSet,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
    },
};

use shell_provider_protocol::{
    IconKey, MAX_TEXT_BYTES, NotificationSeverity, OwnedNotification, WindowsNotificationAccess,
    WindowsNotificationChange, WindowsNotificationEventStatus,
};
use windows::{
    Foundation::TypedEventHandler,
    UI::Notifications::Management::{
        UserNotificationListener, UserNotificationListenerAccessStatus,
    },
    UI::Notifications::{
        KnownNotificationBindings, NotificationKinds, UserNotification,
        UserNotificationChangedEventArgs, UserNotificationChangedKind,
    },
    Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    },
};

const MAX_NATIVE_NOTIFICATIONS: usize = 400;
const MAX_WINDOWS_NOTIFICATIONS: usize = 100;
const WINDOWS_EPOCH_TICKS: i64 = 116_444_736_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsNotificationBatch {
    pub notifications: Vec<OwnedNotification>,
    pub status: WindowsNotificationEventStatus,
    pub skipped: usize,
}

struct WinRtApartment(bool);

impl WinRtApartment {
    fn enter() -> Result<Self, String> {
        match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            Ok(()) => Ok(Self(true)),
            Err(error) if error.code() == RPC_E_CHANGED_MODE => Ok(Self(false)),
            Err(error) => Err(format!("Windows Runtime initialization failed: {error}")),
        }
    }
}

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { RoUninitialize() };
        }
    }
}

pub struct WindowsNotificationEventSource {
    listener: UserNotificationListener,
    event_token: Option<i64>,
    dirty: Arc<AtomicBool>,
    last_change: Arc<AtomicU8>,
    last_native_id: Arc<AtomicU32>,
    access: WindowsNotificationAccess,
    access_reason: String,
    _apartment: WinRtApartment,
}

impl WindowsNotificationEventSource {
    pub fn new() -> Result<Self, String> {
        Self::new_with_access_request(true)
    }

    pub fn new_with_access_request(request_access: bool) -> Result<Self, String> {
        let apartment = WinRtApartment::enter()?;
        let listener = UserNotificationListener::Current()
            .map_err(|error| format!("Windows notification listener is unavailable: {error}"))?;
        let mut access = listener
            .GetAccessStatus()
            .map_err(|error| format!("Windows notification access query failed: {error}"))?;
        if access == UserNotificationListenerAccessStatus::Unspecified && request_access {
            access = listener
                .RequestAccessAsync()
                .and_then(|operation| operation.join())
                .map_err(|error| format!("Windows notification access request failed: {error}"))?;
        }
        let access = map_access(access);
        let access_reason = match access {
            WindowsNotificationAccess::Denied => "Windows notification access was denied".into(),
            WindowsNotificationAccess::Unspecified => {
                "Windows notification access remains unspecified".into()
            }
            WindowsNotificationAccess::Unavailable => {
                "Windows notification listener access is unavailable".into()
            }
            _ => String::new(),
        };
        let dirty = Arc::new(AtomicBool::new(true));
        let last_change = Arc::new(AtomicU8::new(0));
        let last_native_id = Arc::new(AtomicU32::new(0));
        let event_token = if access == WindowsNotificationAccess::Allowed {
            let dirty_callback = Arc::clone(&dirty);
            let change_callback = Arc::clone(&last_change);
            let id_callback = Arc::clone(&last_native_id);
            let handler = TypedEventHandler::<
                UserNotificationListener,
                UserNotificationChangedEventArgs,
            >::new(move |_, args| {
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    let Some(args) = args.as_ref() else {
                        return;
                    };
                    let change = args.ChangeKind().unwrap_or_default();
                    let value = if change == UserNotificationChangedKind::Added {
                        1
                    } else if change == UserNotificationChangedKind::Removed {
                        2
                    } else {
                        0
                    };
                    change_callback.store(value, Ordering::Release);
                    id_callback.store(args.UserNotificationId().unwrap_or(0), Ordering::Release);
                    dirty_callback.store(true, Ordering::Release);
                }));
                Ok(())
            });
            Some(listener.NotificationChanged(&handler).map_err(|error| {
                format!("Windows notification event subscription failed: {error}")
            })?)
        } else {
            None
        };
        Ok(Self {
            listener,
            event_token,
            dirty,
            last_change,
            last_native_id,
            access,
            access_reason,
            _apartment: apartment,
        })
    }

    pub fn access_status(&self) -> WindowsNotificationEventStatus {
        WindowsNotificationEventStatus {
            access: self.access.clone(),
            synchronized: false,
            last_change: self.last_change(),
            reason: self.access_reason.clone(),
        }
    }

    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }

    pub fn last_native_id(&self) -> u32 {
        self.last_native_id.load(Ordering::Acquire)
    }

    pub fn snapshot(&self) -> Result<WindowsNotificationBatch, String> {
        if self.access != WindowsNotificationAccess::Allowed {
            return Ok(WindowsNotificationBatch {
                notifications: Vec::new(),
                status: self.access_status(),
                skipped: 0,
            });
        }
        let items = self
            .listener
            .GetNotificationsAsync(NotificationKinds::Toast)
            .and_then(|operation| operation.join())
            .map_err(|error| format!("Windows notification snapshot failed: {error}"))?;
        let count = usize::try_from(items.Size().unwrap_or(0))
            .unwrap_or(usize::MAX)
            .min(MAX_NATIVE_NOTIFICATIONS);
        let mut notifications = Vec::new();
        let mut skipped = 0usize;
        for index in 0..count {
            let converted = u32::try_from(index)
                .ok()
                .and_then(|index| items.GetAt(index).ok())
                .and_then(|item| convert_notification(&item).ok());
            if let Some(notification) = converted {
                notifications.push(notification);
            } else {
                skipped = skipped.saturating_add(1);
            }
        }
        reduce_notifications(&mut notifications);
        self.dirty.store(false, Ordering::Release);
        Ok(WindowsNotificationBatch {
            notifications,
            status: WindowsNotificationEventStatus {
                access: WindowsNotificationAccess::Allowed,
                synchronized: true,
                last_change: self.last_change(),
                reason: String::new(),
            },
            skipped,
        })
    }

    pub fn remove(&self, native_id: u32) -> Result<(), String> {
        if self.access != WindowsNotificationAccess::Allowed {
            return Err("Windows notification access is not allowed".into());
        }
        self.listener
            .GetNotification(native_id)
            .map_err(|_| "Windows notification identity is stale".to_owned())?;
        self.listener
            .RemoveNotification(native_id)
            .map_err(|error| format!("Windows notification remove failed: {error}"))?;
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        if self.access != WindowsNotificationAccess::Allowed {
            return Err("Windows notification access is not allowed".into());
        }
        self.listener
            .ClearNotifications()
            .map_err(|error| format!("Windows notification clear failed: {error}"))?;
        self.dirty.store(true, Ordering::Release);
        Ok(())
    }

    fn last_change(&self) -> WindowsNotificationChange {
        match self.last_change.load(Ordering::Acquire) {
            1 => WindowsNotificationChange::Added,
            2 => WindowsNotificationChange::Removed,
            _ => WindowsNotificationChange::None,
        }
    }
}

impl Drop for WindowsNotificationEventSource {
    fn drop(&mut self) {
        if let Some(token) = self.event_token.take() {
            let _ = self.listener.RemoveNotificationChanged(token);
        }
    }
}

fn map_access(status: UserNotificationListenerAccessStatus) -> WindowsNotificationAccess {
    if status == UserNotificationListenerAccessStatus::Allowed {
        WindowsNotificationAccess::Allowed
    } else if status == UserNotificationListenerAccessStatus::Denied {
        WindowsNotificationAccess::Denied
    } else {
        WindowsNotificationAccess::Unspecified
    }
}

fn convert_notification(item: &UserNotification) -> Result<OwnedNotification, String> {
    let native_id = item
        .Id()
        .map_err(|error| format!("notification ID unavailable: {error}"))?;
    let app = item
        .AppInfo()
        .map_err(|error| format!("notification AppInfo unavailable: {error}"))?;
    let app_label = app
        .DisplayInfo()
        .and_then(|display| display.DisplayName())
        .map(|value| bounded_text(&value.to_string()))
        .map_err(|error| format!("notification app label unavailable: {error}"))?;
    let aumid = app
        .AppUserModelId()
        .map(|value| value.to_string())
        .map_err(|error| format!("notification app identity unavailable: {error}"))?;
    let binding_name = KnownNotificationBindings::ToastGeneric()
        .map_err(|error| format!("ToastGeneric binding unavailable: {error}"))?;
    let texts = item
        .Notification()
        .and_then(|notification| notification.Visual())
        .and_then(|visual| visual.GetBinding(&binding_name))
        .and_then(|binding| binding.GetTextElements())
        .map_err(|error| format!("notification text binding unavailable: {error}"))?;
    let count = usize::try_from(texts.Size().unwrap_or(0))
        .unwrap_or(usize::MAX)
        .min(64);
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        let text = texts
            .GetAt(index as u32)
            .and_then(|text| text.Text())
            .map(|value| bounded_text(&value.to_string()))
            .map_err(|error| format!("notification text unavailable: {error}"))?;
        if !text.trim().is_empty() {
            values.push(text);
        }
    }
    let title = values.first().cloned().unwrap_or_default();
    let body = bounded_text(&values.into_iter().skip(1).collect::<Vec<_>>().join("\n"));
    if title.is_empty() && body.is_empty() {
        return Err("notification contains no bounded text".into());
    }
    let admitted_unix_ms = windows_ticks_to_unix_ms(
        item.CreationTime()
            .map_err(|error| format!("notification creation time unavailable: {error}"))?
            .UniversalTime,
    )?;
    Ok(OwnedNotification {
        notification_id: format!("windows:{native_id}"),
        key: IconKey {
            client_id: "windows-events".into(),
            icon_id: stable_app_hash(&aumid),
        },
        application_label: if app_label.trim().is_empty() {
            "Application".into()
        } else {
            app_label
        },
        title,
        body,
        severity: NotificationSeverity::Information,
        admitted_unix_ms,
        generation: u64::from(native_id).saturating_add(1),
        icon: None,
    })
}

fn windows_ticks_to_unix_ms(ticks: i64) -> Result<u64, String> {
    let unix_ticks = ticks
        .checked_sub(WINDOWS_EPOCH_TICKS)
        .ok_or_else(|| "notification creation time predates Unix epoch".to_owned())?;
    if unix_ticks < 0 {
        return Err("notification creation time predates Unix epoch".into());
    }
    u64::try_from(unix_ticks / 10_000)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "notification creation time is invalid".to_owned())
}

fn stable_app_hash(value: &str) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for byte in value.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash.max(1)
}

fn bounded_text(value: &str) -> String {
    if value.len() <= MAX_TEXT_BYTES {
        return value.to_owned();
    }
    let mut end = MAX_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn reduce_notifications(notifications: &mut Vec<OwnedNotification>) {
    notifications.sort_by(|left, right| {
        right
            .admitted_unix_ms
            .cmp(&left.admitted_unix_ms)
            .then_with(|| left.notification_id.cmp(&right.notification_id))
    });
    let mut seen = BTreeSet::new();
    notifications.retain(|notification| seen.insert(notification.notification_id.clone()));
    notifications.truncate(MAX_WINDOWS_NOTIFICATIONS);
}

pub fn parse_windows_notification_id(value: &str) -> Option<u32> {
    let raw = value.strip_prefix("windows:")?;
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    raw.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_hash_id_and_text_bounds_are_deterministic() {
        assert_eq!(
            windows_ticks_to_unix_ms(WINDOWS_EPOCH_TICKS + 10_000),
            Ok(1)
        );
        assert!(windows_ticks_to_unix_ms(WINDOWS_EPOCH_TICKS - 1).is_err());
        assert_eq!(stable_app_hash("fixture"), stable_app_hash("fixture"));
        assert_ne!(stable_app_hash("fixture"), stable_app_hash("other"));
        assert_eq!(parse_windows_notification_id("windows:42"), Some(42));
        assert_eq!(parse_windows_notification_id("windows:"), None);
        assert_eq!(parse_windows_notification_id("windows:-1"), None);
        assert_eq!(parse_windows_notification_id("owned:42"), None);
        let bounded = bounded_text(&"界".repeat(MAX_TEXT_BYTES));
        assert!(bounded.len() <= MAX_TEXT_BYTES && bounded.is_char_boundary(bounded.len()));
    }

    #[test]
    fn reducer_deduplicates_sorts_and_caps() {
        let mut values = (0..=MAX_WINDOWS_NOTIFICATIONS)
            .map(|index| OwnedNotification {
                notification_id: format!("windows:{index}"),
                key: IconKey {
                    client_id: "windows-events".into(),
                    icon_id: 1,
                },
                application_label: "fixture".into(),
                title: format!("title-{index}"),
                body: String::new(),
                severity: NotificationSeverity::Information,
                admitted_unix_ms: index as u64 + 1,
                generation: index as u64 + 1,
                icon: None,
            })
            .collect::<Vec<_>>();
        values.push(values[0].clone());
        reduce_notifications(&mut values);
        assert_eq!(values.len(), MAX_WINDOWS_NOTIFICATIONS);
        assert!(
            values
                .windows(2)
                .all(|pair| pair[0].admitted_unix_ms >= pair[1].admitted_unix_ms)
        );
    }

    #[test]
    fn production_source_uses_documented_listener_and_no_private_shell_route() {
        let production = include_str!("windows_notification_events.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for required in [
            "UserNotificationListener::Current",
            "RequestAccessAsync",
            "NotificationChanged",
            "GetNotificationsAsync",
            "NotificationKinds::Toast",
            "KnownNotificationBindings::ToastGeneric",
            "RemoveNotification",
            "ClearNotifications",
            "RemoveNotificationChanged",
            "RoUninitialize",
            "catch_unwind",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        for forbidden in [
            "wpndatabase",
            "ShellExperienceHost",
            "explorer.exe",
            "ms-settings:",
        ] {
            assert!(!production.contains(forbidden), "forbidden {forbidden}");
        }
    }
}
