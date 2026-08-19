use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

use gpui::{
    App, AppContext, Context, CursorStyle, FocusHandle, InteractiveElement, IntoElement,
    MouseButton, ObjectFit, ParentElement, Render, RenderImage, ResizeEdge,
    StatefulInteractiveElement, Styled, StyledImage, Subscription, Window, div, img,
    prelude::FluentBuilder as _, px, rgb, svg,
};

use crate::{
    AccessibleTask, NotificationAccessibleNode, NotificationAreaModel, NotificationPlacement,
    StatusRegion, SystemFlyoutKind, SystemStatusAction, TaskOverlay, TaskVisualState,
    TaskbarLayout,
};
use settings_store::{TaskbarAlignment, TaskbarSearchMode};
use shell_provider_protocol::{
    IconData, IconKey, NotificationEventKind, StatusAvailability, SystemStatusSnapshot,
};

thread_local! {
    static ICON_RENDER_CACHE: RefCell<VecDeque<(u64, IconData, Arc<RenderImage>)>> = const { RefCell::new(VecDeque::new()) };
}

const ICON_RENDER_CACHE_LIMIT: usize = 2_048;
const SHOW_DESKTOP_CORNER_WIDTH: f32 = 8.0;
const CLOCK_CONTROL_WIDTH: f32 = 112.0;
const LEGACY_CLOCK_CONTROL_WIDTH: f32 = 82.0;

fn icon_hash(icon: &IconData) -> u64 {
    let mut hasher = DefaultHasher::new();
    icon.width.hash(&mut hasher);
    icon.height.hash(&mut hasher);
    icon.rgba.hash(&mut hasher);
    hasher.finish()
}

fn bc7_render_image(icon: &IconData) -> Option<Arc<RenderImage>> {
    let raster = platform_win::common::icon::encode_bc7(icon)?;
    RenderImage::new_bc7_srgb(gpui::CompressedRaster {
        kind: gpui::CompressedRasterKind::Icon,
        width: raster.width,
        height: raster.height,
        padded_width: raster.padded_width,
        padded_height: raster.padded_height,
        row_pitch: raster.row_pitch,
        blocks: Arc::from(raster.blocks),
    })
    .map(Arc::new)
}

fn uncached_icon_render_image(icon: &IconData) -> Option<Arc<RenderImage>> {
    if !platform_win::common::icon::valid_icon_data(icon) {
        return None;
    }
    if gpui::compressed_gpu_cache_stats().0.supported == Some(true)
        && let Some(image) = bc7_render_image(icon)
    {
        return Some(image);
    }
    // GPUI's RenderImage upload contract is BGRA on Windows even though the
    // backing image crate type is named RgbaImage.
    let mut bgra = icon.rgba.clone();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let buffer = image::RgbaImage::from_raw(icon.width, icon.height, bgra)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

pub(crate) fn icon_render_image(icon: &IconData) -> Option<Arc<RenderImage>> {
    if !platform_win::common::icon::valid_icon_data(icon) {
        return None;
    }
    let key = icon_hash(icon);
    ICON_RENDER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((_, _, image)) = cache
            .iter()
            .find(|(candidate_key, candidate, _)| *candidate_key == key && candidate == icon)
        {
            return Some(Arc::clone(image));
        }
        let image = uncached_icon_render_image(icon)?;
        if cache.len() == ICON_RENDER_CACHE_LIMIT {
            cache.pop_front();
        }
        cache.push_back((key, icon.clone(), Arc::clone(&image)));
        Some(image)
    })
}

fn task_display_label(
    name: &str,
    group_size: usize,
    show_labels: bool,
    has_real_icon: bool,
) -> String {
    if !show_labels && has_real_icon {
        return String::new();
    }
    let name = if name.trim().is_empty() {
        "Untitled"
    } else {
        name
    };
    if group_size > 1 {
        format!("{name} ({group_size})")
    } else {
        name.to_owned()
    }
}

fn adaptive_labeled_task_width(
    window_width: f32,
    left_reserved: f32,
    right_reserved: f32,
    task_count: usize,
    rows: u8,
) -> f32 {
    if task_count == 0 {
        return 160.0;
    }
    let columns = task_count.div_ceil(usize::from(rows.clamp(1, 3))).max(1) as f32;
    ((window_width - left_reserved - right_reserved).max(0.0) / columns).clamp(44.0, 160.0)
}

fn fixed_entry_indicator_width(task_width: f32) -> f32 {
    (task_width - 16.0).max(12.0)
}

fn toggled_system_flyout(
    current: Option<SystemFlyoutKind>,
    requested: SystemFlyoutKind,
) -> Option<SystemFlyoutKind> {
    (current != Some(requested)).then_some(requested)
}

fn activates_button(key: &str) -> bool {
    matches!(key, "enter" | "space")
}

fn clock_accessible_label(status: &StatusRegion, rows: u8) -> String {
    if rows >= 3 {
        format!("{} {} {}", status.time, status.weekday, status.date)
    } else {
        format!("{} {}", status.time, status.date)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TaskbarChromeTokens {
    panel: u32,
    border: u32,
    text: u32,
    secondary_text: u32,
    hover: u32,
    pressed: u32,
    focus: u32,
    active: u32,
    attention: u32,
}

impl TaskbarChromeTokens {
    fn new(theme: Option<&str>) -> Self {
        match theme {
            Some("high-contrast") => Self {
                panel: 0x000000,
                border: 0xffffff,
                text: 0xffffff,
                secondary_text: 0xffffff,
                hover: 0x1f1f1f,
                pressed: 0x333333,
                focus: 0xffff00,
                active: 0x000000,
                attention: 0x000000,
            },
            Some("dark") => Self {
                panel: 0x202020,
                border: 0x454545,
                text: 0xffffff,
                secondary_text: 0xcacaca,
                hover: 0x323232,
                pressed: 0x3e3e3e,
                focus: 0x60cdff,
                active: 0x292929,
                attention: 0x5c4700,
            },
            _ => Self {
                panel: 0xf3f3f3,
                border: 0xd8d8d8,
                text: 0x202020,
                secondary_text: 0x616161,
                hover: 0xe5e5e5,
                pressed: 0xd7d7d7,
                focus: 0x0067c0,
                active: 0xe9e9e9,
                attention: 0xffd28a,
            },
        }
    }
}

fn taskbar_search_label(locale: Option<&str>) -> &'static str {
    if locale.is_some_and(|locale| locale.eq_ignore_ascii_case("zh-TW")) {
        "搜尋"
    } else {
        "Search"
    }
}

fn compact_input_language(locale: &str) -> String {
    let normalized = locale.trim().replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh-") || normalized == "zh" {
        return "中".into();
    }
    if normalized.starts_with("en-") || normalized == "en" {
        return "ENG".into();
    }
    normalized
        .split('-')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("?")
        .chars()
        .take(3)
        .flat_map(char::to_uppercase)
        .collect()
}

