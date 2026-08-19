use serde::{Deserialize, Serialize};

use crate::{
    IconData, MAX_COLLECTION_ITEMS, MAX_TEXT_BYTES, NotificationIcon, Validate, ValidationError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifyIconLayoutVersion {
    V1,
    V2,
    V3,
    V4,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotifyIconClientIdentity {
    pub process_id: u32,
    pub session_id: u32,
    pub window_identity: i64,
}

impl Validate for NotifyIconClientIdentity {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.process_id == 0 || self.window_identity == 0 {
            Err(ValidationError::OutOfRange("notify_icon.client_identity"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NotifyIconIdentity {
    pub numeric_id: u32,
    pub guid: Option<[u8; 16]>,
}

impl Validate for NotifyIconIdentity {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.numeric_id == 0 && self.guid.is_none() {
            Err(ValidationError::InvalidValue("notify_icon.icon_identity"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotifyIconCallbackRoute {
    pub message_id: u32,
    pub negotiated_version: NotifyIconLayoutVersion,
}

impl Validate for NotifyIconCallbackRoute {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.message_id == 0 {
            Err(ValidationError::OutOfRange("notify_icon.callback_message"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OwnedNotifyIcon {
    pub client: NotifyIconClientIdentity,
    pub identity: NotifyIconIdentity,
    pub callback: NotifyIconCallbackRoute,
    pub tooltip: String,
    pub visible: bool,
    pub pixels: Option<IconData>,
    #[serde(default)]
    pub notification: Option<OwnedNotificationContent>,
    pub generation: u64,
}

impl Validate for OwnedNotifyIcon {
    fn validate(&self) -> Result<(), ValidationError> {
        self.client.validate()?;
        self.identity.validate()?;
        self.callback.validate()?;
        if self.tooltip.len() > MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong("notify_icon.tooltip"));
        }
        if self.generation == 0 {
            return Err(ValidationError::OutOfRange("notify_icon.generation"));
        }
        if let Some(pixels) = &self.pixels {
            pixels.validate()?;
        }
        if let Some(notification) = &self.notification {
            notification.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationSeverity {
    #[default]
    Information,
    Warning,
    Error,
    User,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OwnedNotificationContent {
    pub title: String,
    pub body: String,
    pub severity: NotificationSeverity,
    pub realtime: bool,
    pub timeout_ms: u32,
}

impl Validate for OwnedNotificationContent {
    fn validate(&self) -> Result<(), ValidationError> {
        for (field, value) in [
            ("notification.title", &self.title),
            ("notification.body", &self.body),
        ] {
            if value.len() > MAX_TEXT_BYTES {
                return Err(ValidationError::TextTooLong(field));
            }
        }
        if self.title.is_empty() && self.body.is_empty() {
            return Err(ValidationError::Empty("notification.content"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotifyIconCompatibilityOperation {
    Add {
        icon: OwnedNotifyIcon,
    },
    Modify {
        icon: OwnedNotifyIcon,
    },
    Delete {
        identity: NotifyIconIdentity,
        generation: u64,
    },
    SetFocus {
        identity: NotifyIconIdentity,
        generation: u64,
    },
    SetVersion {
        identity: NotifyIconIdentity,
        version: NotifyIconLayoutVersion,
        generation: u64,
    },
}

impl Validate for NotifyIconCompatibilityOperation {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Add { icon } | Self::Modify { icon } => icon.validate(),
            Self::Delete {
                identity,
                generation,
            }
            | Self::SetFocus {
                identity,
                generation,
            }
            | Self::SetVersion {
                identity,
                generation,
                ..
            } => {
                identity.validate()?;
                if *generation == 0 {
                    Err(ValidationError::OutOfRange(
                        "notify_icon.operation_generation",
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotifyIconCompatibilityRequest {
    pub correlation_id: String,
    pub expected_host_generation: u64,
    pub deadline_unix_ms: u64,
    pub operation: NotifyIconCompatibilityOperation,
}

impl Validate for NotifyIconCompatibilityRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.correlation_id.trim().is_empty() {
            return Err(ValidationError::Empty("notify_icon.correlation_id"));
        }
        if self.correlation_id.len() > MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong("notify_icon.correlation_id"));
        }
        if self.expected_host_generation == 0 || self.deadline_unix_ms == 0 {
            return Err(ValidationError::OutOfRange(
                "notify_icon.request_generation_or_deadline",
            ));
        }
        self.operation.validate()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifyIconTerminalKind {
    Applied,
    NoChange,
    InvalidRequest,
    StaleGeneration,
    Timeout,
    Cancelled,
    OwnerUnavailable,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotifyIconCompatibilityTerminal {
    pub correlation_id: String,
    pub host_generation: u64,
    pub icon_generation: Option<u64>,
    pub terminal: NotifyIconTerminalKind,
    pub message: String,
}

impl Validate for NotifyIconCompatibilityTerminal {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.correlation_id.trim().is_empty() || self.host_generation == 0 {
            return Err(ValidationError::InvalidValue(
                "notify_icon.terminal_identity",
            ));
        }
        if self.message.len() > MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong("notify_icon.terminal_message"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IconKey {
    pub client_id: String,
    pub icon_id: u32,
}

impl Validate for IconKey {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.client_id.trim().is_empty() {
            Err(ValidationError::Empty("notification.client_id"))
        } else if self.client_id.len() > crate::MAX_TEXT_BYTES {
            Err(ValidationError::TextTooLong("notification.client_id"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegisteredIcon {
    pub key: IconKey,
    pub generation: u64,
    pub icon: NotificationIcon,
    pub always_visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OwnedNotification {
    pub notification_id: String,
    pub key: IconKey,
    pub application_label: String,
    pub title: String,
    pub body: String,
    pub severity: NotificationSeverity,
    pub admitted_unix_ms: u64,
    pub generation: u64,
    pub icon: Option<IconData>,
}

impl Validate for OwnedNotification {
    fn validate(&self) -> Result<(), ValidationError> {
        self.key.validate()?;
        if self.notification_id.trim().is_empty() {
            return Err(ValidationError::Empty("notification.notification_id"));
        }
        for (field, value) in [
            ("notification.notification_id", &self.notification_id),
            ("notification.application_label", &self.application_label),
            ("notification.title", &self.title),
            ("notification.body", &self.body),
        ] {
            if value.len() > MAX_TEXT_BYTES {
                return Err(ValidationError::TextTooLong(field));
            }
        }
        if self.title.is_empty() && self.body.is_empty() {
            return Err(ValidationError::Empty("notification.content"));
        }
        if self.admitted_unix_ms == 0 || self.generation == 0 {
            return Err(ValidationError::OutOfRange(
                "notification.time_or_generation",
            ));
        }
        if let Some(icon) = &self.icon {
            icon.validate()?;
        }
        Ok(())
    }
}

impl Validate for RegisteredIcon {
    fn validate(&self) -> Result<(), ValidationError> {
        self.key.validate()?;
        if self.icon.owner_id != self.key.client_id || self.icon.icon_id != self.key.icon_id {
            return Err(ValidationError::InvalidValue("notification.icon_identity"));
        }
        self.icon.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NotificationMutation {
    RegisterClient {
        client_id: String,
    },
    Add {
        icon: RegisteredIcon,
    },
    Modify {
        icon: RegisteredIcon,
    },
    Delete {
        key: IconKey,
        generation: u64,
    },
    Focus {
        key: IconKey,
        generation: u64,
    },
    Disconnect {
        client_id: String,
    },
    Event {
        event: NotificationEvent,
    },
    CancelEvent {
        correlation_id: String,
    },
    DrainEvents {
        client_id: String,
    },
    DismissNotification {
        notification_id: String,
        expected_generation: u64,
    },
    ClearNotifications {
        expected_generation: u64,
    },
    Snapshot,
    Health,
}

impl Validate for NotificationMutation {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::RegisterClient { client_id }
            | Self::Disconnect { client_id }
            | Self::DrainEvents { client_id } => IconKey {
                client_id: client_id.clone(),
                icon_id: 0,
            }
            .validate(),
            Self::Add { icon } | Self::Modify { icon } => icon.validate(),
            Self::Delete { key, .. } | Self::Focus { key, .. } => key.validate(),
            Self::Event { event } => event.validate(),
            Self::CancelEvent { correlation_id } => {
                if correlation_id.trim().is_empty() {
                    Err(ValidationError::Empty("notification.correlation_id"))
                } else if correlation_id.len() > crate::MAX_TEXT_BYTES {
                    Err(ValidationError::TextTooLong("notification.correlation_id"))
                } else {
                    Ok(())
                }
            }
            Self::DismissNotification {
                notification_id,
                expected_generation,
            } => {
                if notification_id.trim().is_empty() {
                    Err(ValidationError::Empty("notification.notification_id"))
                } else if notification_id.len() > MAX_TEXT_BYTES {
                    Err(ValidationError::TextTooLong("notification.notification_id"))
                } else if *expected_generation == 0 {
                    Err(ValidationError::OutOfRange(
                        "notification.expected_generation",
                    ))
                } else {
                    Ok(())
                }
            }
            Self::ClearNotifications {
                expected_generation,
            } => {
                if *expected_generation == 0 {
                    Err(ValidationError::OutOfRange(
                        "notification.expected_generation",
                    ))
                } else {
                    Ok(())
                }
            }
            Self::Snapshot | Self::Health => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventKind {
    Activate,
    Context,
    Hover,
    Focus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub correlation_id: String,
    pub key: IconKey,
    pub kind: NotificationEventKind,
    pub admitted_unix_ms: u64,
}

impl Validate for NotificationEvent {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.correlation_id.trim().is_empty() {
            return Err(ValidationError::Empty("notification.correlation_id"));
        }
        self.key.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotificationSnapshot {
    pub generation: u64,
    pub icons: Vec<RegisteredIcon>,
    #[serde(default)]
    pub notifications: Vec<OwnedNotification>,
}

impl Validate for NotificationSnapshot {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.icons.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge("notification.icons"));
        }
        for icon in &self.icons {
            icon.validate()?;
        }
        if self.notifications.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge(
                "notification.notifications",
            ));
        }
        for notification in &self.notifications {
            notification.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotificationHostHealth {
    pub healthy: bool,
    pub clients: usize,
    pub icons: usize,
    pub capacity: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NotificationHostResponse {
    Accepted { changed: bool, generation: u64 },
    Snapshot(NotificationSnapshot),
    Health(NotificationHostHealth),
    Events(Vec<NotificationEvent>),
    Rejected(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IconData;

    #[test]
    fn icon_identity_and_rgba_size_fail_closed() {
        let icon = RegisteredIcon {
            key: IconKey {
                client_id: "client".into(),
                icon_id: 1,
            },
            generation: 1,
            icon: NotificationIcon {
                owner_id: "other".into(),
                icon_id: 1,
                tooltip: "tip".into(),
                visible: true,
                icon: None,
            },
            always_visible: false,
        };
        assert!(icon.validate().is_err());
        let bad = IconData {
            width: 2,
            height: 2,
            rgba: vec![0; 3],
        };
        assert!(bad.validate().is_err());
    }

    fn compatibility_icon() -> OwnedNotifyIcon {
        OwnedNotifyIcon {
            client: NotifyIconClientIdentity {
                process_id: 42,
                session_id: 1,
                window_identity: 100,
            },
            identity: NotifyIconIdentity {
                numeric_id: 7,
                guid: None,
            },
            callback: NotifyIconCallbackRoute {
                message_id: 0x500,
                negotiated_version: NotifyIconLayoutVersion::V4,
            },
            tooltip: "Controlled icon".into(),
            visible: true,
            pixels: Some(IconData {
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3, 255],
            }),
            notification: None,
            generation: 1,
        }
    }

    #[test]
    fn compatibility_operations_round_trip_with_owned_bounded_identity() {
        let request = NotifyIconCompatibilityRequest {
            correlation_id: "notify-1".into(),
            expected_host_generation: 9,
            deadline_unix_ms: 100,
            operation: NotifyIconCompatibilityOperation::Add {
                icon: compatibility_icon(),
            },
        };
        request.validate().unwrap();
        let encoded = serde_json::to_vec(&request).unwrap();
        assert!(encoded.len() <= crate::MAX_FRAME_BYTES);
        assert_eq!(
            serde_json::from_slice::<NotifyIconCompatibilityRequest>(&encoded).unwrap(),
            request
        );
    }

    #[test]
    fn compatibility_bounds_and_generations_fail_closed() {
        let mut icon = compatibility_icon();
        icon.client.process_id = 0;
        assert!(icon.validate().is_err());
        let mut icon = compatibility_icon();
        icon.tooltip = "x".repeat(MAX_TEXT_BYTES + 1);
        assert!(icon.validate().is_err());
        let request = NotifyIconCompatibilityRequest {
            correlation_id: String::new(),
            expected_host_generation: 0,
            deadline_unix_ms: 0,
            operation: NotifyIconCompatibilityOperation::Delete {
                identity: NotifyIconIdentity {
                    numeric_id: 0,
                    guid: None,
                },
                generation: 0,
            },
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn notification_fields_are_additive_bounded_and_frame_safe() {
        let legacy_icon = serde_json::json!({
            "client": {"process_id":42,"session_id":1,"window_identity":100},
            "identity": {"numeric_id":7,"guid":null},
            "callback": {"message_id":1280,"negotiated_version":"v4"},
            "tooltip":"Legacy icon","visible":true,"pixels":null,"generation":1
        });
        let decoded: OwnedNotifyIcon = serde_json::from_value(legacy_icon).unwrap();
        assert_eq!(decoded.notification, None);
        let legacy_snapshot = serde_json::json!({"generation":1,"icons":[]});
        let decoded: NotificationSnapshot = serde_json::from_value(legacy_snapshot).unwrap();
        assert!(decoded.notifications.is_empty());

        let mut icon = compatibility_icon();
        icon.notification = Some(OwnedNotificationContent {
            title: "Build complete".into(),
            body: "SuperDesktop is ready".into(),
            severity: NotificationSeverity::Information,
            realtime: true,
            timeout_ms: 5_000,
        });
        icon.validate().unwrap();
        assert!(serde_json::to_vec(&icon).unwrap().len() <= crate::MAX_FRAME_BYTES);
        icon.notification.as_mut().unwrap().body = "x".repeat(MAX_TEXT_BYTES + 1);
        assert!(icon.validate().is_err());

        let empty = OwnedNotificationContent::default();
        assert!(empty.validate().is_err());
        let invalid = NotificationMutation::DismissNotification {
            notification_id: String::new(),
            expected_generation: 0,
        };
        assert!(invalid.validate().is_err());
        let invalid = NotificationMutation::ClearNotifications {
            expected_generation: 0,
        };
        assert!(invalid.validate().is_err());
    }
}
