use std::{collections::BTreeMap, rc::Rc, sync::Arc};

use gpui::{
    App, AppContext, Context, FocusHandle, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, Render, RenderImage, StatefulInteractiveElement, Styled, StyledImage, Window,
    div, img, linear_color_stop, linear_gradient, prelude::FluentBuilder as _, px, rgb, svg,
};

use crate::{
    AccessibleTask, NotificationAreaModel, NotificationPlacement, ProgressState, StatusRegion,
    TaskOverlay, TaskbarLayout,
};
use shell_provider_protocol::{IconData, IconKey, NotificationEventKind};

fn notification_render_image(icon: &IconData) -> Option<Arc<RenderImage>> {
    let mut bgra = icon.rgba.clone();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let buffer = image::RgbaImage::from_raw(icon.width, icon.height, bgra)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
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

pub struct TaskbarView {
    pub accessible_root_name: String,
    pub layout: TaskbarLayout,
    pub tasks: Vec<AccessibleTask>,
    pub fixed_name: String,
    pub status: StatusRegion,
    pub notification_area: NotificationAreaModel,
    pub overlays: BTreeMap<String, TaskOverlay>,
    pub show_labels: bool,
    pub callbacks: Option<TaskbarCallbacks>,
    pub keyboard_focus: Option<FocusHandle>,
}

pub type TaskCallback = Rc<dyn Fn(&str, &mut App)>;
pub type NotificationCallback = Rc<dyn Fn(&IconKey, NotificationEventKind)>;

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
        let start_key = start.clone();
        let task_view_key = task_view.clone();
        let fixed_key = fixed.clone();
        let root_fixed_key = fixed.clone();
        let keyboard_focus = self.keyboard_focus.clone();
        let overflow_open = self.notification_area.overflow_open();
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
            .on_key_down(move |event, _, cx| {
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
            })
            .size_full()
            .flex()
            .flex_row()
            .items_stretch()
            .bg(linear_gradient(
                90.,
                linear_color_stop(rgb(0xd8edc0), 0.),
                linear_color_stop(rgb(0xc3efef), 1.),
            ))
            .border_t_1()
            .border_color(rgb(0xe4f4d0))
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
                    .flex_none()
                    .relative()
                    .flex()
                    .items_center()
                    .children(visible_notifications.into_iter().map(|node| {
                        let callback = notification_callback.clone();
                        let context_callback = notification_callback.clone();
                        let key = node.key.clone();
                        let context_key = node.key.clone();
                        let tooltip = node.name.clone();
                        let icon = node.icon.as_ref().and_then(notification_render_image);
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
                                .on_click(_cx.listener(|this, _, _, cx| {
                                    if this.notification_area.overflow_open() {
                                        this.notification_area.dismiss_overflow();
                                    } else {
                                        this.notification_area.open_overflow();
                                    }
                                    cx.notify();
                                }))
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
                                    let icon =
                                        node.icon.as_ref().and_then(notification_render_image);
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
                    .when(!high_contrast, |element| {
                        element
                            .child(div().absolute().top_0().left_0().w_full().h(px(40.)).bg(
                                linear_gradient(
                                    90.,
                                    linear_color_stop(rgb(0xd9efbd), 0.),
                                    linear_color_stop(rgb(0xd7edc1), 1.),
                                ),
                            ))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(40.))
                                    .left_0()
                                    .w_full()
                                    .h(px(40.))
                                    .bg(linear_gradient(
                                        90.,
                                        linear_color_stop(rgb(0xd8ecb6), 0.),
                                        linear_color_stop(rgb(0xd8edb8), 1.),
                                    )),
                            )
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
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .border_b_1()
                            .border_color(rgb(0x0078d4))
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
                            .child(self.fixed_name.clone()),
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
                        let accessible_name = format!("{} [{state}]", task.name);
                        // AccessibleTask does not yet carry a renderable icon. A disabled label
                        // preference therefore falls back to truthful text instead of inventing a
                        // pseudo-icon from the first character of the window title.
                        let display_name =
                            task_display_label(&task.name, task.group_size, show_labels, false);
                        let underline = if !available {
                            rgb(0x6b6b6b)
                        } else if task.attention {
                            rgb(0xff8c00)
                        } else if task.group_size > 1 {
                            rgb(0x744da9)
                        } else if task.active {
                            rgb(0x0067c0)
                        } else if task.minimized {
                            rgb(0x87949a)
                        } else {
                            rgb(0x1683d8)
                        };
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
                            .border_b_1()
                            .border_color(underline)
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
                            .when(overlay.progress != ProgressState::None, |element| {
                                let (fraction, color) = match overlay.progress {
                                    ProgressState::None => (0.0, rgb(0x1683d8)),
                                    ProgressState::Indeterminate => (1.0, rgb(0x1683d8)),
                                    ProgressState::Normal(value) => {
                                        (f32::from(value.min(1000)) / 1000.0, rgb(0x1683d8))
                                    }
                                    ProgressState::Paused(value) => {
                                        (f32::from(value.min(1000)) / 1000.0, rgb(0xffb900))
                                    }
                                    ProgressState::Error(value) => {
                                        (f32::from(value.min(1000)) / 1000.0, rgb(0xd13438))
                                    }
                                };
                                element.child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .bottom_0()
                                        .h(px(3.))
                                        .w(px(190.0 * fraction))
                                        .bg(color),
                                )
                            })
                    })),
            )
            .child(
                div()
                    .ml_auto()
                    .w(px(300.))
                    .h(bar_height)
                    .flex_none()
                    .px_2()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .child(self.status.time.clone())
                    .child(self.status.date.clone()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::task_display_label;

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
    fn task_label_source_contract_keeps_truthful_fallback_and_ellipsis_container() {
        let source = include_str!("view.rs");
        let forbidden = ["task.name", ".chars()", ".next()"].concat();
        assert!(!source.contains(&forbidden));
        for required in [
            "task_display_label(&task.name, task.group_size, show_labels, false)",
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
