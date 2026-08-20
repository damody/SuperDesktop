use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px, rgb,
};

use crate::{WindowsGuiMetrics, taskbar_settings::CommandSurfaceTokens};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemControlContextKind {
    Input,
    Volume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemControlContextCommand {
    LanguagePreferences,
    OpenVolumeMixer,
    OpenSoundSettings,
}

impl SystemControlContextKind {
    pub const fn commands(self) -> &'static [SystemControlContextCommand] {
        match self {
            Self::Input => &[SystemControlContextCommand::LanguagePreferences],
            Self::Volume => &[
                SystemControlContextCommand::OpenVolumeMixer,
                SystemControlContextCommand::OpenSoundSettings,
            ],
        }
    }
}

pub type SystemControlContextAction = Rc<dyn Fn(SystemControlContextCommand, &mut gpui::App)>;
pub type SystemControlContextDismiss = Rc<dyn Fn(&mut Window, &mut gpui::App)>;

pub struct SystemControlContextView {
    kind: SystemControlContextKind,
    action: SystemControlContextAction,
    dismiss: SystemControlContextDismiss,
    focus: FocusHandle,
    _activation_subscription: Subscription,
}

impl SystemControlContextView {
    pub fn new(
        kind: SystemControlContextKind,
        action: SystemControlContextAction,
        dismiss: SystemControlContextDismiss,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                (this.dismiss)(window, cx);
            }
        });
        Self {
            kind,
            action,
            dismiss,
            focus: cx.focus_handle(),
            _activation_subscription: activation_subscription,
        }
    }
}

fn traditional_chinese() -> bool {
    std::env::var("SUPERDESKTOP_LOCALE").is_ok_and(|locale| locale.eq_ignore_ascii_case("zh-TW"))
}

fn label(command: SystemControlContextCommand, zh_tw: bool) -> &'static str {
    match (command, zh_tw) {
        (SystemControlContextCommand::LanguagePreferences, true) => "語言喜好設定",
        (SystemControlContextCommand::LanguagePreferences, false) => "Language preferences",
        (SystemControlContextCommand::OpenVolumeMixer, true) => "開啟音量混音程式",
        (SystemControlContextCommand::OpenVolumeMixer, false) => "Open volume mixer",
        (SystemControlContextCommand::OpenSoundSettings, true) => "音效設定",
        (SystemControlContextCommand::OpenSoundSettings, false) => "Sound settings",
    }
}

impl Render for SystemControlContextView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let tokens = CommandSurfaceTokens::current();
        let zh_tw = traditional_chinese();
        let dismiss = self.dismiss.clone();
        div()
            .id("owned-system-control-context")
            .role(gpui::Role::Menu)
            .aria_label(match self.kind {
                SystemControlContextKind::Input => {
                    label(SystemControlContextCommand::LanguagePreferences, zh_tw)
                }
                SystemControlContextKind::Volume => {
                    if zh_tw {
                        "音量功能表"
                    } else {
                        "Volume menu"
                    }
                }
            })
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p(px(WindowsGuiMetrics::CONTEXT_PADDING))
            .rounded(px(WindowsGuiMetrics::POPUP_RADIUS))
            .border_1()
            .border_color(rgb(tokens.border))
            .bg(rgb(tokens.background))
            .text_color(rgb(tokens.foreground))
            .shadow_lg()
            .flex()
            .flex_col()
            .on_key_down(
                cx.listener(move |_, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        dismiss(window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .children(
                self.kind
                    .commands()
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, command)| {
                        let click_action = self.action.clone();
                        let key_action = self.action.clone();
                        div()
                            .id(("system-control-context-command", index))
                            .role(gpui::Role::MenuItem)
                            .aria_label(label(command, zh_tw))
                            .tab_index(0)
                            .h(px(WindowsGuiMetrics::CONTEXT_ROW_HEIGHT))
                            .px(px(12.))
                            .rounded(px(4.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(move |style| style.bg(rgb(tokens.selected)))
                            .focus_visible(move |style| {
                                style.border_2().border_color(rgb(tokens.border))
                            })
                            .on_click(move |_, _, cx| click_action(command, cx))
                            .on_key_down(move |event, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    key_action(command, cx);
                                }
                            })
                            .child(label(command, zh_tw))
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_commands_are_fixed_by_control_kind() {
        assert_eq!(
            SystemControlContextKind::Input.commands(),
            &[SystemControlContextCommand::LanguagePreferences]
        );
        assert_eq!(
            SystemControlContextKind::Volume.commands(),
            &[
                SystemControlContextCommand::OpenVolumeMixer,
                SystemControlContextCommand::OpenSoundSettings
            ]
        );
    }

    #[test]
    fn owned_context_has_accessible_dismissal_contract() {
        let source = include_str!("system_control_context.rs");
        for required in [
            "owned-system-control-context",
            "Role::Menu",
            "Role::MenuItem",
            "observe_window_activation",
            "event.keystroke.key == \"escape\"",
            "Language preferences",
            "Open volume mixer",
            "Sound settings",
        ] {
            assert!(
                source.contains(required),
                "missing context contract: {required}"
            );
        }
    }
}
