use std::collections::BTreeMap;
use std::fmt;

use crate::json::{self, Value};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExecutionPreference {
    #[default]
    Preview,
    Shell,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeMode {
    Preview,
    Shell,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WallpaperMode {
    #[default]
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
    Span,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WallpaperSettings {
    pub source: Option<String>,
    pub mode: WallpaperMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopPosition {
    pub monitor_id: String,
    pub item_id: String,
    pub logical_x: i32,
    pub logical_y: i32,
    pub layout_revision: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DesktopSortKey {
    #[default]
    Name,
    Kind,
    Size,
    Modified,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DesktopSortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DesktopSettings {
    pub sort_key: DesktopSortKey,
    pub sort_direction: DesktopSortDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskbarSettings {
    pub rows: u8,
    pub pins: Vec<String>,
    pub combine_groups: bool,
    pub show_labels: bool,
    pub previews_enabled: bool,
    pub all_monitors: bool,
}

impl Default for TaskbarSettings {
    fn default() -> Self {
        Self {
            rows: 2,
            pins: Vec::new(),
            combine_groups: true,
            show_labels: false,
            previews_enabled: true,
            all_monitors: true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StartSettings {
    pub initialized: bool,
    pub pinned_ids: Vec<String>,
    pub recent_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessibilitySettings {
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub text_scale_percent: u16,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            reduce_motion: false,
            high_contrast: false,
            text_scale_percent: 100,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsV1 {
    pub schema_version: u32,
    pub revision: u64,
    pub execution_preference: ExecutionPreference,
    pub taskbar: TaskbarSettings,
    pub start: StartSettings,
    pub wallpaper: WallpaperSettings,
    pub desktop: DesktopSettings,
    pub desktop_positions: Vec<DesktopPosition>,
    pub monitor_mapping: BTreeMap<String, String>,
    pub superexplorer_path: Option<String>,
    pub theme: ThemePreference,
    pub accessibility: AccessibilitySettings,
    extensions: BTreeMap<String, Value>,
}

impl Default for SettingsV1 {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            revision: 0,
            execution_preference: ExecutionPreference::Preview,
            taskbar: TaskbarSettings::default(),
            start: StartSettings::default(),
            wallpaper: WallpaperSettings::default(),
            desktop: DesktopSettings::default(),
            desktop_positions: Vec::new(),
            monitor_mapping: BTreeMap::new(),
            superexplorer_path: None,
            theme: ThemePreference::System,
            accessibility: AccessibilitySettings::default(),
            extensions: BTreeMap::new(),
        }
    }
}

impl SettingsV1 {
    pub fn effective_mode(&self, explicit_shell_opt_in: bool) -> RuntimeMode {
        if self.execution_preference == ExecutionPreference::Shell && explicit_shell_opt_in {
            RuntimeMode::Shell
        } else {
            RuntimeMode::Preview
        }
    }

    pub fn encode(&self) -> String {
        json::stringify(&self.to_value())
    }

    pub fn decode(input: &str) -> Result<DecodeOutcome, SettingsError> {
        let value = json::parse(input).map_err(SettingsError::MalformedJson)?;
        Self::from_value(value)
    }

    fn from_value(value: Value) -> Result<DecodeOutcome, SettingsError> {
        let Value::Object(mut object) = value else {
            return Err(SettingsError::InvalidStructure("root must be an object"));
        };
        let version = take_u64(&mut object, "schema_version").unwrap_or(0);
        if version > u64::from(SETTINGS_SCHEMA_VERSION) {
            return Err(SettingsError::UnsupportedFutureVersion(version));
        }
        let mut settings = SettingsV1::default();
        let mut corrections = Vec::new();
        settings.revision = take_u64(&mut object, "revision").unwrap_or(0);
        settings.execution_preference = take_string(&mut object, "execution_preference")
            .and_then(|value| match value.as_str() {
                "preview" => Some(ExecutionPreference::Preview),
                "shell" => Some(ExecutionPreference::Shell),
                _ => None,
            })
            .unwrap_or_default();
        if let Some(Value::Object(mut taskbar)) = object.remove("taskbar") {
            let rows = take_u64(&mut taskbar, "rows").unwrap_or(2);
            if (1..=3).contains(&rows) {
                settings.taskbar.rows = rows as u8;
            } else {
                corrections.push(SettingsCorrection::TaskbarRows);
            }
            settings.taskbar.pins = take_string_array(&mut taskbar, "pins").unwrap_or_default();
            settings.taskbar.combine_groups =
                take_bool(&mut taskbar, "combine_groups").unwrap_or(true);
            settings.taskbar.show_labels = take_bool(&mut taskbar, "show_labels").unwrap_or(false);
            settings.taskbar.previews_enabled =
                take_bool(&mut taskbar, "previews_enabled").unwrap_or(true);
            settings.taskbar.all_monitors = take_bool(&mut taskbar, "all_monitors").unwrap_or(true);
        }
        if let Some(Value::Object(mut start)) = object.remove("start") {
            settings.start.initialized = take_bool(&mut start, "initialized").unwrap_or(false);
            settings.start.pinned_ids =
                take_string_array(&mut start, "pinned_ids").unwrap_or_default();
            settings.start.recent_ids =
                take_string_array(&mut start, "recent_ids").unwrap_or_default();
        }
        if let Some(Value::Object(mut wallpaper)) = object.remove("wallpaper") {
            settings.wallpaper.source = take_optional_string(&mut wallpaper, "source");
            settings.wallpaper.mode = take_string(&mut wallpaper, "mode")
                .and_then(|value| wallpaper_mode(&value))
                .unwrap_or_default();
        }
        if let Some(Value::Object(mut desktop)) = object.remove("desktop") {
            settings.desktop.sort_key = take_string(&mut desktop, "sort_key")
                .and_then(|value| match value.as_str() {
                    "name" => Some(DesktopSortKey::Name),
                    "kind" => Some(DesktopSortKey::Kind),
                    "size" => Some(DesktopSortKey::Size),
                    "modified" => Some(DesktopSortKey::Modified),
                    _ => None,
                })
                .unwrap_or_default();
            settings.desktop.sort_direction = take_string(&mut desktop, "sort_direction")
                .and_then(|value| match value.as_str() {
                    "ascending" => Some(DesktopSortDirection::Ascending),
                    "descending" => Some(DesktopSortDirection::Descending),
                    _ => None,
                })
                .unwrap_or_default();
        }
        settings.desktop_positions = object
            .remove("desktop_positions")
            .and_then(positions)
            .unwrap_or_default();
        settings.monitor_mapping = object
            .remove("monitor_mapping")
            .and_then(string_map)
            .unwrap_or_default();
        settings.superexplorer_path = take_optional_string(&mut object, "superexplorer_path");
        settings.theme = take_string(&mut object, "theme")
            .and_then(|value| match value.as_str() {
                "system" => Some(ThemePreference::System),
                "light" => Some(ThemePreference::Light),
                "dark" => Some(ThemePreference::Dark),
                _ => None,
            })
            .unwrap_or_default();
        if let Some(Value::Object(mut accessibility)) = object.remove("accessibility") {
            settings.accessibility.reduce_motion =
                take_bool(&mut accessibility, "reduce_motion").unwrap_or(false);
            settings.accessibility.high_contrast =
                take_bool(&mut accessibility, "high_contrast").unwrap_or(false);
            let scale = take_u64(&mut accessibility, "text_scale_percent").unwrap_or(100);
            if (100..=225).contains(&scale) {
                settings.accessibility.text_scale_percent = scale as u16;
            } else {
                corrections.push(SettingsCorrection::TextScale);
            }
        }
        settings.extensions = object;
        if version == 0 {
            corrections.push(SettingsCorrection::MigratedV0);
        }
        settings.schema_version = SETTINGS_SCHEMA_VERSION;
        Ok(DecodeOutcome {
            settings,
            corrections,
        })
    }

    fn to_value(&self) -> Value {
        let mut root = self.extensions.clone();
        root.insert(
            "schema_version".into(),
            Value::Number(i64::from(self.schema_version)),
        );
        root.insert("revision".into(), Value::Number(self.revision as i64));
        root.insert(
            "execution_preference".into(),
            Value::String(
                match self.execution_preference {
                    ExecutionPreference::Preview => "preview",
                    ExecutionPreference::Shell => "shell",
                }
                .into(),
            ),
        );
        root.insert(
            "taskbar".into(),
            Value::Object(BTreeMap::from([
                (
                    "pins".into(),
                    Value::Array(
                        self.taskbar
                            .pins
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                ),
                ("rows".into(), Value::Number(i64::from(self.taskbar.rows))),
                (
                    "combine_groups".into(),
                    Value::Bool(self.taskbar.combine_groups),
                ),
                ("show_labels".into(), Value::Bool(self.taskbar.show_labels)),
                (
                    "previews_enabled".into(),
                    Value::Bool(self.taskbar.previews_enabled),
                ),
                (
                    "all_monitors".into(),
                    Value::Bool(self.taskbar.all_monitors),
                ),
            ])),
        );
        root.insert(
            "start".into(),
            Value::Object(BTreeMap::from([
                ("initialized".into(), Value::Bool(self.start.initialized)),
                (
                    "pinned_ids".into(),
                    Value::Array(
                        self.start
                            .pinned_ids
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                ),
                (
                    "recent_ids".into(),
                    Value::Array(
                        self.start
                            .recent_ids
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                ),
            ])),
        );
        root.insert(
            "wallpaper".into(),
            Value::Object(BTreeMap::from([
                (
                    "mode".into(),
                    Value::String(wallpaper_mode_name(self.wallpaper.mode).into()),
                ),
                (
                    "source".into(),
                    self.wallpaper
                        .source
                        .clone()
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                ),
            ])),
        );
        root.insert(
            "desktop_positions".into(),
            Value::Array(self.desktop_positions.iter().map(position_value).collect()),
        );
        root.insert(
            "desktop".into(),
            Value::Object(BTreeMap::from([
                (
                    "sort_key".into(),
                    Value::String(
                        match self.desktop.sort_key {
                            DesktopSortKey::Name => "name",
                            DesktopSortKey::Kind => "kind",
                            DesktopSortKey::Size => "size",
                            DesktopSortKey::Modified => "modified",
                        }
                        .into(),
                    ),
                ),
                (
                    "sort_direction".into(),
                    Value::String(
                        match self.desktop.sort_direction {
                            DesktopSortDirection::Ascending => "ascending",
                            DesktopSortDirection::Descending => "descending",
                        }
                        .into(),
                    ),
                ),
            ])),
        );
        root.insert(
            "monitor_mapping".into(),
            Value::Object(
                self.monitor_mapping
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                    .collect(),
            ),
        );
        root.insert(
            "superexplorer_path".into(),
            self.superexplorer_path
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        root.insert(
            "theme".into(),
            Value::String(
                match self.theme {
                    ThemePreference::System => "system",
                    ThemePreference::Light => "light",
                    ThemePreference::Dark => "dark",
                }
                .into(),
            ),
        );
        root.insert(
            "accessibility".into(),
            Value::Object(BTreeMap::from([
                (
                    "high_contrast".into(),
                    Value::Bool(self.accessibility.high_contrast),
                ),
                (
                    "reduce_motion".into(),
                    Value::Bool(self.accessibility.reduce_motion),
                ),
                (
                    "text_scale_percent".into(),
                    Value::Number(i64::from(self.accessibility.text_scale_percent)),
                ),
            ])),
        );
        Value::Object(root)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodeOutcome {
    pub settings: SettingsV1,
    pub corrections: Vec<SettingsCorrection>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsCorrection {
    TaskbarRows,
    TextScale,
    MigratedV0,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsError {
    MalformedJson(String),
    InvalidStructure(&'static str),
    UnsupportedFutureVersion(u64),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedJson(error) => write!(formatter, "malformed JSON: {error}"),
            Self::InvalidStructure(error) => formatter.write_str(error),
            Self::UnsupportedFutureVersion(version) => {
                write!(formatter, "unsupported settings version {version}")
            }
        }
    }
}

impl std::error::Error for SettingsError {}

fn take_u64(object: &mut BTreeMap<String, Value>, key: &str) -> Option<u64> {
    match object.remove(key)? {
        Value::Number(value) => u64::try_from(value).ok(),
        _ => None,
    }
}
fn take_bool(object: &mut BTreeMap<String, Value>, key: &str) -> Option<bool> {
    match object.remove(key)? {
        Value::Bool(value) => Some(value),
        _ => None,
    }
}
fn take_string(object: &mut BTreeMap<String, Value>, key: &str) -> Option<String> {
    match object.remove(key)? {
        Value::String(value) => Some(value),
        _ => None,
    }
}
fn take_optional_string(object: &mut BTreeMap<String, Value>, key: &str) -> Option<String> {
    match object.remove(key)? {
        Value::String(value) => Some(value),
        Value::Null => None,
        _ => None,
    }
}
fn take_string_array(object: &mut BTreeMap<String, Value>, key: &str) -> Option<Vec<String>> {
    match object.remove(key)? {
        Value::Array(values) => values
            .into_iter()
            .map(|value| match value {
                Value::String(value) => Some(value),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}
fn string_map(value: Value) -> Option<BTreeMap<String, String>> {
    match value {
        Value::Object(values) => values
            .into_iter()
            .map(|(key, value)| match value {
                Value::String(value) => Some((key, value)),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

fn positions(value: Value) -> Option<Vec<DesktopPosition>> {
    let Value::Array(values) = value else {
        return None;
    };
    values
        .into_iter()
        .map(|value| {
            let Value::Object(mut object) = value else {
                return None;
            };
            Some(DesktopPosition {
                monitor_id: take_string(&mut object, "monitor_id")?,
                item_id: take_string(&mut object, "item_id")?,
                logical_x: i32::try_from(take_i64(&mut object, "logical_x")?).ok()?,
                logical_y: i32::try_from(take_i64(&mut object, "logical_y")?).ok()?,
                layout_revision: take_u64(&mut object, "layout_revision")?,
            })
        })
        .collect()
}

fn take_i64(object: &mut BTreeMap<String, Value>, key: &str) -> Option<i64> {
    match object.remove(key)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}
fn position_value(position: &DesktopPosition) -> Value {
    Value::Object(BTreeMap::from([
        ("item_id".into(), Value::String(position.item_id.clone())),
        (
            "layout_revision".into(),
            Value::Number(position.layout_revision as i64),
        ),
        (
            "logical_x".into(),
            Value::Number(i64::from(position.logical_x)),
        ),
        (
            "logical_y".into(),
            Value::Number(i64::from(position.logical_y)),
        ),
        (
            "monitor_id".into(),
            Value::String(position.monitor_id.clone()),
        ),
    ]))
}
fn wallpaper_mode(value: &str) -> Option<WallpaperMode> {
    match value {
        "fill" => Some(WallpaperMode::Fill),
        "fit" => Some(WallpaperMode::Fit),
        "stretch" => Some(WallpaperMode::Stretch),
        "center" => Some(WallpaperMode::Center),
        "tile" => Some(WallpaperMode::Tile),
        "span" => Some(WallpaperMode::Span),
        _ => None,
    }
}
fn wallpaper_mode_name(value: WallpaperMode) -> &'static str {
    match value {
        WallpaperMode::Fill => "fill",
        WallpaperMode::Fit => "fit",
        WallpaperMode::Stretch => "stretch",
        WallpaperMode::Center => "center",
        WallpaperMode::Tile => "tile",
        WallpaperMode::Span => "span",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_v1_round_trip_preserves_every_field_and_unknown_top_level() {
        let input = r#"{"accessibility":{"high_contrast":true,"reduce_motion":true,"text_scale_percent":150},"desktop_positions":[{"item_id":"item","layout_revision":3,"logical_x":-4,"logical_y":8,"monitor_id":"monitor"}],"execution_preference":"shell","future":{"kept":true},"monitor_mapping":{"old":"new"},"revision":9,"schema_version":1,"start":{"initialized":true,"pinned_ids":["app:a"],"recent_ids":["app:b"]},"superexplorer_path":"C:\\SuperExplorer.exe","taskbar":{"pins":["app"],"rows":3},"theme":"dark","wallpaper":{"mode":"span","source":"wall.jpg"}}"#;
        let decoded = SettingsV1::decode(input).unwrap();
        assert!(decoded.corrections.is_empty());
        assert_eq!(
            SettingsV1::decode(&decoded.settings.encode())
                .unwrap()
                .settings,
            decoded.settings
        );
        assert!(decoded.settings.encode().contains("\"future\""));
        assert_eq!(decoded.settings.start.pinned_ids, vec!["app:a"]);
        assert_eq!(decoded.settings.start.recent_ids, vec!["app:b"]);
        assert!(decoded.settings.start.initialized);
    }

    #[test]
    fn shell_preference_never_bypasses_per_launch_opt_in() {
        let settings = SettingsV1 {
            execution_preference: ExecutionPreference::Shell,
            ..SettingsV1::default()
        };
        assert_eq!(settings.effective_mode(false), RuntimeMode::Preview);
        assert_eq!(settings.effective_mode(true), RuntimeMode::Shell);
    }

    #[test]
    fn desktop_sort_preference_round_trips_with_manual_positions() {
        let settings = SettingsV1 {
            desktop: DesktopSettings {
                sort_key: DesktopSortKey::Modified,
                sort_direction: DesktopSortDirection::Descending,
            },
            desktop_positions: vec![DesktopPosition {
                monitor_id: "display-1".into(),
                item_id: "desktop-item:a".into(),
                logical_x: 104,
                logical_y: 224,
                layout_revision: 8,
            }],
            ..SettingsV1::default()
        };
        let decoded = SettingsV1::decode(&settings.encode()).unwrap().settings;
        assert_eq!(decoded.desktop, settings.desktop);
        assert_eq!(decoded.desktop_positions, settings.desktop_positions);
    }

    #[test]
    fn invalid_independent_fields_fall_back_without_losing_others() {
        let input = r#"{"schema_version":1,"revision":7,"taskbar":{"rows":9,"pins":["kept"]},"accessibility":{"text_scale_percent":999}}"#;
        let decoded = SettingsV1::decode(input).unwrap();
        assert_eq!(decoded.settings.revision, 7);
        assert_eq!(decoded.settings.taskbar.rows, 2);
        assert_eq!(decoded.settings.taskbar.pins, vec!["kept"]);
        assert_eq!(
            decoded.corrections,
            vec![
                SettingsCorrection::TaskbarRows,
                SettingsCorrection::TextScale
            ]
        );
    }

    #[test]
    fn migrates_v0_and_refuses_future_version() {
        let migrated = SettingsV1::decode(r#"{"revision":2}"#).unwrap();
        assert_eq!(migrated.corrections, vec![SettingsCorrection::MigratedV0]);
        assert_eq!(
            SettingsV1::decode(r#"{"schema_version":2}"#),
            Err(SettingsError::UnsupportedFutureVersion(2))
        );
    }
}
