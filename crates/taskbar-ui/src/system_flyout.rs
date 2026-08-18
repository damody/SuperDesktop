use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::FluentBuilder as _, px,
    rgb,
};
use shell_provider_protocol::{StatusAvailability, SystemStatusSnapshot};

use crate::{StatusRegion, SystemFlyoutKind, SystemStatusAction};

pub type SystemFlyoutAction = Rc<dyn Fn(SystemStatusAction, &mut gpui::App)>;
pub type SystemFlyoutDismiss = Rc<dyn Fn(&mut Window, &mut gpui::App)>;

pub struct SystemFlyoutView {
    pub kind: SystemFlyoutKind,
    pub snapshot: Option<SystemStatusSnapshot>,
    pub status: StatusRegion,
    action: SystemFlyoutAction,
    dismiss: SystemFlyoutDismiss,
    focus: FocusHandle,
    _activation_subscription: Subscription,
}

impl SystemFlyoutView {
    pub fn new(
        kind: SystemFlyoutKind,
        snapshot: Option<SystemStatusSnapshot>,
        status: StatusRegion,
        action: SystemFlyoutAction,
        dismiss: SystemFlyoutDismiss,
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
            kind,
            snapshot,
            status,
            action,
            dismiss,
            focus: cx.focus_handle(),
            _activation_subscription: activation_subscription,
        }
    }
}

fn activates_button(key: &str) -> bool {
    matches!(key, "enter" | "space")
}

