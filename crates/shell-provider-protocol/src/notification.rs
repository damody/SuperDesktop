use serde::{Deserialize, Serialize};

use crate::{MAX_COLLECTION_ITEMS, NotificationIcon, Validate, ValidationError};

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
    RegisterClient { client_id: String },
    Add { icon: RegisteredIcon },
    Modify { icon: RegisteredIcon },
    Delete { key: IconKey, generation: u64 },
    Focus { key: IconKey, generation: u64 },
    Disconnect { client_id: String },
    Event { event: NotificationEvent },
    DrainEvents { client_id: String },
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
}

impl Validate for NotificationSnapshot {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.icons.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge("notification.icons"));
        }
        for icon in &self.icons {
            icon.validate()?;
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
}