pub struct TaskbarView {
    pub accessible_root_name: String,
    pub layout: TaskbarLayout,
    pub tasks: Vec<AccessibleTask>,
    pub fixed_name: String,
    pub fixed_icon: Option<IconData>,
    pub status: StatusRegion,
    pub system_snapshot: Option<SystemStatusSnapshot>,
    pub system_flyout: Option<SystemFlyoutKind>,
    pub notification_area: NotificationAreaModel,
    pub overlays: BTreeMap<String, TaskOverlay>,
    pub show_labels: bool,
    pub search_mode: TaskbarSearchMode,
    pub show_task_view: bool,
    pub alignment: TaskbarAlignment,
    pub locked: bool,
    pub callbacks: Option<TaskbarCallbacks>,
    pub keyboard_focus: Option<FocusHandle>,
    pub resize_subscription: Option<Subscription>,
}

pub type TaskCallback = Rc<dyn Fn(&str, &mut App)>;
pub type TaskHoverCallback = Rc<dyn Fn(&str, bool, &mut App)>;
pub type NotificationCallback = Rc<dyn Fn(&IconKey, NotificationEventKind)>;
pub type NotificationOverflowCallback = Rc<dyn Fn(Vec<NotificationAccessibleNode>, &mut App)>;
pub type SystemStatusCallback = Rc<dyn Fn(SystemStatusAction, &mut App)>;
pub type SystemFlyoutCallback = Rc<dyn Fn(SystemFlyoutKind, &mut App)>;
pub type TaskbarBackgroundContextCallback = Rc<dyn Fn(gpui::Point<gpui::Pixels>, &mut App)>;
pub type TaskbarResizeCallback = Rc<dyn Fn(u8, &mut Window, &mut App) -> bool>;

fn taskbar_rows_for_logical_height(height: f32) -> u8 {
    ((height / 40.0).round() as i32).clamp(1, 3) as u8
}

struct NotificationTooltip {
    text: String,
}

impl Render for NotificationTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgb(0x20242b))
            .text_color(rgb(0xffffff))
            .child(self.text.clone())
    }
}

#[derive(Clone)]
pub struct TaskbarCallbacks {
    pub start: Rc<dyn Fn(&mut App)>,
    pub show_desktop: Rc<dyn Fn(&mut App)>,
    pub task_view: Rc<dyn Fn(&mut App)>,
    pub fixed: Rc<dyn Fn()>,
    pub task: TaskCallback,
    pub task_hover: TaskHoverCallback,
    pub task_context: TaskCallback,
    pub taskbar_context: TaskbarBackgroundContextCallback,
    pub resize_rows: TaskbarResizeCallback,
    pub notification: NotificationCallback,
    pub notification_overflow: NotificationOverflowCallback,
    pub system_status: SystemStatusCallback,
    pub system_flyout: SystemFlyoutCallback,
    pub rendered: Rc<dyn Fn()>,
}

impl TaskbarView {
    pub fn attach_resize_observer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(callback) = self
            .callbacks
            .as_ref()
            .map(|callbacks| Rc::clone(&callbacks.resize_rows))
        else {
            return;
        };
        self.resize_subscription =
            Some(cx.observe_window_bounds(window, move |this, window, cx| {
                if this.locked {
                    return;
                }
                let rows = taskbar_rows_for_logical_height(window.bounds().size.height.as_f32());
                if rows != this.layout.rows.get() {
                    let _ = callback(rows, window, cx);
                }
            }));
    }
}