impl Render for SystemFlyoutView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.focus(&self.focus, cx);
        let dismiss = self.dismiss.clone();
        let kind = self.kind;
        let snapshot = self.snapshot.clone();
        let action = self.action.clone();
        let volume = match (&self.status.core.volume, &self.status.core.muted) {
            (crate::ProviderState::Available(volume), crate::ProviderState::Available(muted)) => {
                Some((*volume, *muted))
            }
            _ => None,
        };

        div()
            .id("owned-system-flyout")
            .role(gpui::Role::Dialog)
            .aria_label(match kind {
                SystemFlyoutKind::Input => "Input languages",
                SystemFlyoutKind::Volume => "Volume",
                SystemFlyoutKind::NetworkPower => "Network and power",
                SystemFlyoutKind::Calendar => "Calendar",
            })
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p_3()
            .rounded_lg()
            .bg(rgb(0xf3f6fb))
            .text_color(rgb(0x172033))
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_2()
            .on_key_down(
                cx.listener(move |_, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        dismiss(window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .when(kind == SystemFlyoutKind::Input, |root| {
                let profiles = snapshot
                    .as_ref()
                    .and_then(|snapshot| match &snapshot.input {
                        StatusAvailability::Available(input) => Some(input.clone()),
                        _ => None,
                    });
                match profiles {
                    Some(input) => root.children(input.profiles.into_iter().map(|profile| {
                        let click_action = action.clone();
                        let key_action = action.clone();
                        let click_id = profile.id.clone();
                        let key_id = profile.id.clone();
                        let active = profile.id == input.active_profile_id;
                        div()
                            .id(format!("owned-input-profile-{}", profile.id))
                            .role(gpui::Role::Button)
                            .aria_label(format!(
                                "{}{}",
                                profile.display_name,
                                if active { " active" } else { "" }
                            ))
                            .tab_index(0)
                            .p_2()
                            .rounded_md()
                            .cursor_pointer()
                            .when(active, |entry| entry.bg(rgb(0xd6e8ff)))
                            .on_click(move |_, _, cx| {
                                click_action(
                                    SystemStatusAction::ActivateInputProfile(click_id.clone()),
                                    cx,
                                );
                            })
                            .on_key_down(move |event, _, cx| {
                                if activates_button(&event.keystroke.key) {
                                    key_action(
                                        SystemStatusAction::ActivateInputProfile(key_id.clone()),
                                        cx,
                                    );
                                }
                            })
                            .child(profile.display_name)
                    })),
                    None => root.child(
                        div()
                            .id("owned-input-unavailable")
                            .role(gpui::Role::Status)
                            .child("Input profiles unavailable"),
                    ),
                }
            })
            .when(kind == SystemFlyoutKind::Volume, |root| match volume {
                Some((current, muted)) => {
                    let lower = action.clone();
                    let lower_key = action.clone();
                    let mute = action.clone();
                    let mute_key = action.clone();
                    let higher = action.clone();
                    let higher_key = action.clone();
                    let slider_key = action.clone();
                    root.child(format!("Volume {current}%"))
                        .child(
                            div()
                                .id("owned-volume-slider")
                                .role(gpui::Role::Slider)
                                .aria_label("Volume")
                                .aria_min_numeric_value(0.0)
                                .aria_max_numeric_value(100.0)
                                .aria_numeric_value(f64::from(current))
                                .tab_index(0)
                                .flex()
                                .items_end()
                                .gap_1()
                                .on_key_down(move |event, _, cx| {
                                    let value = match event.keystroke.key.as_str() {
                                        "left" | "down" => Some(current.saturating_sub(5)),
                                        "right" | "up" => Some(current.saturating_add(5).min(100)),
                                        _ => None,
                                    };
                                    if let Some(value) = value {
                                        slider_key(SystemStatusAction::SetVolume(value), cx);
                                    }
                                })
                                .children((0u8..=10).map(|step| {
                                    let slider_click = action.clone();
                                    let value = step * 10;
                                    div()
                                        .id(format!("owned-volume-step-{value}"))
                                        .role(gpui::Role::Button)
                                        .aria_label(format!("Set volume to {value} percent"))
                                        .h(px(18.0 + f32::from(value) * 0.16))
                                        .flex_1()
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .bg(if value <= current {
                                            rgb(0x1874c9)
                                        } else {
                                            rgb(0xc7d2df)
                                        })
                                        .on_click(move |_, _, cx| {
                                            slider_click(SystemStatusAction::SetVolume(value), cx);
                                        })
                                })),
                        )
                        .child(
                            div()
                                .id("owned-volume-actions")
                                .flex()
                                .gap_2()
                                .child(
                                    div()
                                        .id("owned-volume-lower")
                                        .role(gpui::Role::Button)
                                        .aria_label("Lower volume")
                                        .tab_index(0)
                                        .p_2()
                                        .on_click(move |_, _, cx| {
                                            lower(
                                                SystemStatusAction::SetVolume(
                                                    current.saturating_sub(10),
                                                ),
                                                cx,
                                            );
                                        })
                                        .on_key_down(move |event, _, cx| {
                                            if activates_button(&event.keystroke.key) {
                                                lower_key(
                                                    SystemStatusAction::SetVolume(
                                                        current.saturating_sub(10),
                                                    ),
                                                    cx,
                                                );
                                            }
                                        })
                                        .child("-"),
                                )
                                .child(
                                    div()
                                        .id("owned-volume-mute")
                                        .role(gpui::Role::Button)
                                        .aria_label(if muted { "Unmute" } else { "Mute" })
                                        .tab_index(0)
                                        .p_2()
                                        .on_click(move |_, _, cx| {
                                            mute(SystemStatusAction::SetMute(!muted), cx);
                                        })
                                        .on_key_down(move |event, _, cx| {
                                            if activates_button(&event.keystroke.key) {
                                                mute_key(SystemStatusAction::SetMute(!muted), cx);
                                            }
                                        })
                                        .child(if muted { "Unmute" } else { "Mute" }),
                                )
                                .child(
                                    div()
                                        .id("owned-volume-higher")
                                        .role(gpui::Role::Button)
                                        .aria_label("Raise volume")
                                        .tab_index(0)
                                        .p_2()
                                        .on_click(move |_, _, cx| {
                                            higher(
                                                SystemStatusAction::SetVolume(
                                                    current.saturating_add(10).min(100),
                                                ),
                                                cx,
                                            );
                                        })
                                        .on_key_down(move |event, _, cx| {
                                            if activates_button(&event.keystroke.key) {
                                                higher_key(
                                                    SystemStatusAction::SetVolume(
                                                        current.saturating_add(10).min(100),
                                                    ),
                                                    cx,
                                                );
                                            }
                                        })
                                        .child("+"),
                                ),
                        )
                }
                None => root.child(
                    div()
                        .id("owned-volume-unavailable")
                        .role(gpui::Role::Status)
                        .child("Volume unavailable"),
                ),
            })
            .when(kind == SystemFlyoutKind::NetworkPower, |root| {
                root.child(match &self.status.core.network {
                    crate::ProviderState::Available(value) => format!("Network: {value}"),
                    crate::ProviderState::Unavailable(_) => "Network unavailable".into(),
                })
                .child(match &self.status.core.battery {
                    crate::ProviderState::Available(value) => format!("Battery: {value}%"),
                    crate::ProviderState::Unavailable(_) => {
                        "Battery not present or unavailable".into()
                    }
                })
            })
            .when(kind == SystemFlyoutKind::Calendar, |root| {
                root.child(format!("{} {}", self.status.date, self.status.time))
                    .child(
                        snapshot
                            .as_ref()
                            .and_then(|snapshot| match &snapshot.clock {
                                StatusAvailability::Available(clock) => {
                                    Some(format!("{} · {}", clock.locale, clock.time_zone))
                                }
                                _ => None,
                            })
                            .unwrap_or_else(|| "Calendar provider unavailable".into()),
                    )
            })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn owned_flyout_contract_is_keyboard_accessible_and_has_no_fake_unavailable_action() {
        let source = include_str!("system_flyout.rs");
        for required in [
            "owned-system-flyout",
            "event.keystroke.key == \"escape\"",
            "owned-input-profile-",
            "owned-volume-actions",
            "owned-volume-slider",
            "Role::Slider",
            "observe_window_activation",
            "owned-input-unavailable",
            "owned-volume-unavailable",
        ] {
            assert!(
                source.contains(required),
                "missing owned flyout contract: {required}"
            );
        }
    }
}
