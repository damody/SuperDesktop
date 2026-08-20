use std::{collections::BTreeSet, rc::Rc};

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Render, ScrollHandle, StatefulInteractiveElement, Styled,
    Subscription, Toggled, Window, canvas, div, point, prelude::FluentBuilder as _, px, rgb,
};
use settings_store::{TaskbarAlignment, TaskbarSearchMode, TaskbarSettings};

fn traditional_chinese() -> bool {
    if let Ok(locale) = std::env::var("SUPERDESKTOP_LOCALE") {
        return locale.eq_ignore_ascii_case("zh-TW");
    }
    platform_win::common::taskbar_status::user_locale_name()
        .is_some_and(|locale| locale.eq_ignore_ascii_case("zh-TW"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CommandSurfaceTokens {
    pub background: u32,
    pub foreground: u32,
    pub border: u32,
    pub selected: u32,
    pub selected_foreground: u32,
    pub destructive: u32,
}

impl CommandSurfaceTokens {
    pub fn current() -> Self {
        let theme = std::env::var("SUPERDESKTOP_THEME");
        let high_contrast = theme.as_deref() == Ok("high-contrast");
        let dark = theme.as_deref() == Ok("dark");
        Self {
            background: if high_contrast {
                0x000000
            } else if dark {
                0x252525
            } else {
                0xf9f9f9
            },
            foreground: if dark || high_contrast {
                0xffffff
            } else {
                0x1b1b1b
            },
            border: if high_contrast {
                0xffffff
            } else if dark {
                0x454545
            } else {
                0xe0e0e0
            },
            selected: if high_contrast {
                0xffff00
            } else if dark {
                0x3b3b3b
            } else {
                0xe9e9e9
            },
            selected_foreground: if high_contrast {
                0x000000
            } else if dark {
                0xffffff
            } else {
                0x1b1b1b
            },
            destructive: if high_contrast { 0xff00ff } else { 0xc42b1c },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TaskbarSettingsLayout {
    pub outer_padding: f32,
    pub content_width: f32,
    pub bottom_padding: f32,
}

impl TaskbarSettingsLayout {
    pub fn for_width(width: f32) -> Self {
        let width = width.max(1.0);
        let outer_padding = if width < 720.0 { 16.0 } else { 32.0 };
        Self {
            outer_padding,
            content_width: (width - outer_padding * 2.0).clamp(1.0, 1000.0),
            bottom_padding: 48.0,
        }
    }
}

const SETTINGS_SCROLLBAR_TRACK_TOP: f32 = 8.0;
const SETTINGS_SCROLLBAR_TRACK_BOTTOM: f32 = 8.0;
const SETTINGS_SCROLLBAR_MIN_THUMB: f32 = 48.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct TaskbarSettingsScrollbarGeometry {
    track_height: f32,
    thumb_height: f32,
    thumb_top: f32,
    progress: f32,
    max_offset: f32,
}

fn taskbar_settings_scrollbar_geometry(
    viewport_height: f32,
    max_offset: f32,
    offset_y: f32,
) -> Option<TaskbarSettingsScrollbarGeometry> {
    let max_offset = max_offset.max(0.0);
    let track_height =
        (viewport_height - SETTINGS_SCROLLBAR_TRACK_TOP - SETTINGS_SCROLLBAR_TRACK_BOTTOM).max(0.0);
    if max_offset <= f32::EPSILON || track_height <= SETTINGS_SCROLLBAR_MIN_THUMB {
        return None;
    }
    let content_height = viewport_height + max_offset;
    let thumb_height = (track_height * viewport_height / content_height)
        .clamp(SETTINGS_SCROLLBAR_MIN_THUMB, track_height);
    let movable_height = (track_height - thumb_height).max(0.0);
    let progress = (-offset_y / max_offset).clamp(0.0, 1.0);
    Some(TaskbarSettingsScrollbarGeometry {
        track_height,
        thumb_height,
        thumb_top: progress * movable_height,
        progress,
        max_offset,
    })
}

fn taskbar_settings_offset_for_thumb(
    geometry: TaskbarSettingsScrollbarGeometry,
    thumb_top: f32,
) -> f32 {
    let movable_height = (geometry.track_height - geometry.thumb_height).max(0.0);
    if movable_height <= f32::EPSILON {
        return 0.0;
    }
    -geometry.max_offset * (thumb_top / movable_height).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TaskbarSettingsTokens {
    pub background: u32,
    pub card: u32,
    pub foreground: u32,
    pub secondary: u32,
    pub border: u32,
    pub focus: u32,
    pub switch_on: u32,
    pub switch_off: u32,
    pub card_radius: u8,
    pub section_height: u8,
    pub row_height: u8,
}

impl TaskbarSettingsTokens {
    pub fn current() -> Self {
        let theme = std::env::var("SUPERDESKTOP_THEME");
        Self::for_theme(
            theme.as_deref() == Ok("dark"),
            theme.as_deref() == Ok("high-contrast"),
        )
    }

    pub const fn for_theme(dark: bool, high_contrast: bool) -> Self {
        Self {
            background: if high_contrast {
                0x000000
            } else if dark {
                0x202020
            } else {
                0xf3f3f3
            },
            card: if high_contrast {
                0x000000
            } else if dark {
                0x2b2b2b
            } else {
                0xfbfbfb
            },
            foreground: if dark || high_contrast {
                0xffffff
            } else {
                0x1b1b1b
            },
            secondary: if dark || high_contrast {
                0xc8c8c8
            } else {
                0x626262
            },
            border: if high_contrast {
                0xffffff
            } else if dark {
                0x454545
            } else {
                0xe0e0e0
            },
            focus: if high_contrast { 0xffff00 } else { 0x0067c0 },
            switch_on: if high_contrast { 0xffff00 } else { 0x0067c0 },
            switch_off: if high_contrast {
                0x000000
            } else if dark {
                0x6b6b6b
            } else {
                0x8a8a8a
            },
            card_radius: 8,
            section_height: 64,
            row_height: 56,
        }
    }
}

const fn setting_glyph(id: TaskbarSettingId) -> &'static str {
    match id {
        TaskbarSettingId::Search => "⌕",
        TaskbarSettingId::TaskView => "▣",
        TaskbarSettingId::Widgets => "▦",
        TaskbarSettingId::PenMenu => "✎",
        TaskbarSettingId::TouchKeyboard => "⌨",
        TaskbarSettingId::OtherTrayIcons => "⋯",
        TaskbarSettingId::Alignment => "↔",
        TaskbarSettingId::Labels => "Aa",
        TaskbarSettingId::CombineGroups => "▤",
        TaskbarSettingId::Previews => "▧",
        TaskbarSettingId::AllMonitors => "▣",
        TaskbarSettingId::Locked => "◇",
        TaskbarSettingId::Rows => "≡",
        TaskbarSettingId::AutoHide => "↕",
        TaskbarSettingId::DateTime => "◷",
        TaskbarSettingId::Notifications => "♧",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskbarContextCommand {
    CycleSearchMode,
    ToggleTaskView,
    ShowDesktop,
    ToggleLockTaskbar,
    OpenTaskManager,
    OpenTaskbarSettings,
    ReturnToDefaultExplorer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskbarContextEffect {
    Command(TaskbarContextCommand),
    Dismiss,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TaskbarContextModel {
    selected: usize,
}

impl TaskbarContextModel {
    pub const COMMANDS: [TaskbarContextCommand; 7] = [
        TaskbarContextCommand::CycleSearchMode,
        TaskbarContextCommand::ToggleTaskView,
        TaskbarContextCommand::ShowDesktop,
        TaskbarContextCommand::OpenTaskManager,
        TaskbarContextCommand::ToggleLockTaskbar,
        TaskbarContextCommand::OpenTaskbarSettings,
        TaskbarContextCommand::ReturnToDefaultExplorer,
    ];

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn move_selection(&mut self, delta: i32) {
        self.selected =
            (self.selected as i32 + delta).rem_euclid(Self::COMMANDS.len() as i32) as usize;
    }

    pub fn activate(&self) -> TaskbarContextEffect {
        TaskbarContextEffect::Command(Self::COMMANDS[self.selected])
    }
}

pub type TaskbarContextAction = Rc<dyn Fn(TaskbarContextCommand, &mut gpui::App)>;
pub type TaskbarSurfaceDismiss = Rc<dyn Fn(&mut Window, &mut gpui::App)>;

pub struct TaskbarContextView {
    pub model: TaskbarContextModel,
    pub locked: bool,
    pub search_mode: TaskbarSearchMode,
    pub show_task_view: bool,
    action: TaskbarContextAction,
    dismiss: TaskbarSurfaceDismiss,
    focus: FocusHandle,
    _activation_subscription: Subscription,
}

impl TaskbarContextView {
    pub fn new(
        locked: bool,
        search_mode: TaskbarSearchMode,
        show_task_view: bool,
        action: TaskbarContextAction,
        dismiss: TaskbarSurfaceDismiss,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                let dismiss = this.dismiss.clone();
                dismiss(window, cx);
            }
        });
        Self {
            model: TaskbarContextModel::default(),
            locked,
            search_mode,
            show_task_view,
            action,
            dismiss,
            focus: cx.focus_handle(),
            _activation_subscription: activation_subscription,
        }
    }
}

impl Render for TaskbarContextView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let tokens = CommandSurfaceTokens::current();
        let action_for_key = self.action.clone();
        let dismiss_for_key = self.dismiss.clone();
        div()
            .id("owned-taskbar-context-menu")
            .role(gpui::Role::Menu)
            .aria_label("Taskbar context menu")
            .tab_index(0)
            .track_focus(&self.focus)
            .absolute()
            .left_0()
            .top_0()
            .w(px(220.))
            .h(px(244.))
            .p(px(4.))
            .rounded(px(8.))
            .border_1()
            .border_color(rgb(tokens.border))
            .bg(rgb(tokens.background))
            .text_color(rgb(tokens.foreground))
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(2.))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "up" => this.model.move_selection(-1),
                        "down" => this.model.move_selection(1),
                        "enter" | "space" => {
                            let TaskbarContextEffect::Command(command) = this.model.activate()
                            else {
                                return;
                            };
                            action_for_key(command, cx);
                            dismiss_for_key(window, cx);
                        }
                        "escape" => dismiss_for_key(window, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .children(TaskbarContextModel::COMMANDS.into_iter().enumerate().map(
                |(index, command)| {
                    let action = self.action.clone();
                    let dismiss = self.dismiss.clone();
                    let icon = match command {
                        TaskbarContextCommand::CycleSearchMode => "⌕",
                        TaskbarContextCommand::ToggleTaskView => "▣",
                        TaskbarContextCommand::ShowDesktop => "▭",
                        TaskbarContextCommand::ToggleLockTaskbar => {
                            if self.locked {
                                "✓"
                            } else {
                                ""
                            }
                        }
                        TaskbarContextCommand::OpenTaskManager => "▥",
                        TaskbarContextCommand::OpenTaskbarSettings => "⚙",
                        TaskbarContextCommand::ReturnToDefaultExplorer => "↩",
                    };
                    let label =
                        taskbar_context_label(command, self.search_mode, traditional_chinese());
                    let checked =
                        taskbar_context_checked(command, self.locked, self.show_task_view);
                    div()
                        .id(format!("taskbar-context-{command:?}"))
                        .role(gpui::Role::MenuItem)
                        .aria_label(if let Some(checked) = checked {
                            format!(
                                "{label}, {}",
                                if checked { "checked" } else { "not checked" }
                            )
                        } else {
                            label.clone()
                        })
                        .aria_selected(checked.unwrap_or(false))
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(4.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .cursor_pointer()
                        .when(self.model.selected() == index, |element| {
                            element
                                .bg(rgb(tokens.selected))
                                .text_color(rgb(tokens.selected_foreground))
                        })
                        .on_click(cx.listener(move |_, _, window, cx| {
                            action(command, cx);
                            dismiss(window, cx);
                        }))
                        .child(div().w(px(16.)).flex_none().child(icon))
                        .child(label)
                },
            ))
    }
}

fn taskbar_context_checked(
    command: TaskbarContextCommand,
    locked: bool,
    show_task_view: bool,
) -> Option<bool> {
    match command {
        TaskbarContextCommand::ToggleTaskView => Some(show_task_view),
        TaskbarContextCommand::ToggleLockTaskbar => Some(locked),
        _ => None,
    }
}

fn taskbar_context_label(
    command: TaskbarContextCommand,
    search_mode: TaskbarSearchMode,
    zh_tw: bool,
) -> String {
    match (command, zh_tw) {
        (TaskbarContextCommand::CycleSearchMode, true) => format!(
            "搜尋：{}",
            match search_mode {
                TaskbarSearchMode::Hidden => "隱藏",
                TaskbarSearchMode::Icon => "僅搜尋圖示",
                TaskbarSearchMode::Box => "搜尋方塊",
            }
        ),
        (TaskbarContextCommand::CycleSearchMode, false) => format!(
            "Search: {}",
            match search_mode {
                TaskbarSearchMode::Hidden => "Hidden",
                TaskbarSearchMode::Icon => "Search icon only",
                TaskbarSearchMode::Box => "Search box",
            }
        ),
        (TaskbarContextCommand::ToggleTaskView, true) => "顯示工作檢視按鈕".into(),
        (TaskbarContextCommand::ToggleTaskView, false) => "Show Task View button".into(),
        (TaskbarContextCommand::ShowDesktop, true) => "顯示桌面".into(),
        (TaskbarContextCommand::ShowDesktop, false) => "Show the desktop".into(),
        (TaskbarContextCommand::OpenTaskManager, true) => "工作管理員".into(),
        (TaskbarContextCommand::OpenTaskManager, false) => "Task Manager".into(),
        (TaskbarContextCommand::ToggleLockTaskbar, true) => "鎖定工作列".into(),
        (TaskbarContextCommand::ToggleLockTaskbar, false) => "Lock the taskbar".into(),
        (TaskbarContextCommand::OpenTaskbarSettings, true) => "工作列設定".into(),
        (TaskbarContextCommand::OpenTaskbarSettings, false) => "Taskbar settings".into(),
        (TaskbarContextCommand::ReturnToDefaultExplorer, true) => "回到預設 Explorer".into(),
        (TaskbarContextCommand::ReturnToDefaultExplorer, false) => {
            "Return to default Explorer".into()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TaskbarSettingsSection {
    Items,
    SystemTray,
    OtherTray,
    Behaviors,
    Related,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskbarSettingId {
    Search,
    TaskView,
    Widgets,
    Labels,
    CombineGroups,
    Previews,
    AllMonitors,
    Locked,
    Rows,
    Alignment,
    PenMenu,
    TouchKeyboard,
    AutoHide,
    OtherTrayIcons,
    DateTime,
    Notifications,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskbarSettingRow {
    pub id: TaskbarSettingId,
    pub section: TaskbarSettingsSection,
    pub title: &'static str,
    pub description: &'static str,
    pub value: String,
    pub enabled: bool,
    pub unavailable_reason: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskbarSettingsEffect {
    Save {
        candidate: TaskbarSettings,
        base_revision: u64,
    },
    OpenOtherTrayIcons,
    OpenRelated(TaskbarSettingId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskbarSettingsModel {
    authoritative: TaskbarSettings,
    revision: u64,
    expanded: BTreeSet<TaskbarSettingsSection>,
    focused_row: usize,
    error: Option<String>,
}

impl TaskbarSettingsModel {
    pub fn new(authoritative: TaskbarSettings, revision: u64) -> Self {
        Self {
            authoritative,
            revision,
            expanded: [
                TaskbarSettingsSection::Items,
                TaskbarSettingsSection::SystemTray,
                TaskbarSettingsSection::OtherTray,
                TaskbarSettingsSection::Behaviors,
                TaskbarSettingsSection::Related,
            ]
            .into_iter()
            .collect(),
            focused_row: 0,
            error: None,
        }
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }
    pub fn settings(&self) -> &TaskbarSettings {
        &self.authoritative
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn expanded(&self, section: TaskbarSettingsSection) -> bool {
        self.expanded.contains(&section)
    }
    pub fn focused_row(&self) -> usize {
        self.focused_row
    }

    pub fn toggle_section(&mut self, section: TaskbarSettingsSection) {
        if !self.expanded.remove(&section) {
            self.expanded.insert(section);
        }
    }

    pub fn move_focus(&mut self, delta: i32) {
        let length = self.rows().len();
        if length > 0 {
            self.focused_row = (self.focused_row as i32 + delta).rem_euclid(length as i32) as usize;
        }
    }

    pub fn rows(&self) -> Vec<TaskbarSettingRow> {
        let s = &self.authoritative;
        vec![
            row(
                TaskbarSettingId::Search,
                TaskbarSettingsSection::Items,
                "Search",
                "Show or hide the Search control",
                match s.search_mode {
                    TaskbarSearchMode::Hidden => "Hidden",
                    TaskbarSearchMode::Icon => "Search icon only",
                    TaskbarSearchMode::Box => "Search box",
                },
                true,
                None,
            ),
            row(
                TaskbarSettingId::TaskView,
                TaskbarSettingsSection::Items,
                "Task View",
                "Show the Task View button",
                on_off(s.show_task_view),
                true,
                None,
            ),
            row(
                TaskbarSettingId::Widgets,
                TaskbarSettingsSection::Items,
                "Widgets",
                "Show the Widgets button",
                "Off",
                false,
                Some("Widgets are not yet owned by SuperDesktop"),
            ),
            row(
                TaskbarSettingId::PenMenu,
                TaskbarSettingsSection::SystemTray,
                "Pen menu",
                "Show the pen menu icon when a pen is in use",
                "Off",
                false,
                Some("Pen menu ownership is unavailable"),
            ),
            row(
                TaskbarSettingId::TouchKeyboard,
                TaskbarSettingsSection::SystemTray,
                "Touch keyboard",
                "Show the touch keyboard icon",
                "Never",
                false,
                Some("Touch keyboard ownership is unavailable"),
            ),
            row(
                TaskbarSettingId::OtherTrayIcons,
                TaskbarSettingsSection::OtherTray,
                "Other system tray icons",
                "Choose icons shown in the overflow",
                "Open",
                true,
                None,
            ),
            row(
                TaskbarSettingId::Alignment,
                TaskbarSettingsSection::Behaviors,
                "Taskbar alignment",
                "Choose where taskbar buttons and Start appear",
                match s.alignment {
                    TaskbarAlignment::Left => "Left",
                    TaskbarAlignment::Center => "Center",
                },
                true,
                None,
            ),
            row(
                TaskbarSettingId::Labels,
                TaskbarSettingsSection::Behaviors,
                "Show labels",
                "Show readable application labels",
                on_off(s.show_labels),
                true,
                None,
            ),
            row(
                TaskbarSettingId::CombineGroups,
                TaskbarSettingsSection::Behaviors,
                "Combine taskbar buttons",
                "Group windows from the same application",
                on_off(s.combine_groups),
                true,
                None,
            ),
            row(
                TaskbarSettingId::Previews,
                TaskbarSettingsSection::Behaviors,
                "Window previews",
                "Show thumbnail previews on hover",
                on_off(s.previews_enabled),
                true,
                None,
            ),
            row(
                TaskbarSettingId::AllMonitors,
                TaskbarSettingsSection::Behaviors,
                "Show on all displays",
                "Display the taskbar on every monitor",
                on_off(s.all_monitors),
                true,
                None,
            ),
            row(
                TaskbarSettingId::Locked,
                TaskbarSettingsSection::Behaviors,
                "Lock the taskbar",
                "Prevent changing the taskbar height",
                on_off(s.locked),
                true,
                None,
            ),
            row(
                TaskbarSettingId::Rows,
                TaskbarSettingsSection::Behaviors,
                "Taskbar rows",
                "Choose one, two, or three task rows",
                match s.rows {
                    1 => "1 row",
                    3 => "3 rows",
                    _ => "2 rows",
                },
                true,
                None,
            ),
            row(
                TaskbarSettingId::AutoHide,
                TaskbarSettingsSection::Behaviors,
                "Automatically hide the taskbar",
                "Hide until the pointer reaches the screen edge",
                on_off(s.auto_hide),
                true,
                None,
            ),
            row(
                TaskbarSettingId::DateTime,
                TaskbarSettingsSection::Related,
                "Date & time",
                "Time zone, clock and calendar",
                "Open",
                true,
                None,
            ),
            row(
                TaskbarSettingId::Notifications,
                TaskbarSettingsSection::Related,
                "Notifications",
                "App and system alerts",
                "Open",
                true,
                None,
            ),
        ]
        .into_iter()
        .filter(|row| self.expanded(row.section))
        .collect()
    }

    pub fn activate(&self, id: TaskbarSettingId) -> Option<TaskbarSettingsEffect> {
        let row = self.rows().into_iter().find(|row| row.id == id)?;
        if !row.enabled {
            return None;
        }
        if id == TaskbarSettingId::OtherTrayIcons {
            return Some(TaskbarSettingsEffect::OpenOtherTrayIcons);
        }
        if matches!(
            id,
            TaskbarSettingId::DateTime | TaskbarSettingId::Notifications
        ) {
            return Some(TaskbarSettingsEffect::OpenRelated(id));
        }
        let mut candidate = self.authoritative.clone();
        match id {
            TaskbarSettingId::Search => {
                candidate.search_mode = match candidate.search_mode {
                    TaskbarSearchMode::Hidden => TaskbarSearchMode::Icon,
                    TaskbarSearchMode::Icon => TaskbarSearchMode::Box,
                    TaskbarSearchMode::Box => TaskbarSearchMode::Hidden,
                }
            }
            TaskbarSettingId::TaskView => candidate.show_task_view = !candidate.show_task_view,
            TaskbarSettingId::Labels => candidate.show_labels = !candidate.show_labels,
            TaskbarSettingId::CombineGroups => candidate.combine_groups = !candidate.combine_groups,
            TaskbarSettingId::Previews => candidate.previews_enabled = !candidate.previews_enabled,
            TaskbarSettingId::AllMonitors => candidate.all_monitors = !candidate.all_monitors,
            TaskbarSettingId::Locked => candidate.locked = !candidate.locked,
            TaskbarSettingId::AutoHide => candidate.auto_hide = !candidate.auto_hide,
            TaskbarSettingId::Rows => {
                candidate.rows = if candidate.rows >= 3 {
                    1
                } else {
                    candidate.rows + 1
                }
            }
            TaskbarSettingId::Alignment => {
                candidate.alignment = match candidate.alignment {
                    TaskbarAlignment::Left => TaskbarAlignment::Center,
                    TaskbarAlignment::Center => TaskbarAlignment::Left,
                }
            }
            _ => return None,
        }
        (1..=3)
            .contains(&candidate.rows)
            .then_some(TaskbarSettingsEffect::Save {
                candidate,
                base_revision: self.revision,
            })
    }

    pub fn apply_saved(&mut self, settings: TaskbarSettings, revision: u64) {
        self.authoritative = settings;
        self.revision = revision;
        self.error = None;
    }

    pub fn reject(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }
}

fn row(
    id: TaskbarSettingId,
    section: TaskbarSettingsSection,
    title: &'static str,
    description: &'static str,
    value: &str,
    enabled: bool,
    unavailable_reason: Option<&'static str>,
) -> TaskbarSettingRow {
    TaskbarSettingRow {
        id,
        section,
        title,
        description,
        value: value.to_owned(),
        enabled,
        unavailable_reason,
    }
}

const fn on_off(value: bool) -> &'static str {
    if value { "On" } else { "Off" }
}

fn localized_row(row: &TaskbarSettingRow, zh: bool) -> (String, String, String) {
    if !zh {
        return (row.title.into(), row.description.into(), row.value.clone());
    }
    let title = match row.id {
        TaskbarSettingId::Search => "搜尋",
        TaskbarSettingId::TaskView => "工作檢視",
        TaskbarSettingId::Widgets => "小工具",
        TaskbarSettingId::PenMenu => "手寫筆功能表",
        TaskbarSettingId::TouchKeyboard => "觸控式鍵盤",
        TaskbarSettingId::OtherTrayIcons => "其他系統匣圖示",
        TaskbarSettingId::Alignment => "工作列對齊",
        TaskbarSettingId::Labels => "顯示標籤",
        TaskbarSettingId::CombineGroups => "合併工作列按鈕",
        TaskbarSettingId::Previews => "視窗預覽",
        TaskbarSettingId::AllMonitors => "在所有顯示器上顯示",
        TaskbarSettingId::Locked => "鎖定工作列",
        TaskbarSettingId::Rows => "工作列列數",
        TaskbarSettingId::AutoHide => "自動隱藏工作列",
        TaskbarSettingId::DateTime => "日期和時間",
        TaskbarSettingId::Notifications => "通知",
    };
    let description = match row.id {
        TaskbarSettingId::Search => "顯示或隱藏搜尋控制項",
        TaskbarSettingId::TaskView => "顯示工作檢視按鈕",
        TaskbarSettingId::Widgets => "顯示小工具按鈕",
        TaskbarSettingId::PenMenu => "使用手寫筆時顯示功能表圖示",
        TaskbarSettingId::TouchKeyboard => "顯示觸控式鍵盤圖示",
        TaskbarSettingId::OtherTrayIcons => "選擇溢位區域顯示的圖示",
        TaskbarSettingId::Alignment => "選擇工作列按鈕與開始選單的顯示位置",
        TaskbarSettingId::Labels => "顯示可閱讀的應用程式標籤",
        TaskbarSettingId::CombineGroups => "將相同應用程式的視窗分組",
        TaskbarSettingId::Previews => "游標停留時顯示縮圖預覽",
        TaskbarSettingId::AllMonitors => "在每個顯示器顯示工作列",
        TaskbarSettingId::Locked => "防止變更工作列高度",
        TaskbarSettingId::Rows => "選擇一列、兩列或三列工作",
        TaskbarSettingId::AutoHide => "游標到達畫面邊緣前隱藏工作列",
        TaskbarSettingId::DateTime => "時區、時鐘及行事曆",
        TaskbarSettingId::Notifications => "應用程式和系統警示",
    };
    let value = match row.value.as_str() {
        "On" => "開啟",
        "Off" => "關閉",
        "Hidden" => "隱藏",
        "Search icon only" => "僅搜尋圖示",
        "Search box" => "搜尋方塊",
        "Left" => "靠左",
        "Center" => "置中",
        "Never" => "永不",
        "Open" => "開啟",
        "1 row" => "1 列",
        "2 rows" => "2 列",
        "3 rows" => "3 列",
        other => other,
    };
    (title.into(), description.into(), value.into())
}

pub type TaskbarSettingsAction =
    Rc<dyn Fn(TaskbarSettingsEffect) -> Result<(TaskbarSettings, u64), String>>;

pub struct TaskbarSettingsView {
    pub model: TaskbarSettingsModel,
    action: TaskbarSettingsAction,
    dismiss: TaskbarSurfaceDismiss,
    focus: FocusHandle,
    scroll: ScrollHandle,
    scrollbar_drag_position: Option<Pixels>,
    scrollbar_geometry_refreshes: u8,
}

impl TaskbarSettingsView {
    pub fn new(
        settings: TaskbarSettings,
        revision: u64,
        action: TaskbarSettingsAction,
        dismiss: TaskbarSurfaceDismiss,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            model: TaskbarSettingsModel::new(settings, revision),
            action,
            dismiss,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            scrollbar_drag_position: None,
            scrollbar_geometry_refreshes: 0,
        }
    }

    fn apply(&mut self, effect: TaskbarSettingsEffect) {
        match (self.action)(effect) {
            Ok((settings, revision)) => self.model.apply_saved(settings, revision),
            Err(error) => self.model.reject(error),
        }
    }

    fn render_scrollbar(
        &mut self,
        tokens: TaskbarSettingsTokens,
        zh: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let dismiss = self.dismiss.clone();
        let close = div()
            .id("taskbar-settings-close")
            .role(gpui::Role::Button)
            .aria_label(if zh {
                "關閉工作列設定"
            } else {
                "Close Taskbar settings"
            })
            .tab_index(0)
            .absolute()
            .top(px(-38.0))
            .right(px(20.0))
            .w(px(36.0))
            .h(px(36.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(22.0))
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(tokens.border)))
            .active(move |style| style.bg(rgb(tokens.secondary)))
            .focus_visible(move |style| style.border_2().border_color(rgb(tokens.focus)))
            .on_click(cx.listener(move |_, _, window, cx| dismiss(window, cx)))
            .child("×");
        let overlay = div()
            .id("taskbar-settings-fixed-overlay")
            .absolute()
            .inset_0()
            .child(close);
        let bounds = self.scroll.bounds();
        let geometry = taskbar_settings_scrollbar_geometry(
            bounds.size.height.as_f32(),
            self.scroll.max_offset().y.as_f32(),
            self.scroll.offset().y.as_f32(),
        );
        let Some(geometry) = geometry else {
            if self.scrollbar_geometry_refreshes < 3 {
                self.scrollbar_geometry_refreshes += 1;
                let entity = cx.entity();
                window.on_next_frame(move |_, cx| cx.notify(entity.entity_id()));
            }
            return overlay;
        };
        self.scrollbar_geometry_refreshes = 3;
        let entity = cx.entity();
        let scroll_handle = self.scroll.clone();
        let track_origin_y = bounds.origin.y + px(SETTINGS_SCROLLBAR_TRACK_TOP);
        let thumb_top = px(geometry.thumb_top);
        let thumb_height = px(geometry.thumb_height);
        let scrollbar_value = f64::from(geometry.progress * 100.0);

        overlay.child(
            div()
                .id("taskbar-settings-scrollbar")
                .role(gpui::Role::ScrollBar)
                .aria_label(if zh {
                    "工作列設定捲軸"
                } else {
                    "Taskbar settings scrollbar"
                })
                .aria_min_numeric_value(0.0)
                .aria_max_numeric_value(100.0)
                .aria_numeric_value(scrollbar_value)
                .tab_index(0)
                .absolute()
                .top(px(SETTINGS_SCROLLBAR_TRACK_TOP))
                .right(px(4.0))
                .bottom(px(SETTINGS_SCROLLBAR_TRACK_BOTTOM))
                .w(px(12.0))
                .rounded_full()
                .bg(rgb(tokens.border))
                .child(
                    div()
                        .id("taskbar-settings-scrollbar-thumb")
                        .absolute()
                        .top(thumb_top)
                        .left(px(2.0))
                        .right(px(2.0))
                        .h(thumb_height)
                        .rounded_full()
                        .cursor_pointer()
                        .bg(rgb(tokens.secondary))
                        .hover(move |style| style.bg(rgb(tokens.foreground)))
                        .child(
                            canvas(
                                |_, _, _| (),
                                move |thumb_bounds, _, window, _| {
                                    window.on_mouse_event({
                                        let entity = entity.clone();
                                        move |event: &MouseDownEvent, _, _, cx| {
                                            if thumb_bounds.contains(&event.position) {
                                                entity.update(cx, |this, _| {
                                                    this.scrollbar_drag_position = Some(
                                                        event.position.y - thumb_bounds.origin.y,
                                                    );
                                                });
                                            }
                                        }
                                    });
                                    window.on_mouse_event({
                                        let entity = entity.clone();
                                        move |_: &MouseUpEvent, _, _, cx| {
                                            entity.update(cx, |this, _| {
                                                this.scrollbar_drag_position = None;
                                            });
                                        }
                                    });
                                    window.on_mouse_event(
                                        move |event: &MouseMoveEvent, _, _, cx| {
                                            if !event.dragging() {
                                                return;
                                            }
                                            let Some(drag_position) =
                                                entity.read(cx).scrollbar_drag_position
                                            else {
                                                return;
                                            };
                                            let thumb_top =
                                                event.position.y - track_origin_y - drag_position;
                                            let offset_y = taskbar_settings_offset_for_thumb(
                                                geometry,
                                                thumb_top.as_f32(),
                                            );
                                            scroll_handle.set_offset(point(px(0.0), px(offset_y)));
                                            cx.notify(entity.entity_id());
                                        },
                                    );
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
    }
}

impl Render for TaskbarSettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let tokens = TaskbarSettingsTokens::current();
        let background = tokens.background;
        let card = tokens.card;
        let foreground = tokens.foreground;
        let secondary = tokens.secondary;
        let border = tokens.border;
        let focus = tokens.focus;
        let layout = TaskbarSettingsLayout::for_width(window.bounds().size.width.as_f32());
        let zh = traditional_chinese();
        let dismiss_for_key = self.dismiss.clone();
        let scroll_for_key = self.scroll.clone();
        let sections = [
            (
                TaskbarSettingsSection::Items,
                if zh {
                    "工作列項目"
                } else {
                    "Taskbar items"
                },
                if zh {
                    "顯示或隱藏工作列上的按鈕"
                } else {
                    "Show or hide buttons that appear on the taskbar"
                },
            ),
            (
                TaskbarSettingsSection::SystemTray,
                if zh {
                    "系統匣圖示"
                } else {
                    "System tray icons"
                },
                if zh {
                    "選擇系統匣中顯示的系統圖示"
                } else {
                    "Choose system icons shown in the tray"
                },
            ),
            (
                TaskbarSettingsSection::OtherTray,
                if zh {
                    "其他系統匣圖示"
                } else {
                    "Other system tray icons"
                },
                if zh {
                    "選擇溢位區域顯示的圖示"
                } else {
                    "Choose icons shown in overflow"
                },
            ),
            (
                TaskbarSettingsSection::Behaviors,
                if zh {
                    "工作列行為"
                } else {
                    "Taskbar behaviors"
                },
                if zh {
                    "工作列對齊、標籤、預覽及顯示器"
                } else {
                    "Alignment, labels, previews and displays"
                },
            ),
            (
                TaskbarSettingsSection::Related,
                if zh {
                    "相關設定"
                } else {
                    "Related settings"
                },
                if zh {
                    "日期、時間及通知"
                } else {
                    "Date, time and notifications"
                },
            ),
        ];
        let scrollbar = self.render_scrollbar(tokens, zh, window, cx);
        div()
            .id("owned-taskbar-settings")
            .role(gpui::Role::Dialog)
            .aria_label(if zh { "個人化，工作列" } else { "Personalization, Taskbar" })
            .tab_index(0)
            .track_focus(&self.focus)
            .absolute()
            .left_0()
            .top_0()
            .w_full()
            .h_full()
            .relative()
            .bg(rgb(background))
            .text_color(rgb(foreground))
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "up" => this.model.move_focus(-1),
                    "down" => this.model.move_focus(1),
                    "pageup" => {
                        let viewport = scroll_for_key.bounds().size.height.as_f32();
                        let next = (scroll_for_key.offset().y.as_f32() + viewport * 0.8).min(0.0);
                        scroll_for_key.set_offset(point(px(0.0), px(next)));
                    }
                    "pagedown" => {
                        let viewport = scroll_for_key.bounds().size.height.as_f32();
                        let maximum = scroll_for_key.max_offset().y.as_f32();
                        let next = (scroll_for_key.offset().y.as_f32() - viewport * 0.8)
                            .max(-maximum);
                        scroll_for_key.set_offset(point(px(0.0), px(next)));
                    }
                    "home" => scroll_for_key.set_offset(point(px(0.0), px(0.0))),
                    "end" => scroll_for_key.set_offset(point(
                        px(0.0),
                        -scroll_for_key.max_offset().y,
                    )),
                    "enter" | "space" => {
                        if let Some(row) = this.model.rows().get(this.model.focused_row()).cloned()
                            && let Some(effect) = this.model.activate(row.id)
                        { this.apply(effect); }
                    }
                    "escape" => dismiss_for_key(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                div()
                    .id("taskbar-settings-window-chrome")
                    .w_full()
                    .h(px(40.0))
                    .flex_none(),
            )
            .child(
                div()
                    .id("taskbar-settings-scroll-body")
                    .relative()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("taskbar-settings-scroll-viewport")
                            .size_full()
                            .p(px(layout.outer_padding))
                            .pt(px(0.0))
                            .pr(px(layout.outer_padding + 20.0))
                            .pb(px(layout.bottom_padding))
                            .scrollbar_width(px(12.0))
                            .overflow_x_hidden()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(
                    div().w(px(layout.content_width)).min_w_0().flex_none().flex().flex_col().gap(px(16.))
                    .child(div().flex_none().text_size(px(14.)).text_color(rgb(secondary)).child(if zh { "個人化  ›  工作列" } else { "Personalization  ›  Taskbar" }))
                    .child(div().flex_none().text_size(px(28.)).child(if zh { "工作列" } else { "Taskbar" }))
                    .child(
                        div().flex_none().p(px(16.)).rounded(px(tokens.card_radius as f32)).border_1().border_color(rgb(border)).bg(rgb(card))
                            .flex().items_center().gap(px(12.)).child("ⓘ").child(div().flex_1().min_w_0().whitespace_normal().child(if zh { "部分 Windows 內建介面仍待 SuperDesktop 完整接管。" } else { "Some Windows inbox surfaces are unavailable until SuperDesktop owns them." })),
                    )
                    .when_some(self.model.error().map(str::to_owned), |element, error| {
                        element.child(div().id("taskbar-settings-error").role(gpui::Role::Alert).aria_label(error.clone()).flex_none().p(px(12.)).rounded(px(8.)).bg(rgb(0x5c1a1a)).text_color(rgb(0xffffff)).child(error))
                    })
                    .children(sections.into_iter().map(|(section, title, description)| {
                        let rows = self.model.rows().into_iter().filter(|row| row.section == section).collect::<Vec<_>>();
                        let expanded = self.model.expanded(section);
                        div().w_full().min_w_0().flex_none().rounded(px(tokens.card_radius as f32)).border_1().border_color(rgb(border)).bg(rgb(card)).overflow_hidden().flex().flex_col()
                            .child(
                                div().id(format!("taskbar-settings-section-{section:?}")).role(gpui::Role::Button).aria_label(format!("{title}, {}", if expanded { "expanded" } else { "collapsed" })).tab_index(0)
                                    .h(px(tokens.section_height as f32)).px(px(16.)).flex().items_center().cursor_pointer()
                                    .focus_visible(move |style| style.border_2().border_color(rgb(focus)))
                                    .on_click(cx.listener(move |this, _, _, cx| { this.model.toggle_section(section); cx.notify(); }))
                                    .child(div().flex_1().flex().flex_col().child(title).child(div().text_size(px(12.)).text_color(rgb(secondary)).child(description)))
                                    .child(if expanded { "⌃" } else { "⌄" }),
                            )
                            .when(expanded, |element| element.children(rows.into_iter().map(|row| {
                                let enabled = row.enabled;
                                let id = row.id;
                                let (title, description, value) = localized_row(&row, zh);
                                let is_switch = matches!(id, TaskbarSettingId::TaskView | TaskbarSettingId::Widgets | TaskbarSettingId::Labels | TaskbarSettingId::CombineGroups | TaskbarSettingId::Previews | TaskbarSettingId::AllMonitors | TaskbarSettingId::Locked | TaskbarSettingId::PenMenu | TaskbarSettingId::AutoHide);
                                let switch_on = matches!(value.as_str(), "On" | "開啟");
                                let aria = if let Some(reason) = row.unavailable_reason { format!("{title}, unavailable: {reason}") } else { format!("{title}, {value}") };
                                div().id(format!("taskbar-setting-{id:?}")).role(if is_switch { gpui::Role::CheckBox } else { gpui::Role::Button }).aria_label(aria).tab_index(0)
                                    .when(is_switch, |element| element.aria_toggled(Toggled::from(switch_on)))
                                    .min_h(px(tokens.row_height as f32)).px(px(16.)).py(px(10.)).border_t_1().border_color(rgb(border)).flex().items_center().gap(px(16.))
                                    .focus_visible(move |style| style.border_2().border_color(rgb(focus)))
                                    .when(!enabled, |element| element.opacity(0.5))
                                    .when(enabled, |element| element.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| { if let Some(effect) = this.model.activate(id) { this.apply(effect); } cx.notify(); })))
                                    .child(div().w(px(32.)).flex_none().text_size(px(18.)).text_color(rgb(secondary)).child(setting_glyph(id)))
                                    .child(div().flex_1().min_w_0().flex().flex_col().child(title).child(div().min_w_0().whitespace_normal().text_size(px(12.)).text_color(rgb(secondary)).child(if zh && !enabled { "此功能尚未由 SuperDesktop 擁有".to_owned() } else { description })))
                                    .child(
                                        div().flex_none()
                                            .when(is_switch, |element| element.w(px(44.)).h(px(24.)).p(px(3.)).rounded_full().border_1().border_color(rgb(if switch_on { tokens.switch_on } else { border })).bg(rgb(if switch_on { tokens.switch_on } else { tokens.switch_off })).flex().items_center().when(switch_on, |element| element.justify_end()).child(div().w(px(18.)).h(px(18.)).rounded_full().bg(rgb(if switch_on && tokens.switch_on == 0xffff00 { 0x000000 } else { 0xffffff }))))
                                            .when(!is_switch, |element| element.px(px(10.)).py(px(6.)).rounded(px(6.)).border_1().border_color(rgb(border)).child(value)),
                                    )
                            })))
                    }))
                )
                    )
                    .child(scrollbar),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_navigation_wraps_and_emits_one_typed_command() {
        let mut model = TaskbarContextModel::default();
        assert_eq!(
            model.activate(),
            TaskbarContextEffect::Command(TaskbarContextCommand::CycleSearchMode)
        );
        model.move_selection(-1);
        assert_eq!(
            model.activate(),
            TaskbarContextEffect::Command(TaskbarContextCommand::ReturnToDefaultExplorer)
        );
        model.move_selection(1);
        assert_eq!(model.selected(), 0);
        assert_eq!(TaskbarContextModel::COMMANDS.len(), 7);
        assert_eq!(
            TaskbarContextModel::COMMANDS[4],
            TaskbarContextCommand::ToggleLockTaskbar
        );
        assert_eq!(
            TaskbarContextModel::COMMANDS[6],
            TaskbarContextCommand::ReturnToDefaultExplorer
        );
    }

    #[test]
    fn context_menu_observes_window_deactivation_without_descendant_focus_hooks() {
        let source = include_str!("taskbar_settings.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "_activation_subscription: Subscription",
            "cx.observe_window_activation(window",
            "if !window.is_window_active()",
            "let dismiss = this.dismiss.clone()",
            "dismiss(window, cx)",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        assert!(!production.contains("cx.on_focus_out"));
        assert!(!production.contains("cx.on_blur"));
    }

    #[test]
    fn context_labels_and_checked_states_are_truthful_in_both_locales() {
        assert_eq!(
            taskbar_context_label(
                TaskbarContextCommand::CycleSearchMode,
                TaskbarSearchMode::Icon,
                false,
            ),
            "Search: Search icon only"
        );
        assert_eq!(
            taskbar_context_label(
                TaskbarContextCommand::CycleSearchMode,
                TaskbarSearchMode::Box,
                true,
            ),
            "搜尋：搜尋方塊"
        );
        assert_eq!(
            taskbar_context_label(
                TaskbarContextCommand::ToggleTaskView,
                TaskbarSearchMode::Hidden,
                true,
            ),
            "顯示工作檢視按鈕"
        );
        assert_eq!(
            taskbar_context_checked(TaskbarContextCommand::ToggleTaskView, false, true),
            Some(true)
        );
        assert_eq!(
            taskbar_context_checked(TaskbarContextCommand::ToggleLockTaskbar, false, true),
            Some(false)
        );
        assert_eq!(
            taskbar_context_checked(TaskbarContextCommand::ShowDesktop, false, true),
            None
        );
    }

    #[test]
    fn settings_model_changes_only_supported_fields_and_reconciles_revision() {
        let settings = TaskbarSettings::default();
        let mut model = TaskbarSettingsModel::new(settings.clone(), 7);
        assert_eq!(model.activate(TaskbarSettingId::Widgets), None);
        let Some(TaskbarSettingsEffect::Save {
            candidate,
            base_revision,
        }) = model.activate(TaskbarSettingId::Search)
        else {
            panic!("save effect")
        };
        assert_eq!(base_revision, 7);
        assert_eq!(candidate.search_mode, TaskbarSearchMode::Icon);
        assert_eq!(candidate.rows, settings.rows);
        assert_eq!(candidate.locked, settings.locked);
        model.apply_saved(candidate.clone(), 8);
        assert_eq!(model.settings(), &candidate);
        assert_eq!(model.revision(), 8);

        let Some(TaskbarSettingsEffect::Save {
            candidate: unlocked,
            base_revision: 8,
        }) = model.activate(TaskbarSettingId::Locked)
        else {
            panic!("lock save effect")
        };
        assert!(!unlocked.locked);
        assert_eq!(unlocked.rows, candidate.rows);

        model.apply_saved(unlocked, 9);
        let Some(TaskbarSettingsEffect::Save {
            candidate: auto_hidden,
            base_revision: 9,
        }) = model.activate(TaskbarSettingId::AutoHide)
        else {
            panic!("auto-hide save effect")
        };
        assert!(auto_hidden.auto_hide);
        assert!(!auto_hidden.locked);
        let authoritative_before_failure = model.settings().clone();
        let revision_before_failure = model.revision();
        model.reject("simulated atomic save failure");
        assert_eq!(model.settings(), &authoritative_before_failure);
        assert_eq!(model.revision(), revision_before_failure);
        assert_eq!(model.error(), Some("simulated atomic save failure"));
    }

    #[test]
    fn alignment_defaults_left_toggles_both_directions_and_describes_start() {
        let mut model = TaskbarSettingsModel::new(TaskbarSettings::default(), 12);
        assert_eq!(model.settings().alignment, TaskbarAlignment::Left);
        let row = model
            .rows()
            .into_iter()
            .find(|row| row.id == TaskbarSettingId::Alignment)
            .expect("alignment row");
        assert_eq!(row.value, "Left");
        assert_eq!(
            localized_row(&row, false).1,
            "Choose where taskbar buttons and Start appear"
        );
        assert_eq!(
            localized_row(&row, true).1,
            "選擇工作列按鈕與開始選單的顯示位置"
        );

        let Some(TaskbarSettingsEffect::Save {
            candidate: centered,
            base_revision: 12,
        }) = model.activate(TaskbarSettingId::Alignment)
        else {
            panic!("center alignment save effect")
        };
        assert_eq!(centered.alignment, TaskbarAlignment::Center);
        model.apply_saved(centered, 13);

        let Some(TaskbarSettingsEffect::Save {
            candidate: left,
            base_revision: 13,
        }) = model.activate(TaskbarSettingId::Alignment)
        else {
            panic!("left alignment save effect")
        };
        assert_eq!(left.alignment, TaskbarAlignment::Left);
    }

    #[test]
    fn unsupported_rows_explain_unavailability_and_never_mutate() {
        let model = TaskbarSettingsModel::new(TaskbarSettings::default(), 1);
        for id in [
            TaskbarSettingId::Widgets,
            TaskbarSettingId::PenMenu,
            TaskbarSettingId::TouchKeyboard,
        ] {
            let row = model.rows().into_iter().find(|row| row.id == id).unwrap();
            assert!(!row.enabled);
            assert!(row.unavailable_reason.is_some());
            assert_eq!(model.activate(id), None);
        }

        let auto_hide = model
            .rows()
            .into_iter()
            .find(|row| row.id == TaskbarSettingId::AutoHide)
            .unwrap();
        assert!(auto_hide.enabled);
        assert_eq!(auto_hide.value, "Off");
        assert_eq!(auto_hide.unavailable_reason, None);
    }

    #[test]
    fn geometry_source_contract_uses_windows11_dimensions_and_accessibility() {
        let source = include_str!("taskbar_settings.rs");
        for token in [
            ".w(px(16.))",
            ".h(px(32.))",
            ".rounded(px(8.))",
            ".shadow_lg()",
            "Role::Menu",
            "Role::Dialog",
            "Role::Alert",
            "Role::CheckBox",
            "aria_toggled",
            ".w_full()",
            ".h_full()",
            ".justify_center()",
            ".overflow_y_scroll()",
            ".focus_visible(move |style|",
        ] {
            assert!(source.contains(token), "missing {token}");
        }
    }

    #[test]
    fn responsive_layout_uses_compact_padding_and_bounded_centered_content() {
        let compact = TaskbarSettingsLayout::for_width(640.0);
        assert_eq!(compact.outer_padding, 16.0);
        assert_eq!(compact.content_width, 608.0);
        assert_eq!(compact.bottom_padding, 48.0);
        let normal = TaskbarSettingsLayout::for_width(1100.0);
        assert_eq!(normal.outer_padding, 32.0);
        assert_eq!(normal.content_width, 1000.0);
        let wide = TaskbarSettingsLayout::for_width(2200.0);
        assert_eq!(wide.content_width, 1000.0);
    }

    #[test]
    fn settings_scrollbar_geometry_is_bounded_and_handles_fit_without_division() {
        assert_eq!(taskbar_settings_scrollbar_geometry(860.0, 0.0, 0.0), None);
        assert_eq!(taskbar_settings_scrollbar_geometry(48.0, 200.0, 0.0), None);

        let top = taskbar_settings_scrollbar_geometry(860.0, 1_000.0, 0.0).unwrap();
        assert_eq!(top.progress, 0.0);
        assert_eq!(top.thumb_top, 0.0);
        assert!(top.thumb_height >= SETTINGS_SCROLLBAR_MIN_THUMB);
        assert!(top.thumb_height <= top.track_height);

        let middle = taskbar_settings_scrollbar_geometry(860.0, 1_000.0, -500.0).unwrap();
        assert!((middle.progress - 0.5).abs() < f32::EPSILON);
        assert!(middle.thumb_top > 0.0);
        assert!(middle.thumb_top < middle.track_height - middle.thumb_height);

        let bottom = taskbar_settings_scrollbar_geometry(860.0, 1_000.0, -2_000.0).unwrap();
        assert_eq!(bottom.progress, 1.0);
        assert_eq!(bottom.thumb_top, bottom.track_height - bottom.thumb_height);
        assert_eq!(taskbar_settings_offset_for_thumb(bottom, -100.0), 0.0);
        assert_eq!(
            taskbar_settings_offset_for_thumb(bottom, bottom.track_height * 2.0),
            -1_000.0
        );
    }

    #[test]
    fn settings_window_chrome_source_tracks_scroll_and_uses_shared_dismissal() {
        let source = include_str!("taskbar_settings.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "scroll: ScrollHandle",
            "scrollbar_drag_position: Option<Pixels>",
            ".track_scroll(&self.scroll)",
            "taskbar-settings-scrollbar",
            "taskbar-settings-scrollbar-thumb",
            "Role::ScrollBar",
            ".aria_min_numeric_value(0.0)",
            ".aria_max_numeric_value(100.0)",
            "taskbar-settings-close",
            "Role::Button",
            "taskbar-settings-fixed-overlay",
            "dismiss(window, cx)",
            ".scrollbar_width(px(12.0))",
            "taskbar_settings_offset_for_thumb",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        assert!(!production.contains("TitlebarOptions"));
    }

    #[test]
    fn settings_tokens_keep_geometry_stable_and_high_contrast_explicit() {
        let light = TaskbarSettingsTokens::for_theme(false, false);
        let dark = TaskbarSettingsTokens::for_theme(true, false);
        let contrast = TaskbarSettingsTokens::for_theme(false, true);
        for value in [light, dark, contrast] {
            assert_eq!(value.card_radius, 8);
            assert_eq!(value.section_height, 64);
            assert_eq!(value.row_height, 56);
        }
        assert_ne!(light.background, dark.background);
        assert_eq!(contrast.border, 0xffffff);
        assert_eq!(contrast.focus, 0xffff00);
        assert_eq!(contrast.switch_on, 0xffff00);
    }
}
