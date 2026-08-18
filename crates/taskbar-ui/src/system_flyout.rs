use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::FluentBuilder as _, px,
    rgb, svg,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct CalendarMonth {
    year: i32,
    month: u8,
    selected_day: u8,
    cells: Vec<Option<u8>>,
}

fn calendar_month(date: &str) -> Option<CalendarMonth> {
    let mut parts = date.split('/');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let selected_day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) {
        return None;
    }
    let days = days_in_month(year, month);
    if !(1..=days).contains(&selected_day) {
        return None;
    }
    let leading = ((weekday_sunday_zero(year, month, 1) + 6) % 7) as usize;
    let mut cells = vec![None; 42];
    for day in 1..=days {
        cells[leading + usize::from(day - 1)] = Some(day);
    }
    Some(CalendarMonth {
        year,
        month,
        selected_day,
        cells,
    })
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn weekday_sunday_zero(year: i32, month: u8, day: u8) -> u8 {
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let adjusted_year = if month < 3 { year - 1 } else { year };
    ((adjusted_year + adjusted_year / 4 - adjusted_year / 100
        + adjusted_year / 400
        + OFFSETS[usize::from(month.saturating_sub(1))]
        + i32::from(day))
    .rem_euclid(7)) as u8
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
        let calendar = calendar_month(&self.status.date);

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
            .p_4()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(0xd0d0d0))
            .bg(rgb(0xf3f3f3))
            .text_color(rgb(0x1f1f1f))
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
                    Some(input) => root
                        .child(
                            div()
                                .id("owned-input-heading")
                                .text_size(px(18.))
                                .mb_2()
                                .child("Keyboard layout"),
                        )
                        .children(input.profiles.into_iter().map(|profile| {
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
                                .h(px(52.))
                                .px_3()
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .when(active, |entry| entry.bg(rgb(0xe5f1fb)))
                                .on_click(move |_, _, cx| {
                                    click_action(
                                        SystemStatusAction::ActivateInputProfile(click_id.clone()),
                                        cx,
                                    );
                                })
                                .on_key_down(move |event, _, cx| {
                                    if activates_button(&event.keystroke.key) {
                                        key_action(
                                            SystemStatusAction::ActivateInputProfile(
                                                key_id.clone(),
                                            ),
                                            cx,
                                        );
                                    }
                                })
                                .child(profile.display_name)
                                .when(active, |entry| {
                                    entry.child(
                                        div()
                                            .ml_auto()
                                            .text_color(rgb(0x0067c0))
                                            .text_size(px(18.))
                                            .child("✓"),
                                    )
                                })
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
                    root.child(
                        div()
                            .id("owned-volume-heading")
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_size(px(18.))
                            .child(
                                svg()
                                    .external_path(concat!(
                                        env!("CARGO_MANIFEST_DIR"),
                                        "/assets/volume-status.svg"
                                    ))
                                    .w(px(20.))
                                    .h(px(20.))
                                    .text_color(rgb(0x202020)),
                            )
                            .child(format!("Volume  {current}%")),
                    )
                    .child(
                        div()
                            .id("owned-volume-slider")
                            .role(gpui::Role::Slider)
                            .aria_label("Volume")
                            .aria_min_numeric_value(0.0)
                            .aria_max_numeric_value(100.0)
                            .aria_numeric_value(f64::from(current))
                            .tab_index(0)
                            .relative()
                            .w_full()
                            .h(px(34.))
                            .mt_2()
                            .mb_2()
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
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(15.))
                                    .w_full()
                                    .h(px(4.))
                                    .rounded_full()
                                    .bg(rgb(0xc9c9c9)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(15.))
                                    .w(px(3.1 * f32::from(current)))
                                    .h(px(4.))
                                    .rounded_full()
                                    .bg(rgb(0x0067c0)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px((3.1 * f32::from(current) - 7.0).max(0.0)))
                                    .top(px(9.))
                                    .w(px(16.))
                                    .h(px(16.))
                                    .rounded_full()
                                    .border_3()
                                    .border_color(rgb(0x0067c0))
                                    .bg(rgb(0xffffff)),
                            )
                            .children((0u8..=10).map(|step| {
                                let slider_click = action.clone();
                                let value = step * 10;
                                div()
                                    .id(format!("owned-volume-step-{value}"))
                                    .role(gpui::Role::Button)
                                    .aria_label(format!("Set volume to {value} percent"))
                                    .absolute()
                                    .left(px(f32::from(step) * 31.0))
                                    .top_0()
                                    .w(px(31.))
                                    .h(px(34.))
                                    .cursor_pointer()
                                    .opacity(0.01)
                                    .on_click(move |_, _, cx| {
                                        slider_click(SystemStatusAction::SetVolume(value), cx);
                                    })
                            })),
                    )
                    .child(
                        div()
                            .id("owned-volume-actions")
                            .flex()
                            .justify_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("owned-volume-lower")
                                    .role(gpui::Role::Button)
                                    .aria_label("Lower volume")
                                    .tab_index(0)
                                    .w(px(48.))
                                    .h(px(34.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(rgb(0xd0d0d0))
                                    .bg(rgb(0xffffff))
                                    .flex()
                                    .items_center()
                                    .justify_center()
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
                                    .w(px(96.))
                                    .h(px(34.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(rgb(0xd0d0d0))
                                    .bg(rgb(0xffffff))
                                    .flex()
                                    .items_center()
                                    .justify_center()
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
                                    .w(px(48.))
                                    .h(px(34.))
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(rgb(0xd0d0d0))
                                    .bg(rgb(0xffffff))
                                    .flex()
                                    .items_center()
                                    .justify_center()
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
                root.child(div().text_size(px(18.)).mb_2().child("Quick settings"))
                    .child(
                        div()
                            .id("owned-network-card")
                            .h(px(82.))
                            .p_3()
                            .rounded(px(8.))
                            .bg(rgb(0xe5f1fb))
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(44.))
                                    .h(px(44.))
                                    .rounded_full()
                                    .bg(rgb(0x0067c0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        svg()
                                            .external_path(concat!(
                                                env!("CARGO_MANIFEST_DIR"),
                                                "/assets/network-status.svg"
                                            ))
                                            .w(px(22.))
                                            .h(px(22.))
                                            .text_color(rgb(0xffffff)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(match &self.status.core.network {
                                        crate::ProviderState::Available(value) => value.clone(),
                                        crate::ProviderState::Unavailable(_) => {
                                            "Network unavailable".into()
                                        }
                                    })
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(0x5c5c5c))
                                            .child("Network and Internet"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("owned-power-summary")
                            .mt_2()
                            .p_3()
                            .rounded(px(8.))
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0xd8d8d8))
                            .child(match &self.status.core.battery {
                                crate::ProviderState::Available(value) => {
                                    format!("Battery  {value}%")
                                }
                                crate::ProviderState::Unavailable(_) => {
                                    "AC power · No battery detected".into()
                                }
                            }),
                    )
            })
            .when(kind == SystemFlyoutKind::Calendar, |root| {
                let metadata = snapshot
                    .as_ref()
                    .and_then(|snapshot| match &snapshot.clock {
                        StatusAvailability::Available(clock) => {
                            Some(format!("{} · {}", clock.locale, clock.time_zone))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| "Calendar provider unavailable".into());
                let Some(calendar) = calendar.clone() else {
                    return root.child("Calendar unavailable");
                };
                let selected_day = calendar.selected_day;
                root.child(
                    div().id("owned-calendar-heading").flex().items_end().child(
                        div()
                            .text_size(px(20.))
                            .child(format!("{}  {}", self.status.date, self.status.time)),
                    ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(0x5c5c5c))
                        .mb_2()
                        .child(metadata),
                )
                .child(div().h(px(1.)).w_full().bg(rgb(0xd8d8d8)))
                .child(
                    div()
                        .id("owned-calendar-month")
                        .mt_2()
                        .mb_2()
                        .text_size(px(16.))
                        .child(format!("{} / {:02}", calendar.year, calendar.month)),
                )
                .child(
                    div().flex().children(
                        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                            .into_iter()
                            .enumerate()
                            .map(|(index, day)| {
                                div()
                                    .id(format!("owned-calendar-weekday-{index}"))
                                    .w(px(48.))
                                    .text_size(px(11.))
                                    .text_color(rgb(0x5c5c5c))
                                    .flex()
                                    .justify_center()
                                    .child(day)
                            }),
                    ),
                )
                .child(
                    div()
                        .id("owned-calendar-grid")
                        .w(px(336.))
                        .flex()
                        .flex_wrap()
                        .children(calendar.cells.into_iter().enumerate().map(|(index, day)| {
                            let selected = day == Some(selected_day);
                            div()
                                .id(format!("owned-calendar-day-{index}"))
                                .w(px(48.))
                                .h(px(40.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(selected, |cell| {
                                    cell.rounded_full()
                                        .bg(rgb(0x0067c0))
                                        .text_color(rgb(0xffffff))
                                })
                                .child(day.map_or_else(String::new, |day| day.to_string()))
                        })),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::calendar_month;

    #[test]
    fn windows_style_calendar_grid_handles_leap_year_and_selected_day() {
        let month = calendar_month("2028/02/29").unwrap();
        assert_eq!(month.cells.iter().flatten().count(), 29);
        assert!(month.cells.contains(&Some(29)));
        assert_eq!(month.selected_day, 29);
        assert!(calendar_month("2026/13/01").is_none());
    }

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
            "owned-network-card",
            "owned-calendar-grid",
            "Keyboard layout",
        ] {
            assert!(
                source.contains(required),
                "missing owned flyout contract: {required}"
            );
        }
    }
}
