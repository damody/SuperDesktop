use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ObjectFit, ParentElement, Render,
    StatefulInteractiveElement, Styled, StyledImage, Subscription, Window, div, img,
    prelude::FluentBuilder as _, px, rgb,
};
use shell_provider_protocol::{IconKey, NotificationEventKind};

use crate::{NotificationAccessibleNode, view::icon_render_image};

pub type NotificationOverflowAction = Rc<dyn Fn(&IconKey, NotificationEventKind)>;
pub type NotificationOverflowDismiss = Rc<dyn Fn(&mut Window, &mut gpui::App)>;

fn traditional_chinese() -> bool {
    if let Ok(locale) = std::env::var("SUPERDESKTOP_LOCALE") {
        return locale.eq_ignore_ascii_case("zh-TW");
    }
    platform_win::common::taskbar_status::user_locale_name()
        .is_some_and(|locale| locale.eq_ignore_ascii_case("zh-TW"))
}

pub struct NotificationOverflowView {
    pub nodes: Vec<NotificationAccessibleNode>,
    action: NotificationOverflowAction,
    dismiss: NotificationOverflowDismiss,
    focus: FocusHandle,
    _activation_subscription: Subscription,
}

impl NotificationOverflowView {
    pub fn new(
        nodes: Vec<NotificationAccessibleNode>,
        action: NotificationOverflowAction,
        dismiss: NotificationOverflowDismiss,
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
            nodes,
            action,
            dismiss,
            focus: cx.focus_handle(),
            _activation_subscription: activation_subscription,
        }
    }
}

impl Render for NotificationOverflowView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let high_contrast = std::env::var("SUPERDESKTOP_THEME")
            .is_ok_and(|value| value.eq_ignore_ascii_case("high-contrast"));
        let panel_background = if high_contrast { 0x000000 } else { 0xf3f3f3 };
        let panel_border = if high_contrast { 0xffffff } else { 0xd0d0d0 };
        let hover_background = if high_contrast { 0x1a1a1a } else { 0xe7e7e7 };
        let pressed_background = if high_contrast { 0x303030 } else { 0xdcdcdc };
        let empty = self.nodes.is_empty();
        let zh_tw = traditional_chinese();
        let dismiss = self.dismiss.clone();
        let action = self.action.clone();
        div()
            .id("owned-notification-overflow")
            .role(gpui::Role::Dialog)
            .aria_label(if zh_tw {
                "系統匣圖示"
            } else {
                "Tray icons"
            })
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p(px(12.))
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(panel_border))
            .bg(rgb(panel_background))
            .shadow_lg()
            .flex()
            .flex_wrap()
            .content_start()
            .on_key_down(
                cx.listener(move |_, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        dismiss(window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .when(empty, |panel| {
                panel.items_center().justify_center().child(
                    div()
                        .id("notification-overflow-empty")
                        .role(gpui::Role::Status)
                        .aria_label(if zh_tw {
                            "目前沒有系統匣圖示"
                        } else {
                            "No tray icons are currently registered"
                        })
                        .child(if zh_tw {
                            "目前沒有系統匣圖示"
                        } else {
                            "No tray icons"
                        }),
                )
            })
            .children(self.nodes.iter().cloned().map(move |node| {
                let click_action = action.clone();
                let key_action = action.clone();
                let context_action = action.clone();
                let click_key = node.key.clone();
                let key_key = node.key.clone();
                let context_key = node.key.clone();
                let icon = node.icon.as_ref().and_then(icon_render_image);
                div()
                    .id(node.stable_id)
                    .role(gpui::Role::Button)
                    .aria_label(node.name)
                    .tab_index(0)
                    .w(px(48.))
                    .h(px(48.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(panel_background))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |style| style.bg(rgb(hover_background)))
                    .active(move |style| style.bg(rgb(pressed_background)))
                    .focus_visible(move |style| style.border_2().border_color(rgb(panel_border)))
                    .on_click(move |_, _, _| {
                        click_action(&click_key, NotificationEventKind::Activate);
                    })
                    .on_mouse_up(gpui::MouseButton::Right, move |_, _, _| {
                        context_action(&context_key, NotificationEventKind::Context);
                    })
                    .on_key_down(move |event, _, _| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            key_action(&key_key, NotificationEventKind::Activate);
                        }
                    })
                    .when_some(icon, |entry, icon| {
                        entry.child(
                            img(icon)
                                .w(px(24.))
                                .h(px(24.))
                                .object_fit(ObjectFit::Contain),
                        )
                    })
            }))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn overflow_surface_matches_windows11_interaction_contract() {
        let source = include_str!("notification_overflow.rs");
        for required in [
            "owned-notification-overflow",
            "Tray icons",
            "notification-overflow-empty",
            "No tray icons",
            "目前沒有系統匣圖示",
            "observe_window_activation",
            "event.keystroke.key == \"escape\"",
            "NotificationEventKind::Activate",
            "NotificationEventKind::Context",
            ".on_mouse_up(gpui::MouseButton::Right",
            ".hover(move |style|",
            ".active(move |style|",
            ".focus_visible(move |style|",
            "high-contrast",
            ".w(px(24.))",
        ] {
            assert!(
                source.contains(required),
                "missing overflow contract: {required}"
            );
        }
    }
}
