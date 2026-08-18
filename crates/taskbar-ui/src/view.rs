use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

use gpui::{
    App, AppContext, Context, FocusHandle, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, Render, RenderImage, StatefulInteractiveElement, Styled, StyledImage, Window,
    div, img, prelude::FluentBuilder as _, px, rgb, svg,
};

use crate::{
    AccessibleTask, NotificationAccessibleNode, NotificationAreaModel, NotificationPlacement,
    StatusRegion, SystemFlyoutKind, SystemStatusAction, TaskOverlay, TaskVisualState,
    TaskbarLayout,
};
use shell_provider_protocol::{
    IconData, IconKey, NotificationEventKind, StatusAvailability, SystemStatusSnapshot,
};

thread_local! {
    static ICON_RENDER_CACHE: RefCell<VecDeque<(u64, IconData, Arc<RenderImage>)>> = const { RefCell::new(VecDeque::new()) };
}

const ICON_RENDER_CACHE_LIMIT: usize = 2_048;

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

fn toggled_system_flyout(
    current: Option<SystemFlyoutKind>,
    requested: SystemFlyoutKind,
) -> Option<SystemFlyoutKind> {
    (current != Some(requested)).then_some(requested)
}

fn activates_button(key: &str) -> bool {
    matches!(key, "enter" | "space")
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
    pub callbacks: Option<TaskbarCallbacks>,
    pub keyboard_focus: Option<FocusHandle>,
}

pub type TaskCallback = Rc<dyn Fn(&str, &mut App)>;
pub type NotificationCallback = Rc<dyn Fn(&IconKey, NotificationEventKind)>;
pub type NotificationOverflowCallback = Rc<dyn Fn(Vec<NotificationAccessibleNode>, &mut App)>;
pub type SystemStatusCallback = Rc<dyn Fn(SystemStatusAction, &mut App)>;
pub type SystemFlyoutCallback = Rc<dyn Fn(SystemFlyoutKind, &mut App)>;

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
    pub task_view: Rc<dyn Fn(&mut App)>,
    pub fixed: Rc<dyn Fn()>,
    pub task: TaskCallback,
    pub task_context: TaskCallback,
    pub notification: NotificationCallback,
    pub notification_overflow: NotificationOverflowCallback,
    pub system_status: SystemStatusCallback,
    pub system_flyout: SystemFlyoutCallback,
    pub rendered: Rc<dyn Fn()>,
}

