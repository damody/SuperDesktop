use serde::{Deserialize, Serialize};

use crate::{MAX_COLLECTION_ITEMS, MAX_ICON_BYTES, Validate, ValidationError};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellItemKind {
    File,
    Folder,
    Application,
    Setting,
    Virtual,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellItem {
    pub id: String,
    pub display_name: String,
    pub parsing_name: String,
    pub kind: ShellItemKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRisk {
    Normal,
    Destructive,
    RequiresElevation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub id: CommandId,
    pub label: String,
    pub enabled: bool,
    pub risk: CommandRisk,
    #[serde(default)]
    pub children: Vec<CommandDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCategory {
    Application,
    File,
    Setting,
    Command,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub category: SearchCategory,
    pub score_milli: u16,
    pub activation: CommandDescriptor,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationIcon {
    pub owner_id: String,
    pub icon_id: u32,
    pub tooltip: String,
    pub visible: bool,
    pub icon: Option<IconData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPreview {
    pub window_id: u64,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualDesktop {
    pub id: String,
    pub name: String,
    pub ordinal: u32,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderCapability {
    ContextMenu,
    SearchApplications,
    SearchFiles,
    SearchSettings,
    NotificationArea,
    TaskPreview,
    VirtualDesktop,
}

fn validate_text(field: &'static str, value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::Empty(field));
    }
    if value.len() > crate::MAX_TEXT_BYTES {
        return Err(ValidationError::TextTooLong(field));
    }
    Ok(())
}

impl Validate for ShellItem {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text("shell_item.id", &self.id)?;
        validate_text("shell_item.display_name", &self.display_name)?;
        validate_text("shell_item.parsing_name", &self.parsing_name)
    }
}

impl Validate for CommandDescriptor {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text("command.id", &self.id.0)?;
        validate_text("command.label", &self.label)?;
        if self.children.len() > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge("command.children"));
        }
        for child in &self.children {
            child.validate()?;
        }
        Ok(())
    }
}

impl Validate for SearchResult {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text("search_result.id", &self.id)?;
        validate_text("search_result.title", &self.title)?;
        if self.score_milli > 1_000 {
            return Err(ValidationError::OutOfRange("search_result.score_milli"));
        }
        self.activation.validate()
    }
}

impl Validate for IconData {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.rgba.len() > MAX_ICON_BYTES {
            return Err(ValidationError::CollectionTooLarge("icon.rgba"));
        }
        let expected = u64::from(self.width) * u64::from(self.height) * 4;
        if expected != self.rgba.len() as u64 {
            return Err(ValidationError::InvalidValue("icon.rgba"));
        }
        Ok(())
    }
}

impl Validate for NotificationIcon {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text("notification.owner_id", &self.owner_id)?;
        if self.tooltip.len() > crate::MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong("notification.tooltip"));
        }
        if let Some(icon) = &self.icon {
            icon.validate()?;
        }
        Ok(())
    }
}

impl Validate for TaskPreview {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text("task_preview.title", &self.title)?;
        if self.width == 0 || self.height == 0 {
            return Err(ValidationError::OutOfRange("task_preview.size"));
        }
        Ok(())
    }
}

impl Validate for VirtualDesktop {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_text("virtual_desktop.id", &self.id)?;
        validate_text("virtual_desktop.name", &self.name)
    }
}
