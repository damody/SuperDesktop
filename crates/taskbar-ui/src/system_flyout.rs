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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemFlyoutTheme {
    Light,
    Dark,
    HighContrast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemFlyoutPresentation {
    pub theme: SystemFlyoutTheme,
    pub traditional_chinese: bool,
}

impl SystemFlyoutPresentation {
    pub const fn new(theme: SystemFlyoutTheme, traditional_chinese: bool) -> Self {
        Self {
            theme,
            traditional_chinese,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SystemFlyoutChromeTokens {
    panel: u32,
    card: u32,
    border: u32,
    text: u32,
    secondary: u32,
    accent: u32,
    accent_text: u32,
    hover: u32,
    pressed: u32,
    selected: u32,
    focus: u32,
    unavailable: u32,
}

impl SystemFlyoutChromeTokens {
    const fn new(theme: SystemFlyoutTheme) -> Self {
        match theme {
            SystemFlyoutTheme::Light => Self {
                panel: 0xf3f3f3,
                card: 0xffffff,
                border: 0xd0d0d0,
                text: 0x1f1f1f,
                secondary: 0x5c5c5c,
                accent: 0x0067c0,
                accent_text: 0xffffff,
                hover: 0xe7e7e7,
                pressed: 0xdcdcdc,
                selected: 0xe5f1fb,
                focus: 0x005fb8,
                unavailable: 0x6b6b6b,
            },
            SystemFlyoutTheme::Dark => Self {
                panel: 0x202020,
                card: 0x2d2d2d,
                border: 0x454545,
                text: 0xffffff,
                secondary: 0xc8c8c8,
                accent: 0x60cdff,
                accent_text: 0x000000,
                hover: 0x3a3a3a,
                pressed: 0x454545,
                selected: 0x093d55,
                focus: 0x60cdff,
                unavailable: 0xa0a0a0,
            },
            SystemFlyoutTheme::HighContrast => Self {
                panel: 0x000000,
                card: 0x000000,
                border: 0xffffff,
                text: 0xffffff,
                secondary: 0xffffff,
                accent: 0xffff00,
                accent_text: 0x000000,
                hover: 0x1a1a1a,
                pressed: 0x303030,
                selected: 0x000000,
                focus: 0x00ffff,
                unavailable: 0xc0c0c0,
            },
        }
    }
}

pub struct SystemFlyoutView {
    pub kind: SystemFlyoutKind,
    pub snapshot: Option<SystemStatusSnapshot>,
    pub status: StatusRegion,
    presentation: SystemFlyoutPresentation,
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
        presentation: SystemFlyoutPresentation,
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
            presentation,
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

fn localized<'a>(presentation: SystemFlyoutPresentation, zh_tw: &'a str, en: &'a str) -> &'a str {
    if presentation.traditional_chinese {
        zh_tw
    } else {
        en
    }
}

fn compact_profile_tag(language_tag: &str) -> String {
    let normalized = language_tag.replace('_', "-");
    let primary = normalized.split('-').next().unwrap_or_default();
    if primary.eq_ignore_ascii_case("zh") {
        return "中".into();
    }
    if primary.eq_ignore_ascii_case("en") {
        return "ENG".into();
    }
    let bounded = primary
        .chars()
        .flat_map(char::to_uppercase)
        .take(3)
        .collect::<String>();
    if bounded.is_empty() {
        "—".into()
    } else {
        bounded
    }
}

fn calendar_weekdays(traditional_chinese: bool) -> [&'static str; 7] {
    if traditional_chinese {
        ["一", "二", "三", "四", "五", "六", "日"]
    } else {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    }
}

fn calendar_month_heading(calendar: &CalendarMonth, traditional_chinese: bool) -> String {
    const ENGLISH_MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    if traditional_chinese {
        format!("{}年{}月", calendar.year, calendar.month)
    } else {
        format!(
            "{} {}",
            ENGLISH_MONTHS[usize::from(calendar.month.saturating_sub(1))],
            calendar.year
        )
    }
}

fn network_summary(
    snapshot: Option<&SystemStatusSnapshot>,
    presentation: SystemFlyoutPresentation,
) -> (String, String, bool) {
    match snapshot.map(|snapshot| &snapshot.network) {
        Some(StatusAvailability::Available(network)) => (
            if network.display_name.trim().is_empty() {
                localized(presentation, "已連線的網路", "Connected network").into()
            } else {
                network.display_name.clone()
            },
            if network.internet {
                localized(presentation, "網際網路存取", "Internet access").into()
            } else if network.connected {
                localized(presentation, "無網際網路", "No Internet").into()
            } else {
                localized(presentation, "未連線", "Disconnected").into()
            },
            true,
        ),
        Some(StatusAvailability::NotPresent) => (
            localized(presentation, "找不到網路介面", "No network adapter").into(),
            localized(presentation, "未提供網路連線", "Network not present").into(),
            false,
        ),
        _ => (
            localized(presentation, "網路無法使用", "Network unavailable").into(),
            localized(presentation, "狀態提供者無法使用", "Status provider unavailable").into(),
            false,
        ),
    }
}

fn power_summary(
    snapshot: Option<&SystemStatusSnapshot>,
    presentation: SystemFlyoutPresentation,
) -> (String, bool) {
    match snapshot.map(|snapshot| &snapshot.power) {
        Some(StatusAvailability::Available(power)) => match power.battery_percent {
            Some(percent) => (
                if power.charging {
                    format!(
                        "{} {percent}% · {}",
                        localized(presentation, "電池", "Battery"),
                        localized(presentation, "充電中", "Charging")
                    )
                } else if power.ac_online {
                    format!(
                        "{} {percent}% · {}",
                        localized(presentation, "電池", "Battery"),
                        localized(presentation, "已接上電源", "Plugged in")
                    )
                } else {
                    format!("{} {percent}%", localized(presentation, "電池", "Battery"))
                },
                true,
            ),
            None => (
                localized(
                    presentation,
                    "交流電源 · 未偵測到電池",
                    "AC power · No battery detected",
                )
                .into(),
                true,
            ),
        },
        Some(StatusAvailability::NotPresent) => (
            localized(
                presentation,
                "交流電源 · 此裝置沒有電池",
                "AC power · This device has no battery",
            )
            .into(),
            true,
        ),
        _ => (
            localized(presentation, "電源狀態無法使用", "Power status unavailable").into(),
            false,
        ),
    }
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
        let presentation = self.presentation;
        let tokens = SystemFlyoutChromeTokens::new(presentation.theme);
        let volume = match (&self.status.core.volume, &self.status.core.muted) {
            (crate::ProviderState::Available(volume), crate::ProviderState::Available(muted)) => {
                Some((*volume, *muted))
            }
            _ => None,
        };
        let calendar = calendar_month(&self.status.date);
        let (network_name, network_detail, network_available) =
            network_summary(snapshot.as_ref(), presentation);
        let (power_text, power_available) = power_summary(snapshot.as_ref(), presentation);

        div()
            .id("owned-system-flyout")
            .role(gpui::Role::Dialog)
            .aria_label(match kind {
                SystemFlyoutKind::Input => {
                    localized(presentation, "輸入法與鍵盤配置", "Input languages")
                }
                SystemFlyoutKind::Volume => localized(presentation, "音量", "Volume"),
                SystemFlyoutKind::NetworkPower => {
                    localized(presentation, "網路與電源", "Network and power")
                }
                SystemFlyoutKind::Calendar => localized(presentation, "日期與時間", "Calendar"),
            })
            .tab_index(0)
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(tokens.border))
            .bg(rgb(tokens.panel))
            .text_color(rgb(tokens.text))
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
                                .text_size(px(16.))
                                .mb_2()
                                .child(localized(
                                    presentation,
                                    "鍵盤配置",
                                    "Keyboard layout",
                                )),
                        )
                        .children(input.profiles.into_iter().map(|profile| {
                            let click_action = action.clone();
                            let key_action = action.clone();
                            let click_id = profile.id.clone();
                            let key_id = profile.id.clone();
                            let active = profile.id == input.active_profile_id;
                            let accessible_name = format!(
                                "{} ({}, {}){}",
                                profile.display_name,
                                profile.language_tag,
                                profile.id,
                                if active {
                                    localized(presentation, "，使用中", ", active")
                                } else {
                                    ""
                                }
                            );
                            let tag = compact_profile_tag(&profile.language_tag);
                            div()
                                .id(format!("owned-input-profile-{}", profile.id))
                                .role(gpui::Role::Button)
                                .aria_label(accessible_name)
                                .tab_index(0)
                                .h(px(52.))
                                .px_3()
                                .rounded(px(8.))
                                .flex()
                                .items_center()
                                .gap_3()
                                .cursor_pointer()
                                .when(active, |entry| {
                                    entry
                                        .bg(rgb(tokens.selected))
                                        .border_2()
                                        .border_color(rgb(if presentation.theme
                                            == SystemFlyoutTheme::HighContrast
                                        {
                                            tokens.accent
                                        } else {
                                            tokens.selected
                                        }))
                                })
                                .hover(move |style| style.bg(rgb(tokens.hover)))
                                .active(move |style| style.bg(rgb(tokens.pressed)))
                                .focus_visible(move |style| {
                                    style.border_2().border_color(rgb(tokens.focus))
                                })
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
                                .child(
                                    div()
                                        .w(px(42.))
                                        .text_size(px(12.))
                                        .text_color(rgb(tokens.secondary))
                                        .child(tag),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(profile.display_name),
                                )
                                .when(active, |entry| {
                                    entry.child(
                                        div()
                                            .ml_auto()
                                            .text_color(rgb(tokens.accent))
                                            .text_size(px(18.))
                                            .child("✓"),
                                    )
                                })
                        })),
                    None => root.child(
                        div()
                            .id("owned-input-unavailable")
                            .role(gpui::Role::Status)
                            .text_color(rgb(tokens.unavailable))
                            .child(localized(
                                presentation,
                                "輸入設定檔無法使用",
                                "Input profiles unavailable",
                            )),
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
                            .text_size(px(16.))
                            .child(
                                svg()
                                    .external_path(concat!(
                                        env!("CARGO_MANIFEST_DIR"),
                                        "/assets/volume-status.svg"
                                    ))
                                    .w(px(20.))
                                    .h(px(20.))
                                    .text_color(rgb(tokens.text)),
                            )
                            .child(format!(
                                "{}  {current}%",
                                localized(presentation, "音量", "Volume")
                            )),
                    )
                    .child(
                        div()
                            .id("owned-volume-slider")
                            .role(gpui::Role::Slider)
                            .aria_label(localized(presentation, "音量", "Volume"))
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
                                    "home" => Some(0),
                                    "end" => Some(100),
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
                                    .bg(rgb(tokens.border)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(15.))
                                    .w(px(3.1 * f32::from(current)))
                                    .h(px(4.))
                                    .rounded_full()
                                    .bg(rgb(tokens.accent)),
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
                                    .border_color(rgb(tokens.accent))
                                    .bg(rgb(tokens.card)),
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
                                    .aria_label(localized(
                                        presentation,
                                        "降低音量",
                                        "Lower volume",
                                    ))
                                    .tab_index(0)
                                    .w(px(48.))
                                    .h(px(34.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(rgb(tokens.border))
                                    .bg(rgb(tokens.card))
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
                                    .aria_label(if muted {
                                        localized(presentation, "取消靜音", "Unmute")
                                    } else {
                                        localized(presentation, "靜音", "Mute")
                                    })
                                    .tab_index(0)
                                    .w(px(96.))
                                    .h(px(34.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(rgb(tokens.border))
                                    .bg(rgb(tokens.card))
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
                                        mute(SystemStatusAction::SetMute(!muted), cx);
                                    })
                                    .on_key_down(move |event, _, cx| {
                                        if activates_button(&event.keystroke.key) {
                                            mute_key(SystemStatusAction::SetMute(!muted), cx);
                                        }
                                    })
                                    .child(if muted {
                                        localized(presentation, "取消靜音", "Unmute")
                                    } else {
                                        localized(presentation, "靜音", "Mute")
                                    }),
                            )
                            .child(
                                div()
                                    .id("owned-volume-higher")
                                    .role(gpui::Role::Button)
                                    .aria_label(localized(
                                        presentation,
                                        "提高音量",
                                        "Raise volume",
                                    ))
                                    .tab_index(0)
                                    .w(px(48.))
                                    .h(px(34.))
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(rgb(tokens.border))
                                    .bg(rgb(tokens.card))
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
                        .text_color(rgb(tokens.unavailable))
                        .child(localized(
                            presentation,
                            "音量無法使用",
                            "Volume unavailable",
                        )),
                ),
            })
            .when(kind == SystemFlyoutKind::NetworkPower, |root| {
                root.child(
                    div()
                        .text_size(px(16.))
                        .mb_2()
                        .child(localized(presentation, "快速設定", "Quick settings")),
                )
                    .child(
                        div()
                            .id("owned-network-card")
                            .role(gpui::Role::Status)
                            .aria_label(format!("{network_name}. {network_detail}"))
                            .h(px(82.))
                            .p_3()
                            .rounded(px(8.))
                            .bg(rgb(if network_available {
                                tokens.selected
                            } else {
                                tokens.card
                            }))
                            .border_1()
                            .border_color(rgb(tokens.border))
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(44.))
                                    .h(px(44.))
                                    .rounded_full()
                                    .bg(rgb(if network_available {
                                        tokens.accent
                                    } else {
                                        tokens.border
                                    }))
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
                                            .text_color(rgb(if network_available {
                                                tokens.accent_text
                                            } else {
                                                tokens.unavailable
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(network_name)
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(if network_available {
                                                tokens.secondary
                                            } else {
                                                tokens.unavailable
                                            }))
                                            .child(network_detail),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("owned-power-summary")
                            .role(gpui::Role::Status)
                            .aria_label(power_text.clone())
                            .mt_2()
                            .p_3()
                            .rounded(px(8.))
                            .bg(rgb(tokens.card))
                            .border_1()
                            .border_color(rgb(tokens.border))
                            .text_color(rgb(if power_available {
                                tokens.text
                            } else {
                                tokens.unavailable
                            }))
                            .child(power_text),
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
                    .unwrap_or_else(|| {
                        localized(
                            presentation,
                            "日期時間提供者無法使用",
                            "Calendar provider unavailable",
                        )
                        .into()
                    });
                let Some(calendar) = calendar.clone() else {
                    return root.child(
                        div()
                            .role(gpui::Role::Status)
                            .text_color(rgb(tokens.unavailable))
                            .child(localized(
                                presentation,
                                "月曆無法使用",
                                "Calendar unavailable",
                            )),
                    );
                };
                let selected_day = calendar.selected_day;
                root.child(
                    div()
                        .id("owned-calendar-heading")
                        .role(gpui::Role::Heading)
                        .flex()
                        .items_end()
                        .child(
                            div()
                                .text_size(px(20.))
                                .child(format!("{}  {}", self.status.date, self.status.time)),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(tokens.secondary))
                        .mb_2()
                        .child(metadata),
                )
                .child(div().h(px(1.)).w_full().bg(rgb(tokens.border)))
                .child(
                    div()
                        .id("owned-calendar-month")
                        .mt_2()
                        .mb_2()
                        .text_size(px(16.))
                        .child(calendar_month_heading(
                            &calendar,
                            presentation.traditional_chinese,
                        )),
                )
                .child(
                    div().flex().children(
                        calendar_weekdays(presentation.traditional_chinese)
                            .into_iter()
                            .enumerate()
                            .map(|(index, day)| {
                                div()
                                    .id(format!("owned-calendar-weekday-{index}"))
                                    .w(px(48.))
                                    .text_size(px(11.))
                                    .text_color(rgb(tokens.secondary))
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
                                        .bg(rgb(tokens.accent))
                                        .text_color(rgb(tokens.accent_text))
                                        .when(
                                            presentation.theme
                                                == SystemFlyoutTheme::HighContrast,
                                            |cell| {
                                                cell.border_2().border_color(rgb(tokens.focus))
                                            },
                                        )
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