impl Render for TaskbarView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bar_height = px(self.layout.height / window.scale_factor());
        let start = self.callbacks.as_ref().map(|value| Rc::clone(&value.start));
        let task_view = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.task_view));
        let fixed = self.callbacks.as_ref().map(|value| Rc::clone(&value.fixed));
        let task_callback = self.callbacks.as_ref().map(|value| Rc::clone(&value.task));
        let task_context_callback = self
            .callbacks
            .as_ref()
            .map(|value| Rc::clone(&value.task_context));
        let overlays = self.overlays.clone();
        let show_labels = self.show_labels;
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
        let has_overflow = !overflow_notifications.is_empty();
        let high_contrast = std::env::var("SUPERDESKTOP_THEME").as_deref() == Ok("high-contrast");
        let reduced_motion = std::env::var("SUPERDESKTOP_REDUCED_MOTION").as_deref() == Ok("1");
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
            .bg(rgb(0xf3f3f3))
            .border_t_1()
            .border_color(rgb(0xd8d8d8))
            .text_color(if high_contrast {
                rgb(0xffffff)
            } else {
                rgb(0x182220)
            })
            .when(high_contrast, |element| element.bg(rgb(0x000000)))
            .text_size(px(14.))
            .child(
                div()
                    .id("notification-area")
                    .role(gpui::Role::Group)
                    .aria_label("Notification area")
                    .h(bar_height)
                    .absolute()
                    .right(px(230.))
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
                            .on_click(move |_, _, _| {
                                if let Some(callback) = &callback {
                                    callback(&key, NotificationEventKind::Activate);
                                }
                            })
                            .on_mouse_down(gpui::MouseButton::Right, move |_, _, _| {
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
                    .when(has_overflow, |area| {
                        area.child(
                            div()
                                .id("notification-overflow-control")
                                .role(gpui::Role::Button)
                                .aria_label("Show hidden icons")
                                .tab_index(0)
                                .w(px(32.))
                                .h(px(36.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(0.))
                                .on_click(_cx.listener(move |this, _, _, cx| {
                                    if this.notification_area.overflow_open() {
                                        this.notification_area.dismiss_overflow();
                                    } else {
                                        this.notification_area.open_overflow();
                                    }
                                    if let Some(callback) = &notification_overflow_callback {
                                        callback(
                                            this.notification_area
                                                .accessible_nodes()
                                                .into_iter()
                                                .filter(|node| node.placement == NotificationPlacement::Overflow)
                                                .collect(),
                                            cx,
                                        );
                                    }
                                    cx.notify();
                                }))
                                .on_key_down(_cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    if activates_button(&event.keystroke.key)
                                        && let Some(callback) = &notification_overflow_key_callback
                                    {
                                        this.notification_area.open_overflow();
                                        callback(
                                            this.notification_area
                                                .accessible_nodes()
                                                .into_iter()
                                                .filter(|node| node.placement == NotificationPlacement::Overflow)
                                                .collect(),
                                            cx,
                                        );
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
                                        .text_color(rgb(0x202020)),
                                )
                                .child("⌃"),
                        )
                    })
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
                    .aria_label("Start")
                    .tab_index(0)
                    .w(px(48.))
                    .h(bar_height)
                    .flex_none()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(high_contrast, |element| {
                        element.border_2().border_color(rgb(0xffff00))
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
                            .w(px(14.))
                            .h(px(14.))
                            .text_color(start_color),
                    ),
            )
            .child(
                div()
                    .id("task-view-control")
                    .role(gpui::Role::Button)
                    .aria_label("Task View")
                    .tab_index(0)
                    .w(px(44.))
                    .h(bar_height)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(move |_, _, cx| {
                        if let Some(callback) = &task_view {
                            callback(cx);
                        }
                    })
                    .child("▣"),
            )
            .child(
                div()
                    .h(bar_height)
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .flex_wrap()
                    .content_start()
                    .items_start()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("superexplorer-fixed-entry")
                            .role(gpui::Role::Button)
                            .aria_label(self.fixed_name.clone())
                            .tab_index(0)
                            .w(px(150.))
                            .h(px(40.))
                            .flex_none()
                            .px_2()
                            .relative()
                            .flex()
                            .items_center()
                            .cursor_pointer()
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
                            .child(self.fixed_name.clone())
                            .child(
                                div()
                                    .absolute()
                                    .left(px(72.))
                                    .bottom_0()
                                    .w(px(6.))
                                    .h(px(3.))
                                    .rounded_full()
                                    .bg(rgb(if high_contrast { 0x00ffff } else { 0x5b8db8 })),
                            ),
                    )
                    .children(self.tasks.iter().map(move |task| {
                        let callback = task_callback.clone();
                        let key_callback = callback.clone();
                        let context_callback = task_context_callback.clone();
                        let stable_id = task.stable_id.clone();
                        let key_stable_id = stable_id.clone();
                        let context_stable_id = stable_id.clone();
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
                        div()
                            .id(task.stable_id.clone())
                            .role(gpui::Role::Button)
                            .aria_label(accessible_name)
                            .tab_index(0)
                            .w(px(190.))
                            .h(px(40.))
                            .flex_none()
                            .px_2()
                            .relative()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .when(task.active && !visual.attention_surface, |element| {
                                element.bg(if high_contrast { rgb(0x000000) } else { rgb(0xe5e5e5) })
                            })
                            .when(visual.attention_surface, |element| {
                                element.bg(if high_contrast { rgb(0x000000) } else { rgb(0xffd28a) })
                            })
                            .when_some(visual.progress_color, |element, color| {
                                let fraction = visual.progress_fraction.unwrap_or(0.0);
                                let width = if visual.progress_indeterminate && !reduced_motion { 42.0 } else { 190.0 * fraction };
                                let left = if visual.progress_indeterminate && !reduced_motion { (190.0 - width) * visual.progress_offset } else { 0.0 };
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
                                    .left(px((190.0 - visual.indicator_width) / 2.0))
                                    .bottom_0()
                                    .w(px(visual.indicator_width))
                                    .h(px(visual.indicator_height))
                                    .rounded_full()
                                    .bg(rgb(visual.indicator_color))
                                    .opacity(visual.indicator_opacity),
                            )
                            .when(visual.second_indicator, |element| {
                                element.child(
                                    div()
                                        .absolute()
                                        .left(px((190.0 - visual.indicator_width) / 2.0 + 2.0))
                                        .bottom(px(4.))
                                        .w(px((visual.indicator_width - 4.0).max(6.0)))
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
                    .aria_label("System status")
                    .w(px(230.))
                    .h(bar_height)
                    .flex_none()
                    .px_2()
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
                                    .text_color(rgb(0x202020)),
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
                                    .text_color(rgb(0x202020)),
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
                            .w(px(54.))
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
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
                                crate::ProviderState::Available(value) => value.clone(),
                                crate::ProviderState::Unavailable(_) => "Input —".into(),
                            }),
                    )
                    .child(
                        div()
                            .id("clock-calendar-control")
                            .role(gpui::Role::Button)
                            .aria_label(format!("{} {}", self.status.time, self.status.date))
                            .tab_index(0)
                            .w(px(88.))
                            .h(bar_height)
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
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
                            .child(self.status.time.clone())
                            .child(self.status.date.clone()),
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
        activates_button, bc7_render_image, icon_render_image, task_display_label,
        toggled_system_flyout,
    };
    use crate::SystemFlyoutKind;
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
    fn task_visual_source_contract_uses_windows_indicators_and_background_progress() {
        let source = include_str!("view.rs");
        for required in [
            "TaskVisualState::compose",
            "(190.0 - visual.indicator_width) / 2.0",
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
}
