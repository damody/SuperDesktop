use std::{collections::BTreeSet, rc::Rc};

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Toggled, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use settings_store::{TaskbarAlignment, TaskbarSearchMode, TaskbarSettings};

fn traditional_chinese() -> bool {
    std::env::var("SUPERDESKTOP_LOCALE").as_deref() == Ok("zh-TW")
        || platform_win::common::taskbar_status::user_locale_name()
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskbarContextCommand {
    ToggleLockTaskbar,
    OpenTaskManager,
    OpenTaskbarSettings,
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
    pub const COMMANDS: [TaskbarContextCommand; 3] = [
        TaskbarContextCommand::ToggleLockTaskbar,
        TaskbarContextCommand::OpenTaskManager,
        TaskbarContextCommand::OpenTaskbarSettings,
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
    action: TaskbarContextAction,
    dismiss: TaskbarSurfaceDismiss,
    focus: FocusHandle,
}

impl TaskbarContextView {
    pub fn new(
        locked: bool,
        action: TaskbarContextAction,
        dismiss: TaskbarSurfaceDismiss,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            model: TaskbarContextModel::default(),
            locked,
            action,
            dismiss,
            focus: cx.focus_handle(),
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
            .h(px(114.))
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
                    let (icon, label) = match command {
                        TaskbarContextCommand::ToggleLockTaskbar => (
                            if self.locked { "✓" } else { "" },
                            if traditional_chinese() {
                                "鎖定工作列"
                            } else {
                                "Lock the taskbar"
                            },
                        ),
                        TaskbarContextCommand::OpenTaskManager => (
                            "▥",
                            if traditional_chinese() {
                                "工作管理員"
                            } else {
                                "Task Manager"
                            },
                        ),
                        TaskbarContextCommand::OpenTaskbarSettings => (
                            "⚙",
                            if traditional_chinese() {
                                "工作列設定"
                            } else {
                                "Taskbar settings"
                            },
                        ),
                    };
                    div()
                        .id(format!("taskbar-context-{command:?}"))
                        .role(gpui::Role::MenuItem)
                        .aria_label(if command == TaskbarContextCommand::ToggleLockTaskbar {
                            format!(
                                "{label}, {}",
                                if self.locked {
                                    "checked"
                                } else {
                                    "not checked"
                                }
                            )
                        } else {
                            label.into()
                        })
                        .aria_selected(
                            command == TaskbarContextCommand::ToggleLockTaskbar && self.locked,
                        )
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
                "Choose where taskbar buttons appear",
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
        TaskbarSettingId::Alignment => "選擇工作列按鈕顯示位置",
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
        }
    }

    fn apply(&mut self, effect: TaskbarSettingsEffect) {
        match (self.action)(effect) {
            Ok((settings, revision)) => self.model.apply_saved(settings, revision),
            Err(error) => self.model.reject(error),
        }
    }
}

impl Render for TaskbarSettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let high_contrast = std::env::var("SUPERDESKTOP_THEME").as_deref() == Ok("high-contrast");
        let dark = std::env::var("SUPERDESKTOP_THEME").as_deref() == Ok("dark");
        let background = if high_contrast {
            0x000000
        } else if dark {
            0x202020
        } else {
            0xf3f3f3
        };
        let card = if high_contrast {
            0x000000
        } else if dark {
            0x2b2b2b
        } else {
            0xfbfbfb
        };
        let foreground = if dark || high_contrast {
            0xffffff
        } else {
            0x1b1b1b
        };
        let secondary = if dark || high_contrast {
            0xc8c8c8
        } else {
            0x626262
        };
        let border = if high_contrast {
            0xffffff
        } else if dark {
            0x454545
        } else {
            0xe0e0e0
        };
        let zh = traditional_chinese();
        let dismiss = self.dismiss.clone();
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
        div()
            .id("owned-taskbar-settings")
            .role(gpui::Role::Dialog)
            .aria_label(if zh { "個人化，工作列" } else { "Personalization, Taskbar" })
            .tab_index(0)
            .track_focus(&self.focus)
            .absolute()
            .left_0()
            .top_0()
            .w(px(900.))
            .h(px(760.))
            .bg(rgb(background))
            .text_color(rgb(foreground))
            .overflow_x_hidden()
            .overflow_y_scroll()
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "up" => this.model.move_focus(-1),
                    "down" => this.model.move_focus(1),
                    "enter" | "space" => {
                        if let Some(row) = this.model.rows().get(this.model.focused_row()).cloned()
                            && let Some(effect) = this.model.activate(row.id)
                        { this.apply(effect); }
                    }
                    "escape" => dismiss(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                div().w(px(836.)).min_w_0().p(px(32.)).flex().flex_col().gap(px(16.))
                    .child(div().text_size(px(14.)).text_color(rgb(secondary)).child(if zh { "個人化  ›  工作列" } else { "Personalization  ›  Taskbar" }))
                    .child(div().text_size(px(28.)).child(if zh { "工作列" } else { "Taskbar" }))
                    .child(
                        div().p(px(16.)).rounded(px(8.)).border_1().border_color(rgb(border)).bg(rgb(card))
                            .flex().items_center().gap(px(12.)).child("ⓘ").child(div().flex_1().min_w_0().whitespace_normal().child(if zh { "部分 Windows 內建介面仍待 SuperDesktop 完整接管。" } else { "Some Windows inbox surfaces are unavailable until SuperDesktop owns them." })),
                    )
                    .when_some(self.model.error().map(str::to_owned), |element, error| {
                        element.child(div().id("taskbar-settings-error").role(gpui::Role::Alert).aria_label(error.clone()).p(px(12.)).rounded(px(8.)).bg(rgb(0x5c1a1a)).text_color(rgb(0xffffff)).child(error))
                    })
                    .children(sections.into_iter().map(|(section, title, description)| {
                        let rows = self.model.rows().into_iter().filter(|row| row.section == section).collect::<Vec<_>>();
                        let expanded = self.model.expanded(section);
                        div().w_full().min_w_0().rounded(px(8.)).border_1().border_color(rgb(border)).bg(rgb(card)).overflow_hidden().flex().flex_col()
                            .child(
                                div().id(format!("taskbar-settings-section-{section:?}")).role(gpui::Role::Button).aria_label(format!("{title}, {}", if expanded { "expanded" } else { "collapsed" })).tab_index(0)
                                    .h(px(64.)).px(px(16.)).flex().items_center().cursor_pointer()
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
                                    .min_h(px(56.)).px(px(16.)).py(px(10.)).border_t_1().border_color(rgb(border)).flex().items_center().gap(px(16.))
                                    .when(!enabled, |element| element.opacity(0.5))
                                    .when(enabled, |element| element.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| { if let Some(effect) = this.model.activate(id) { this.apply(effect); } cx.notify(); })))
                                    .child(div().flex_1().min_w_0().flex().flex_col().child(title).child(div().min_w_0().whitespace_normal().text_size(px(12.)).text_color(rgb(secondary)).child(if zh && !enabled { "此功能尚未由 SuperDesktop 擁有".to_owned() } else { description })))
                                    .child(
                                        div().flex_none()
                                            .when(is_switch, |element| element.w(px(44.)).h(px(24.)).p(px(3.)).rounded_full().bg(rgb(if switch_on { 0x0067c0 } else { 0x8a8a8a })).flex().items_center().when(switch_on, |element| element.justify_end()).child(div().w(px(18.)).h(px(18.)).rounded_full().bg(rgb(0xffffff))))
                                            .when(!is_switch, |element| element.px(px(10.)).py(px(6.)).rounded(px(6.)).border_1().border_color(rgb(border)).child(value)),
                                    )
                            })))
                    }))
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
            TaskbarContextEffect::Command(TaskbarContextCommand::ToggleLockTaskbar)
        );
        model.move_selection(-1);
        assert_eq!(
            model.activate(),
            TaskbarContextEffect::Command(TaskbarContextCommand::OpenTaskbarSettings)
        );
        model.move_selection(1);
        assert_eq!(model.selected(), 0);
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
        ] {
            assert!(source.contains(token), "missing {token}");
        }
    }
}