impl Render for TaskbarView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bar_height = px(self.layout.height / window.scale_factor());
        let start = self.callbacks.as_ref().map(|value| Rc::clone(&value.start));
        let show_desktop = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.show_desktop));
        let show_desktop_key = show_desktop.clone();
        let task_view = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.task_view));
        let fixed = self.callbacks.as_ref().map(|value| Rc::clone(&value.fixed));
        let task_callback = self.callbacks.as_ref().map(|value| Rc::clone(&value.task));
        let task_hover_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.task_hover));
        let task_context_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.task_context));
        let taskbar_context_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.taskbar_context));
        let overlays = self.overlays.clone();
        let show_labels = self.show_labels;
        let search_mode = self.search_mode;
        let show_task_view = self.show_task_view;
        let alignment = self.alignment;
        let notification_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.notification));
        let notification_overflow_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.notification_overflow));
        let notification_overflow_key_callback = notification_overflow_callback.clone();
        let system_status_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.system_status));
        let system_flyout_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.system_flyout));
        let network_flyout_callback = system_flyout_callback.clone();
        let network_flyout_key_callback = system_flyout_callback.clone();
        let volume_flyout_callback = system_flyout_callback.clone();
        let volume_flyout_key_callback = system_flyout_callback.clone();
        let input_flyout_callback = system_flyout_callback.clone();
        let input_flyout_key_callback = system_flyout_callback.clone();
        let calendar_flyout_callback = system_flyout_callback;
        let calendar_flyout_key_callback = calendar_flyout_callback.clone();
        let volume_callback = system_status_callback.clone();
        let mute_callback = system_status_callback.clone();
        let input_callback = system_status_callback.clone();
        let system_snapshot = self.system_snapshot.clone();
        // Product composition opens the flyout in its own popup window. Keep
        // the inline branch disabled so the taskbar HWND never clips it.
        let system_flyout: Option<SystemFlyoutKind> = None;
        let start_key = start.clone();
        let search_callback = start.clone();
        let task_view_key = task_view.clone();
        let fixed_key = fixed.clone();
        let root_fixed_key = fixed.clone();
        let keyboard_focus = self.keyboard_focus.clone();
        let dismiss_focus = self.keyboard_focus.clone();
        let overflow_open = false;
        let notification_nodes = self.notification_area.accessible_nodes();
        let visible_notifications = notification_nodes
            .iter()
            .filter(|node| node.placement == NotificationPlacement::Visible)
            .cloned()
            .collect::<Vec<_>>();
        let overflow_notifications = notification_nodes
            .iter()
            .filter(|node| node.placement == NotificationPlacement::Overflow)
            .cloned()
            .collect::<Vec<_>>();
        let notification_area_reserved_width = visible_notifications.len() as f32 * 36.0 + 32.0;
        let theme = std::env::var("SUPERDESKTOP_THEME").ok();
        let high_contrast = theme.as_deref() == Some("high-contrast");
        let tokens = TaskbarChromeTokens::new(theme.as_deref());
        let locale = std::env::var("SUPERDESKTOP_LOCALE")
            .ok()
            .or_else(platform_win::common::taskbar_status::user_locale_name);
        let search_label = taskbar_search_label(locale.as_deref());
        let zh_tw = search_label == "搜尋";
        let reduced_motion = std::env::var("SUPERDESKTOP_REDUCED_MOTION").as_deref() == Ok("1");
        let search_width = match search_mode {
            TaskbarSearchMode::Hidden => 0.0,
            TaskbarSearchMode::Icon => 44.0,
            TaskbarSearchMode::Box => 168.0,
        };
        let taskbar_rows = self.layout.rows.get();
        let left_reserved = 44.0 + search_width + if show_task_view { 44.0 } else { 0.0 };
        let right_reserved = 210.0
            + (CLOCK_CONTROL_WIDTH - LEGACY_CLOCK_CONTROL_WIDTH)
            + notification_area_reserved_width
            + SHOW_DESKTOP_CORNER_WIDTH;
        let adaptive_task_slots = self.tasks.len().saturating_add(1);
        let adaptive_task_width = adaptive_labeled_task_width(
            window.bounds().size.width.as_f32(),
            left_reserved,
            right_reserved,
            adaptive_task_slots,
            taskbar_rows,
        );
        let fixed_indicator_width = fixed_entry_indicator_width(adaptive_task_width);
        let start_color = if high_contrast {
            rgb(0xffff00)
        } else {
            rgb(0x000000)
        };
        if let Some(rendered) = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.rendered))
        {
            rendered();
        }
        div()
            .id("supertaskbar-root")
            .role(gpui::Role::List)
            .aria_label(self.accessible_root_name.clone())
            .tab_index(0)
            .when_some(keyboard_focus, |element, focus| element.track_focus(&focus))
            .on_mouse_up(gpui::MouseButton::Right, move |event, _, cx| {
                if let Some(callback) = &taskbar_context_callback {
                    callback(event.position, cx);
                    cx.stop_propagation();
                }
            })
            .on_key_down(_cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && this.system_flyout.take().is_some() {
                    if let Some(focus) = &dismiss_focus {
                        window.focus(focus, cx);
                    }
                    cx.notify();
                    return;
                }
                if event.keystroke.key == "tab" && event.keystroke.modifiers.platform {
                    if let Some(callback) = &task_view_key {
                        callback(cx);
                    }
                    return;
                }
                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                    && let Some(callback) = &root_fixed_key
                {
                    callback();
                }
            }))
            .size_full()
            .flex()
            .flex_row()
            .items_stretch()
            .bg(rgb(tokens.panel))
            .border_t_1()
            .border_color(rgb(tokens.border))
            .text_color(rgb(tokens.text))
            .text_size(px(13.))
            .when(!self.locked, |root| {
                root.child(
                    div()
                        .id("taskbar-resize-strip")
                        .role(gpui::Role::Button)
                        .aria_label(if zh_tw {
                            "調整工作列高度"
                        } else {
                            "Resize taskbar height"
                        })
                        .absolute()
                        .left_0()
                        .right_0()
                        .top_0()
                        .h(px(12.))
                        .cursor(CursorStyle::ResizeUpDown)
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.start_window_resize(ResizeEdge::Top);
                            cx.stop_propagation();
                        }),
                )
            })
            .child(
                div()
                    .id("notification-area")
                    .role(gpui::Role::Group)
                    .aria_label(if zh_tw { "通知區域" } else { "Notification area" })
                    .h(bar_height)
                    .absolute()
                    .right(px(210.))
                    .flex()
                    .items_center()
                    .children(visible_notifications.into_iter().map(|node| {
                        let callback = notification_callback.clone();
                        let context_callback = notification_callback.clone();
                        let key = node.key.clone();
                        let context_key = node.key.clone();
                        let tooltip = node.name.clone();
                        let icon = node.icon.as_ref().and_then(icon_render_image);
                        let has_native_icon = icon.is_some();
                        div()
                            .id(node.stable_id)
                            .role(gpui::Role::Button)
                            .aria_label(node.name.clone())
                            .tooltip(move |_, cx| {
                                let text = tooltip.clone();
                                cx.new(|_| NotificationTooltip { text }).into()
                            })
                            .tab_index(0)
                            .w(px(36.))
                            .h(px(36.))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(node.focused, |element| element.bg(rgb(0x285b8f)))
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(move |_, _, _| {
                                if let Some(callback) = &callback {
                                    callback(&key, NotificationEventKind::Activate);
                                }
                            })
                            .on_mouse_up(gpui::MouseButton::Right, move |_, _, _| {
                                if let Some(callback) = &context_callback {
                                    callback(&context_key, NotificationEventKind::Context);
                                }
                            })
                            .when_some(icon, |element, image| {
                                element.child(
                                    img(image)
                                        .w(px(24.))
                                        .h(px(24.))
                                        .object_fit(ObjectFit::Contain),
                                )
                            })
                            .when(!has_native_icon, |element| element.child("•"))
                    }))
                    .child(
                        div()
                                .id("notification-overflow-control")
                                .role(gpui::Role::Button)
                                .aria_label(if zh_tw {
                                    "顯示所有系統匣圖示"
                                } else {
                                    "Show all tray icons"
                                })
                                .tab_index(0)
                                .w(px(32.))
                                .h(px(36.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(0.))
                                .hover(move |style| style.bg(rgb(tokens.hover)))
                                .active(move |style| style.bg(rgb(tokens.pressed)))
                                .focus_visible(move |style| {
                                    style.border_2().border_color(rgb(tokens.focus))
                                })
                                .on_click(_cx.listener(move |this, _, _, cx| {
                                    if let Some(callback) = &notification_overflow_callback {
                                        callback(this.notification_area.accessible_nodes(), cx);
                                    }
                                    cx.notify();
                                }))
                                .on_key_down(_cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if activates_button(&event.keystroke.key)
                                        && let Some(callback) = &notification_overflow_key_callback
                                    {
                                        callback(this.notification_area.accessible_nodes(), cx);
                                        cx.notify();
                                    }
                                }))
                                .child(
                                    svg()
                                        .external_path(concat!(
                                            env!("CARGO_MANIFEST_DIR"),
                                            "/assets/chevron-up.svg"
                                        ))
                                        .w(px(16.))
                                        .h(px(16.))
                                        .text_color(rgb(tokens.text)),
                                )
                                .child("⌃"),
                    )
                    .when(overflow_open, |area| {
                        area.child(
                            div()
                                .id("notification-overflow")
                                .role(gpui::Role::Dialog)
                                .aria_label("Hidden notification icons")
                                .absolute()
                                .bottom(bar_height)
                                .right_0()
                                .p_2()
                                .flex()
                                .flex_wrap()
                                .w(px(220.))
                                .bg(rgb(0x20242b))
                                .children(overflow_notifications.into_iter().map(|node| {
                                    let callback = notification_callback.clone();
                                    let key = node.key.clone();
                                    let icon = node.icon.as_ref().and_then(icon_render_image);
                                    let has_native_icon = icon.is_some();
                                    div()
                                        .id(node.stable_id)
                                        .role(gpui::Role::Button)
                                        .aria_label(node.name.clone())
                                        .tab_index(0)
                                        .w(px(40.))
                                        .h(px(40.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .on_click(move |_, _, _| {
                                            if let Some(callback) = &callback {
                                                callback(&key, NotificationEventKind::Activate);
                                            }
                                        })
                                        .when_some(icon, |element, image| {
                                            element.child(
                                                img(image)
                                                    .w(px(24.))
                                                    .h(px(24.))
                                                    .object_fit(ObjectFit::Contain),
                                            )
                                        })
                                        .when(!has_native_icon, |element| element.child("•"))
                                })),
                        )
                    }),
            )
            .child(
                div()
                    .id("start-control")
                    .role(gpui::Role::Button)
                    .aria_label(if search_label == "搜尋" { "開始" } else { "Start" })
                    .tab_index(0)
                    .w(px(44.))
                    .h(bar_height)
                    .flex_none()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |style| style.bg(rgb(tokens.hover)))
                    .active(move |style| style.bg(rgb(tokens.pressed)))
                    .focus_visible(move |style| {
                        style.border_2().border_color(rgb(tokens.focus))
                    })
                    .on_click(move |_, _, cx| {
                        if let Some(callback) = &start {
                            callback(cx);
                        }
                    })
                    .on_key_down(move |event, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                            && let Some(callback) = &start_key
                        {
                            callback(cx);
                        }
                    })
                    .child(
                        svg()
                            .external_path(concat!(
                                env!("CARGO_MANIFEST_DIR"),
                                "/assets/windows-start.svg"
                            ))
                            .w(px(16.))
                            .h(px(16.))
                            .text_color(start_color),
                    ),
            )
            .when(search_mode != TaskbarSearchMode::Hidden, |element| {
                let search = search_callback.clone();
                element.child(
                    div()
                        .id("taskbar-search-control")
                        .role(gpui::Role::Button)
                        .aria_label(search_label)
                        .tab_index(0)
                        .h(bar_height)
                        .w(px(if search_mode == TaskbarSearchMode::Box { 168.0 } else { 44.0 }))
                        .px(px(12.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.))
                        .cursor_pointer()
                        .hover(move |style| style.bg(rgb(tokens.hover)))
                        .active(move |style| style.bg(rgb(tokens.pressed)))
                        .focus_visible(move |style| {
                            style.border_2().border_color(rgb(tokens.focus))
                        })
                        .on_click(move |_, _, cx| {
                            if let Some(callback) = &search { callback(cx); }
                        })
                        .child("⌕")
                        .when(search_mode == TaskbarSearchMode::Box, |element| element.child(search_label)),
                )
            })
            .when(show_task_view, |element| element.child(
                div()
                    .id("task-view-control")
                    .role(gpui::Role::Button)
                    .aria_label(if zh_tw { "工作檢視" } else { "Task View" })
                    .tab_index(0)
                    .w(px(44.))
                    .h(bar_height)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |style| style.bg(rgb(tokens.hover)))
                    .active(move |style| style.bg(rgb(tokens.pressed)))
                    .focus_visible(move |style| {
                        style.border_2().border_color(rgb(tokens.focus))
                    })
                    .on_click(move |_, _, cx| {
                        if let Some(callback) = &task_view {
                            callback(cx);
                        }
                    })
                    .child("▣"),
            ))
            .child(
                div()
                    .h(bar_height)
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .flex_wrap()
                    .pr(px(notification_area_reserved_width))
                    .when(alignment == TaskbarAlignment::Left, |element| element.content_start())
                    .when(alignment == TaskbarAlignment::Center, |element| element.content_center())
                    .items_start()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("superexplorer-fixed-entry")
                            .role(gpui::Role::Button)
                            .aria_label(self.fixed_name.clone())
                            .tab_index(0)
                            .w(px(adaptive_task_width))
                            .h(px(40.))
                            .flex_none()
                            .px_2()
                            .relative()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(move |_, _, _| {
                                if let Some(callback) = &fixed {
                                    callback();
                                }
                            })
                            .on_key_down(move |event, _, _| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                    && let Some(callback) = &fixed_key
                                {
                                    callback();
                                }
                            })
                            .when_some(
                                self.fixed_icon.as_ref().and_then(icon_render_image),
                                |element, icon| {
                                    element.child(
                                        img(icon)
                                            .w(px(24.))
                                            .h(px(24.))
                                            .mr_2()
                                            .flex_none()
                                            .object_fit(ObjectFit::Contain),
                                    )
                                },
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(self.fixed_name.clone()),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(8.))
                                    .bottom_0()
                                    .w(px(fixed_indicator_width))
                                    .h(px(3.))
                                    .rounded_full()
                                    .bg(rgb(if high_contrast { 0x00ffff } else { 0x5b8db8 })),
                            ),
                    )
                    .children(self.tasks.iter().map(move |task| {
                        let callback = task_callback.clone();
                        let key_callback = callback.clone();
                        let context_callback = task_context_callback.clone();
                        let hover_callback = task_hover_callback.clone();
                        let stable_id = task.stable_id.clone();
                        let key_stable_id = stable_id.clone();
                        let context_stable_id = stable_id.clone();
                        let hover_stable_id = stable_id.clone();
                        let available = task.available;
                        let overlay =
                            overlays
                                .get(&task.stable_id)
                                .cloned()
                                .unwrap_or_else(|| TaskOverlay {
                                    attention: task.attention,
                                    ..TaskOverlay::default()
                                });
                        let visual = TaskVisualState::compose(
                            available,
                            task.active,
                            task.minimized,
                            task.group_size,
                            &overlay,
                            high_contrast,
                            reduced_motion,
                        );
                        let state = if !available {
                            "unavailable".to_owned()
                        } else if task.attention {
                            "attention".to_owned()
                        } else if task.group_size > 1 {
                            format!("group:{}", task.group_size)
                        } else if task.active {
                            "active".to_owned()
                        } else if task.minimized {
                            "minimized".to_owned()
                        } else {
                            "available".to_owned()
                        };
                        let accessible_name = if visual.accessibility_suffix.is_empty() {
                            format!("{} [{state}]", task.name)
                        } else {
                            format!("{} [{state}, {}]", task.name, visual.accessibility_suffix)
                        };
                        let icon = task.icon.as_ref().and_then(icon_render_image);
                        let display_name = task_display_label(
                            &task.name,
                            task.group_size,
                            show_labels,
                            icon.is_some(),
                        );
                        let labeled_button = show_labels || icon.is_none();
                        let task_width = if labeled_button { adaptive_task_width } else { 44.0 };
                        let indicator_width =
                            visual.indicator_width_for(labeled_button, task_width);
                        div()
                            .id(task.stable_id.clone())
                            .role(gpui::Role::Button)
                            .aria_label(accessible_name)
                            .tab_index(0)
                            .w(px(task_width))
                            .h(px(40.))
                            .flex_none()
                            .px_2()
                            .relative()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .when(task.active && !visual.attention_surface, |element| {
                                element.bg(rgb(tokens.active))
                            })
                            .when(visual.attention_surface, |element| {
                                element.bg(rgb(tokens.attention))
                            })
                            .when_some(visual.progress_color, |element, color| {
                                let fraction = visual.progress_fraction.unwrap_or(0.0);
                                let width = if visual.progress_indeterminate && !reduced_motion { 42.0_f32.min(task_width) } else { task_width * fraction };
                                let left = if visual.progress_indeterminate && !reduced_motion { (task_width - width) * visual.progress_offset } else { 0.0 };
                                element.child(
                                    div()
                                        .absolute()
                                        .left(px(left))
                                        .top_0()
                                        .bottom_0()
                                        .w(px(width))
                                        .bg(rgb(color))
                                        .opacity(if high_contrast { 1.0 } else { 0.34 }),
                                )
                            })
                            .when(!available, |element| element.opacity(0.55))
                            .when(available, move |element| {
                                element
                                    .on_click(move |_, _, cx| {
                                        if let Some(callback) = &callback {
                                            callback(&stable_id, cx);
                                        }
                                    })
                                    .on_key_down(move |event, _, cx| {
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                            && let Some(callback) = &key_callback
                                        {
                                            callback(&key_stable_id, cx);
                                        }
                                    })
                                    .on_mouse_down(gpui::MouseButton::Right, move |_, _, cx| {
                                        if let Some(callback) = &context_callback {
                                            callback(&context_stable_id, cx);
                                        }
                                        cx.stop_propagation();
                                    })
                                    .on_hover(move |&hovered, _, cx| {
                                        if let Some(callback) = &hover_callback {
                                            callback(&hover_stable_id, hovered, cx);
                                        }
                                    })
                            })
                            .when_some(icon, |element, icon| {
                                element.child(
                                    img(icon)
                                        .w(px(24.))
                                        .h(px(24.))
                                        .mr_2()
                                        .flex_none()
                                        .object_fit(ObjectFit::Contain),
                                )
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(display_name),
                            )
                            .when_some(overlay.badge.clone(), |element, badge| {
                                element.child(
                                    div()
                                        .ml_auto()
                                        .flex_none()
                                        .px_1()
                                        .rounded_md()
                                        .bg(rgb(0x8b1a1a))
                                        .text_color(rgb(0xffffff))
                                        .child(badge),
                                )
                            })
                            .child(
                                div()
                                    .absolute()
                                    .left(px((task_width - indicator_width) / 2.0))
                                    .bottom_0()
                                    .w(px(indicator_width))
                                    .h(px(visual.indicator_height))
                                    .rounded_full()
                                    .bg(rgb(visual.indicator_color))
                                    .opacity(visual.indicator_opacity),
                            )
                            .when(visual.second_indicator, |element| {
                                element.child(
                                    div()
                                        .absolute()
                                        .left(px((task_width - indicator_width) / 2.0 + 2.0))
                                        .bottom(px(4.))
                                        .w(px((indicator_width - 4.0).max(6.0)))
                                        .h(px(1.))
                                        .rounded_full()
                                        .bg(rgb(visual.indicator_color))
                                        .opacity(visual.indicator_opacity),
                                )
                            })
                    })),
            )
            .child(
                div()
                    .ml_auto()
                    .id("system-status-region")
                    .role(gpui::Role::Group)
                    .aria_label(if zh_tw { "系統狀態" } else { "System status" })
                    .w(px(210.))
                    .h(bar_height)
                    .flex_none()
                    .pl_1()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(
                        div()
                            .id("network-power-control")
                            .role(gpui::Role::Button)
                            .aria_label(match &self.status.core.network {
                                crate::ProviderState::Available(value) => {
                                    format!("Network {value}")
                                }
                                crate::ProviderState::Unavailable(reason) => {
                                    format!("Network unavailable {reason}")
                                }
                            })
                            .tab_index(0)
                            .w(px(36.))
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(0.))
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(_cx.listener(move |this, _, _, cx| {
                                this.system_flyout = toggled_system_flyout(
                                    this.system_flyout,
                                    SystemFlyoutKind::NetworkPower,
                                );
                                if let Some(callback) = &network_flyout_callback {
                                    callback(SystemFlyoutKind::NetworkPower, cx);
                                }
                                cx.notify();
                            }))
                            .on_key_down(_cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if activates_button(&event.keystroke.key) {
                                        this.system_flyout = toggled_system_flyout(
                                            this.system_flyout,
                                            SystemFlyoutKind::NetworkPower,
                                        );
                                        if let Some(callback) = &network_flyout_key_callback {
                                            callback(SystemFlyoutKind::NetworkPower, cx);
                                        }
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(
                                svg()
                                    .external_path(concat!(
                                        env!("CARGO_MANIFEST_DIR"),
                                        "/assets/network-status.svg"
                                    ))
                                    .w(px(18.))
                                    .h(px(18.))
                                    .text_color(rgb(tokens.text)),
                            )
                            .child(match &self.status.core.network {
                                crate::ProviderState::Available(_) => "Network",
                                crate::ProviderState::Unavailable(_) => "Network —",
                            }),
                    )
                    .child(
                        div()
                            .id("volume-control")
                            .role(gpui::Role::Button)
                            .aria_label(match (&self.status.core.volume, &self.status.core.muted) {
                                (crate::ProviderState::Available(volume), crate::ProviderState::Available(true)) => format!("Volume {volume} percent muted"),
                                (crate::ProviderState::Available(volume), _) => format!("Volume {volume} percent"),
                                _ => "Volume unavailable".into(),
                            })
                            .tab_index(0)
                            .w(px(36.))
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(0.))
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(_cx.listener(move |this, _, _, cx| {
                                this.system_flyout = toggled_system_flyout(
                                    this.system_flyout,
                                    SystemFlyoutKind::Volume,
                                );
                                if let Some(callback) = &volume_flyout_callback {
                                    callback(SystemFlyoutKind::Volume, cx);
                                }
                                cx.notify();
                            }))
                            .on_key_down(_cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if activates_button(&event.keystroke.key) {
                                        this.system_flyout = toggled_system_flyout(
                                            this.system_flyout,
                                            SystemFlyoutKind::Volume,
                                        );
                                        if let Some(callback) = &volume_flyout_key_callback {
                                            callback(SystemFlyoutKind::Volume, cx);
                                        }
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(
                                svg()
                                    .external_path(concat!(
                                        env!("CARGO_MANIFEST_DIR"),
                                        "/assets/volume-status.svg"
                                    ))
                                    .w(px(18.))
                                    .h(px(18.))
                                    .text_color(rgb(tokens.text)),
                            )
                            .child(match (&self.status.core.volume, &self.status.core.muted) {
                                (crate::ProviderState::Available(volume), crate::ProviderState::Available(true)) => format!("Muted {volume}%"),
                                (crate::ProviderState::Available(volume), _) => format!("Volume {volume}%"),
                                _ => "Volume —".into(),
                            }),
                    )
                    .child(
                        div()
                            .id("input-language-control")
                            .role(gpui::Role::Button)
                            .aria_label(match &self.status.core.input_language {
                                crate::ProviderState::Available(value) => {
                                    format!("Input language {value}")
                                }
                                crate::ProviderState::Unavailable(reason) => {
                                    format!("Input language unavailable {reason}")
                                }
                            })
                            .tab_index(0)
                            .w(px(44.))
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(_cx.listener(move |this, _, _, cx| {
                                this.system_flyout = toggled_system_flyout(
                                    this.system_flyout,
                                    SystemFlyoutKind::Input,
                                );
                                if let Some(callback) = &input_flyout_callback {
                                    callback(SystemFlyoutKind::Input, cx);
                                }
                                cx.notify();
                            }))
                            .on_key_down(_cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if activates_button(&event.keystroke.key) {
                                        this.system_flyout = toggled_system_flyout(
                                            this.system_flyout,
                                            SystemFlyoutKind::Input,
                                        );
                                        if let Some(callback) = &input_flyout_key_callback {
                                            callback(SystemFlyoutKind::Input, cx);
                                        }
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(match &self.status.core.input_language {
                                crate::ProviderState::Available(value) => compact_input_language(value),
                                crate::ProviderState::Unavailable(_) => "Input —".into(),
                            }),
                    )
                    .child(
                        div()
                            .id("clock-calendar-control")
                            .role(gpui::Role::Button)
                            .aria_label(clock_accessible_label(&self.status, taskbar_rows))
                            .tab_index(0)
                            .w(px(CLOCK_CONTROL_WIDTH))
                            .h(bar_height)
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(tokens.hover)))
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(_cx.listener(move |this, _, _, cx| {
                                this.system_flyout = toggled_system_flyout(
                                    this.system_flyout,
                                    SystemFlyoutKind::Calendar,
                                );
                                if let Some(callback) = &calendar_flyout_callback {
                                    callback(SystemFlyoutKind::Calendar, cx);
                                }
                                cx.notify();
                            }))
                            .on_key_down(_cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if activates_button(&event.keystroke.key) {
                                        this.system_flyout = toggled_system_flyout(
                                            this.system_flyout,
                                            SystemFlyoutKind::Calendar,
                                        );
                                        if let Some(callback) = &calendar_flyout_key_callback {
                                            callback(SystemFlyoutKind::Calendar, cx);
                                        }
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .w_full()
                                    .text_center()
                                    .whitespace_nowrap()
                                    .text_size(px(12.))
                                    .child(self.status.time.clone()),
                            )
                            .when(taskbar_rows >= 3, |clock| {
                                clock.child(
                                    div()
                                        .w_full()
                                        .text_center()
                                        .whitespace_nowrap()
                                        .text_size(px(11.))
                                        .text_color(rgb(tokens.secondary_text))
                                        .child(self.status.weekday.clone()),
                                )
                            })
                            .child(
                                div()
                                    .w_full()
                                    .text_center()
                                    .whitespace_nowrap()
                                    .text_size(px(11.))
                                    .text_color(rgb(tokens.secondary_text))
                                    .child(self.status.date.clone()),
                            ),
                    )
                    .child(
                        div()
                            .id("show-desktop-corner")
                            .role(gpui::Role::Button)
                            .aria_label(if zh_tw { "顯示桌面" } else { "Show desktop" })
                            .tab_index(0)
                            .w(px(SHOW_DESKTOP_CORNER_WIDTH))
                            .h(bar_height)
                            .flex_none()
                            .relative()
                            .cursor_pointer()
                            .border_l_1()
                            .border_color(rgb(tokens.border))
                            .hover(move |style| {
                                style.bg(rgb(tokens.hover)).border_color(rgb(tokens.focus))
                            })
                            .active(move |style| style.bg(rgb(tokens.pressed)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.focus))
                            })
                            .on_click(_cx.listener(move |_, _, _, cx| {
                                if let Some(callback) = &show_desktop {
                                    callback(cx);
                                }
                            }))
                            .on_key_down(_cx.listener(
                                move |_, event: &gpui::KeyDownEvent, _, cx| {
                                    if activates_button(&event.keystroke.key)
                                        && let Some(callback) = &show_desktop_key
                                    {
                                        callback(cx);
                                    }
                                },
                            )),
                    )
                    .when_some(system_flyout, |region, flyout| {
                        region.child(
                            div()
                                .id("system-status-flyout")
                                .role(gpui::Role::Dialog)
                                .aria_label(match flyout {
                                    SystemFlyoutKind::Input => "Input languages",
                                    SystemFlyoutKind::Volume => "Volume",
                                    SystemFlyoutKind::NetworkPower => "Network and power",
                                    SystemFlyoutKind::Calendar => "Calendar",
                                })
                                .absolute()
                                .right_0()
                                .bottom(bar_height)
                                .w(px(360.))
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0xf3f6fb))
                                .shadow_lg()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .children(match flyout {
                                    SystemFlyoutKind::Input => system_snapshot
                                        .as_ref()
                                        .and_then(|snapshot| match &snapshot.input {
                                            StatusAvailability::Available(input) => Some(
                                                input.profiles
                                                    .iter()
                                                    .map(|profile| {
                                                        let callback = input_callback.clone();
                                                        let key_callback = input_callback.clone();
                                                        let id = profile.id.clone();
                                                        let key_id = profile.id.clone();
                                                        div()
                                                            .id(format!("input-profile-{}", profile.id))
                                                            .role(gpui::Role::Button)
                                                            .aria_label(format!(
                                                                "{}{}",
                                                                profile.display_name,
                                                                if profile.id == input.active_profile_id {
                                                                    " active"
                                                                } else {
                                                                    ""
                                                                }
                                                            ))
                                                            .tab_index(0)
                                                            .p_2()
                                                            .rounded_md()
                                                            .when(
                                                                profile.id == input.active_profile_id,
                                                                |element| element.bg(rgb(0xd6e8ff)),
                                                            )
                                                            .on_click(move |_, _, cx| {
                                                                if let Some(callback) = &callback {
                                                                    callback(SystemStatusAction::ActivateInputProfile(id.clone()), cx);
                                                                }
                                                            })
                                                            .on_key_down(move |event, _, cx| {
                                                                if activates_button(
                                                                    &event.keystroke.key,
                                                                ) && let Some(callback) =
                                                                    &key_callback
                                                                {
                                                                    callback(
                                                                        SystemStatusAction::ActivateInputProfile(
                                                                            key_id.clone(),
                                                                        ),
                                                                        cx,
                                                                    );
                                                                }
                                                            })
                                                            .child(profile.display_name.clone())
                                                    })
                                                    .collect::<Vec<_>>(),
                                            ),
                                            _ => None,
                                        })
                                        .unwrap_or_else(|| {
                                            vec![div()
                                                .id("input-profiles-unavailable")
                                                .child("Input profiles unavailable")]
                                        }),
                                    SystemFlyoutKind::Volume => {
                                        match (&self.status.core.volume, &self.status.core.muted) {
                                            (crate::ProviderState::Available(current), crate::ProviderState::Available(muted)) => {
                                                let current = *current;
                                                let muted = *muted;
                                                let lower_callback = volume_callback.clone();
                                                let higher_callback = volume_callback.clone();
                                                let mute_callback = mute_callback.clone();
                                                vec![
                                                    div().id("volume-value").child(format!("Volume {current}%")),
                                                    div().id("volume-actions").flex().gap_2()
                                                        .child(div().id("volume-lower").role(gpui::Role::Button).aria_label("Lower volume").tab_index(0).p_2().on_click(move |_,_,cx| { if let Some(callback)=&lower_callback { callback(SystemStatusAction::SetVolume(current.saturating_sub(10)), cx); } }).child("-"))
                                                        .child(div().id("volume-mute").role(gpui::Role::Button).aria_label(if muted {"Unmute"} else {"Mute"}).tab_index(0).p_2().on_click(move |_,_,cx| { if let Some(callback)=&mute_callback { callback(SystemStatusAction::SetMute(!muted), cx); } }).child(if muted {"Unmute"} else {"Mute"}))
                                                        .child(div().id("volume-higher").role(gpui::Role::Button).aria_label("Raise volume").tab_index(0).p_2().on_click(move |_,_,cx| { if let Some(callback)=&higher_callback { callback(SystemStatusAction::SetVolume(current.saturating_add(10).min(100)), cx); } }).child("+")),
                                                ]
                                            }
                                            _ => vec![div().id("volume-unavailable").child("Volume unavailable")],
                                        }
                                    }
                                    SystemFlyoutKind::NetworkPower => vec![
                                        div().id("network-value").child(match &self.status.core.network { crate::ProviderState::Available(value)=>format!("Network: {value}"), crate::ProviderState::Unavailable(_)=>"Network unavailable".into() }),
                                        div().id("power-value").child(match &self.status.core.battery { crate::ProviderState::Available(value)=>format!("Battery: {value}%"), crate::ProviderState::Unavailable(_)=>"Battery not present or unavailable".into() }),
                                    ],
                                    SystemFlyoutKind::Calendar => vec![
                                        div().id("calendar-value").child(format!("{} {}", self.status.date, self.status.time)),
                                        div().id("calendar-zone").child(system_snapshot.as_ref().and_then(|snapshot| match &snapshot.clock { StatusAvailability::Available(clock)=>Some(format!("{} · {}",clock.locale,clock.time_zone)), _=>None }).unwrap_or_else(|| "Calendar provider unavailable".into())),
                                    ],
                                }),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CLOCK_CONTROL_WIDTH, SHOW_DESKTOP_CORNER_WIDTH, TaskbarChromeTokens, activates_button,
        adaptive_labeled_task_width, bc7_render_image, clock_accessible_label,
        compact_input_language, fixed_entry_indicator_width, icon_render_image, task_display_label,
        taskbar_rows_for_logical_height, taskbar_search_label, toggled_system_flyout,
    };
    use crate::{
        ClockLocale, CoreStatus, ProviderState, StatusRegion, SystemFlyoutKind, TestClock,
    };
    use shell_provider_protocol::IconData;

    #[test]
    fn no_icon_always_uses_readable_english_and_traditional_chinese_labels() {
        assert_eq!(task_display_label("Discord", 1, true, false), "Discord");
        assert_eq!(
            task_display_label("工作管理員", 1, false, false),
            "工作管理員"
        );
        assert_eq!(task_display_label("瀏覽器", 3, false, false), "瀏覽器 (3)");
        assert_eq!(task_display_label("", 1, false, false), "Untitled");
    }

    #[test]
    fn labels_can_only_be_hidden_when_a_real_icon_exists() {
        assert_eq!(task_display_label("Discord", 1, false, true), "");
        assert_eq!(task_display_label("Discord", 1, true, true), "Discord");
    }

    #[test]
    fn system_flyouts_are_exclusive_and_rapid_switches_are_deterministic() {
        assert!(activates_button("enter"));
        assert!(activates_button("space"));
        assert!(!activates_button("tab"));
        let current = toggled_system_flyout(None, SystemFlyoutKind::Input);
        assert_eq!(current, Some(SystemFlyoutKind::Input));
        let current = toggled_system_flyout(current, SystemFlyoutKind::Volume);
        assert_eq!(current, Some(SystemFlyoutKind::Volume));
        assert_eq!(
            toggled_system_flyout(current, SystemFlyoutKind::Volume),
            None
        );
    }

    #[test]
    fn clock_width_rows_alignment_and_accessible_order_match_windows() {
        let status = StatusRegion::new(
            TestClock {
                year: 2026,
                month: 8,
                day: 19,
                hour: 15,
                minute: 30,
                second: 23,
            },
            ClockLocale::ZhTw,
            CoreStatus {
                network: ProviderState::Unavailable("fixture"),
                volume: ProviderState::Unavailable("fixture"),
                muted: ProviderState::Unavailable("fixture"),
                input_language: ProviderState::Unavailable("fixture"),
                battery: ProviderState::Unavailable("fixture"),
                notifications: ProviderState::Unavailable("fixture"),
            },
        );
        assert_eq!(CLOCK_CONTROL_WIDTH, 112.0);
        assert_eq!(
            clock_accessible_label(&status, 3),
            "下午 03:30:23 星期三 2026/8/19"
        );
        assert_eq!(
            clock_accessible_label(&status, 1),
            "下午 03:30:23 2026/8/19"
        );
        assert_eq!(
            clock_accessible_label(&status, 2),
            clock_accessible_label(&status, 1)
        );
        let source = include_str!("view.rs");
        for required in [
            "CLOCK_CONTROL_WIDTH: f32 = 112.0",
            "taskbar_rows >= 3",
            ".w_full()",
            ".text_center()",
            ".whitespace_nowrap()",
            "clock_accessible_label(&self.status, taskbar_rows)",
            "CLOCK_CONTROL_WIDTH - LEGACY_CLOCK_CONTROL_WIDTH",
        ] {
            assert!(
                source.contains(required),
                "missing clock contract: {required}"
            );
        }
    }

    #[test]
    fn system_status_source_contract_keeps_accessible_owned_controls_and_safe_unavailable_ui() {
        let source = include_str!("view.rs");
        for required in [
            "network-power-control",
            "volume-control",
            "input-language-control",
            "clock-calendar-control",
            "system-status-flyout",
            "SystemStatusAction::ActivateInputProfile",
            "SystemStatusAction::SetVolume",
            "SystemStatusAction::SetMute",
            "volume-unavailable",
            "event.keystroke.key == \"escape\"",
        ] {
            assert!(
                source.contains(required),
                "missing system UI contract: {required}"
            );
        }
    }

    #[test]
    fn show_desktop_corner_is_exact_owned_accessible_and_full_height() {
        assert_eq!(SHOW_DESKTOP_CORNER_WIDTH, 8.0);
        assert!(activates_button("enter"));
        assert!(activates_button("space"));
        assert!(!activates_button("escape"));
        let source = include_str!("view.rs");
        for required in [
            "show-desktop-corner",
            "顯示桌面",
            "Show desktop",
            ".role(gpui::Role::Button)",
            ".w(px(SHOW_DESKTOP_CORNER_WIDTH))",
            ".h(bar_height)",
            ".border_l_1()",
            "callback(cx)",
        ] {
            assert!(
                source.contains(required),
                "missing show desktop UI contract: {required}"
            );
        }
        let corner = source
            .split("show-desktop-corner")
            .nth(1)
            .and_then(|tail| tail.split("system-status-flyout").next())
            .expect("show desktop render source");
        assert!(!corner.contains("border_b_1"));
        assert!(!corner.contains("Shell_TrayWnd"));
    }

    #[test]
    fn task_visual_source_contract_uses_windows_indicators_and_background_progress() {
        let source = include_str!("view.rs");
        for required in [
            "TaskVisualState::compose",
            "(task_width - indicator_width) / 2.0",
            "visual.second_indicator",
            "visual.progress_fraction",
            "visual.progress_indeterminate",
            "visual.attention_surface",
            "visual.accessibility_suffix",
        ] {
            assert!(
                source.contains(required),
                "missing visual-state contract: {required}"
            );
        }
        let task_render = source
            .split("TaskVisualState::compose")
            .nth(1)
            .and_then(|tail| tail.split("system-status-region").next())
            .expect("task visual render source");
        assert!(!task_render.contains(".border_b_1()"));
    }

    #[test]
    fn taskbar_chrome_tokens_and_compact_locale_labels_are_deterministic() {
        let light = TaskbarChromeTokens::new(Some("light"));
        let dark = TaskbarChromeTokens::new(Some("dark"));
        let contrast = TaskbarChromeTokens::new(Some("high-contrast"));
        assert_ne!(light.panel, light.hover);
        assert_ne!(light.hover, light.pressed);
        assert_eq!(dark.panel, 0x202020);
        assert_eq!(dark.text, 0xffffff);
        assert_ne!(dark.hover, dark.pressed);
        assert_eq!(contrast.panel, 0x000000);
        assert_eq!(contrast.border, 0xffffff);
        assert_eq!(contrast.focus, 0xffff00);
        assert_eq!(taskbar_search_label(Some("zh-TW")), "搜尋");
        assert_eq!(taskbar_search_label(Some("en-US")), "Search");
        assert_eq!(compact_input_language("zh-TW"), "中");
        assert_eq!(compact_input_language("zh_CN"), "中");
        assert_eq!(compact_input_language("en-US"), "ENG");
        assert_eq!(compact_input_language("ja-JP"), "JA");
        assert_eq!(compact_input_language(""), "?");
    }

    #[test]
    fn taskbar_chrome_source_uses_shared_interaction_and_system_geometry() {
        let source = include_str!("view.rs");
        for required in [
            "TaskbarChromeTokens::new",
            ".hover(move |style|",
            ".active(move |style|",
            ".focus_visible(move |style|",
            ".w(px(210.))",
            "notification_area_reserved_width",
            ".pr(px(notification_area_reserved_width))",
            "compact_input_language(value)",
            ".text_size(px(12.))",
            ".text_size(px(11.))",
        ] {
            assert!(
                source.contains(required),
                "missing taskbar chrome: {required}"
            );
        }
        let fixed_start = source.find(".id(\"superexplorer-fixed-entry\")").unwrap();
        let fixed_end = source[fixed_start..]
            .find(".children(self.tasks.iter()")
            .unwrap()
            + fixed_start;
        let fixed_source = &source[fixed_start..fixed_end];
        assert!(fixed_source.contains(".w(px(adaptive_task_width))"));
        assert!(fixed_source.contains(".w(px(fixed_indicator_width))"));
        assert!(!fixed_source.contains(".w(px(160.))"));
    }

    #[test]
    fn tray_show_all_control_is_unconditional_complete_and_theme_aware() {
        let source = include_str!("view.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        let start = production
            .find(".id(\"notification-overflow-control\")")
            .expect("show-all control");
        let end = production[start..]
            .find(".when(overflow_open")
            .map(|offset| start + offset)
            .expect("inline overflow boundary");
        let control = &production[start..end];
        for required in [
            "visible_notifications.len() as f32 * 36.0 + 32.0",
            "Show all tray icons",
            "顯示所有系統匣圖示",
            "callback(this.notification_area.accessible_nodes(), cx)",
            ".text_color(rgb(tokens.text))",
            "activates_button(&event.keystroke.key)",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        assert!(!production.contains("when(has_overflow"));
        assert!(!control.contains("NotificationPlacement::Overflow"));
        assert!(production.contains(".on_mouse_up(gpui::MouseButton::Right"));
        assert_eq!(
            control
                .matches("callback(this.notification_area.accessible_nodes(), cx)")
                .count(),
            2,
            "pointer and keyboard routes must both forward complete fresh snapshots"
        );
    }

    #[test]
    fn taskbar_resize_quantization_lock_strip_and_continuous_rows_are_deterministic() {
        assert_eq!(taskbar_rows_for_logical_height(1.0), 1);
        assert_eq!(taskbar_rows_for_logical_height(59.9), 1);
        assert_eq!(taskbar_rows_for_logical_height(60.0), 2);
        assert_eq!(taskbar_rows_for_logical_height(99.9), 2);
        assert_eq!(taskbar_rows_for_logical_height(100.0), 3);
        assert_eq!(taskbar_rows_for_logical_height(10_000.0), 3);
        let source = include_str!("view.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "taskbar-resize-strip",
            "CursorStyle::ResizeUpDown",
            "ResizeEdge::Top",
            ".h(px(12.))",
            "attach_resize_observer",
            "if this.locked",
            ".border_t_1()",
        ] {
            assert!(
                source.contains(required),
                "missing resize contract: {required}"
            );
        }
        assert!(!production.contains("(1..row_count)"));
        assert!(!production.contains("40.0 * f32::from(row)"));
    }

    #[test]
    fn task_icon_pixels_are_validated_as_rgba() {
        let image = icon_render_image(&IconData {
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
        })
        .unwrap();
        assert_eq!(image.as_bytes(0), Some([0, 0, 255, 255].as_slice()));
        assert!(
            icon_render_image(&IconData {
                width: 1,
                height: 1,
                rgba: vec![0; 3],
            })
            .is_none()
        );
    }

    #[test]
    fn bc7_task_icon_payload_is_directly_renderable() {
        let icon = IconData {
            width: 4,
            height: 4,
            rgba: vec![255; 4 * 4 * 4],
        };
        let image = bc7_render_image(&icon).unwrap();
        assert!(image.compressed_raster().is_some());
        assert_eq!(image.as_bytes(0).unwrap().len(), 16);
    }

    #[test]
    fn task_label_source_contract_keeps_truthful_fallback_and_ellipsis_container() {
        let source = include_str!("view.rs");
        let forbidden = ["task.name", ".chars()", ".next()"].concat();
        assert!(!source.contains(&forbidden));
        for required in [
            "task.icon.as_ref().and_then(icon_render_image)",
            ".w(px(24.))",
            ".flex_1()",
            ".min_w_0()",
            ".overflow_hidden()",
            ".whitespace_nowrap()",
            ".text_ellipsis()",
        ] {
            assert!(
                source.contains(required),
                "missing label contract: {required}"
            );
        }
        assert!(source.contains(
            ".h(bar_height)\n                    .flex_1()\n                    .min_w_0()\n                    .flex()\n                    .flex_col()\n                    .flex_wrap()"
        ));
    }

    #[test]
    fn adaptive_task_width_shrinks_before_overflow_and_respects_rows() {
        assert_eq!(
            adaptive_labeled_task_width(2400.0, 204.0, 300.0, 4, 1),
            160.0
        );
        let crowded = adaptive_labeled_task_width(1920.0, 204.0, 510.0, 10, 1);
        assert!((crowded - 120.6).abs() < 0.01);
        assert_eq!(
            adaptive_labeled_task_width(800.0, 204.0, 510.0, 10, 1),
            44.0
        );
        assert_eq!(
            adaptive_labeled_task_width(1920.0, 204.0, 510.0, 10, 2),
            160.0
        );
        let without_fixed = adaptive_labeled_task_width(1920.0, 44.0, 510.0, 12, 1);
        let with_fixed = adaptive_labeled_task_width(1920.0, 44.0, 510.0, 13, 1);
        assert!(with_fixed < without_fixed);
        assert_eq!(fixed_entry_indicator_width(160.0), 144.0);
        assert_eq!(fixed_entry_indicator_width(44.0), 28.0);
        let source = include_str!("view.rs");
        for required in [
            "adaptive_labeled_task_width(",
            "left_reserved",
            "right_reserved",
            "adaptive_task_slots",
            "fixed_entry_indicator_width(adaptive_task_width)",
            "let task_width = if labeled_button { adaptive_task_width } else { 44.0 }",
        ] {
            assert!(source.contains(required), "missing {required}");
        }
    }
}
