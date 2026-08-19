use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ObjectFit, ParentElement, Render,
    StatefulInteractiveElement, Styled, StyledImage, Subscription, Window, div, img,
    prelude::FluentBuilder as _, px, rgb, svg,
};
use shell_provider_protocol::{
    NotificationSnapshot, StatusAvailability, SystemStatusSnapshot, WifiNetwork,
};

use crate::{StatusRegion, SystemFlyoutKind, SystemStatusAction, view::icon_render_image};

pub type SystemFlyoutAction = Rc<dyn Fn(SystemStatusAction, &mut gpui::App)>;
pub type NotificationCenterActionHandler =
    Rc<dyn Fn(NotificationCenterAction, &mut gpui::App) -> Result<NotificationSnapshot, String>>;
pub type SystemFlyoutDismiss = Rc<dyn Fn(&mut Window, &mut gpui::App)>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationCenterAction {
    Dismiss {
        notification_id: String,
        expected_generation: u64,
    },
    ClearAll {
        expected_generation: u64,
    },
}

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
    pub notifications: Option<NotificationSnapshot>,
    presentation: SystemFlyoutPresentation,
    keyboard_settings_open: bool,
    action: SystemFlyoutAction,
    notification_action: NotificationCenterActionHandler,
    notification_error: Option<String>,
    dismiss: SystemFlyoutDismiss,
    focus: FocusHandle,
    _activation_subscription: Subscription,
}

impl SystemFlyoutView {
    pub fn new(
        kind: SystemFlyoutKind,
        snapshot: Option<SystemStatusSnapshot>,
        status: StatusRegion,
        notifications: Option<NotificationSnapshot>,
        presentation: SystemFlyoutPresentation,
        action: SystemFlyoutAction,
        notification_action: NotificationCenterActionHandler,
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
            notifications,
            presentation,
            keyboard_settings_open: false,
            action,
            notification_action,
            notification_error: None,
            dismiss,
            focus: cx.focus_handle(),
            _activation_subscription: activation_subscription,
        }
    }

    fn apply_notification_action(
        &mut self,
        action: NotificationCenterAction,
        cx: &mut Context<Self>,
    ) {
        match (self.notification_action)(action, cx) {
            Ok(snapshot) => {
                self.notifications = Some(snapshot);
                self.notification_error = None;
            }
            Err(error) => self.notification_error = Some(error),
        }
        cx.notify();
    }
}

fn activates_button(key: &str) -> bool {
    matches!(key, "enter" | "space")
}

fn wifi_row_action(network: &WifiNetwork) -> Option<SystemStatusAction> {
    if network.connected {
        Some(SystemStatusAction::DisconnectWifi {
            interface_id: network.interface_id.clone(),
        })
    } else if network.connectable {
        network
            .profile_name
            .clone()
            .map(|profile_name| SystemStatusAction::ConnectWifi {
                interface_id: network.interface_id.clone(),
                profile_name,
            })
    } else {
        None
    }
}

fn notification_time_label(admitted_unix_ms: u64) -> String {
    let minutes = admitted_unix_ms / 60_000;
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}

fn localized<'a>(presentation: SystemFlyoutPresentation, zh_tw: &'a str, en: &'a str) -> &'a str {
    if presentation.traditional_chinese {
        zh_tw
    } else {
        en
    }
}

#[cfg(test)]
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

fn input_profile_glyph(language_tag: &str) -> &'static str {
    let normalized = language_tag.replace('_', "-");
    let primary = normalized.split('-').next().unwrap_or_default();
    if primary.eq_ignore_ascii_case("zh") && normalized.to_ascii_lowercase().contains("-cn") {
        "拼"
    } else if primary.eq_ignore_ascii_case("zh") {
        "ㄅ"
    } else if primary.eq_ignore_ascii_case("en") {
        "A"
    } else {
        "鍵"
    }
}

fn input_profile_primary(
    language_tag: &str,
    fallback: &str,
    presentation: SystemFlyoutPresentation,
) -> String {
    let normalized = language_tag.replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh-tw") {
        localized(
            presentation,
            "繁體中文（台灣）",
            "Chinese (Traditional, Taiwan)",
        )
        .into()
    } else if normalized.starts_with("zh-cn") {
        localized(
            presentation,
            "簡體中文（中國）",
            "Chinese (Simplified, China)",
        )
        .into()
    } else if normalized.starts_with("en") {
        localized(presentation, "英文", "English").into()
    } else {
        fallback.into()
    }
}

fn input_profile_subtitle(
    language_tag: &str,
    presentation: SystemFlyoutPresentation,
) -> &'static str {
    let normalized = language_tag.replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh-cn") {
        localized(presentation, "微軟拼音", "Microsoft Pinyin")
    } else if normalized.starts_with("zh-tw") {
        localized(presentation, "微軟注音", "Microsoft Bopomofo")
    } else {
        localized(presentation, "鍵盤", "Keyboard")
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
            localized(
                presentation,
                "狀態提供者無法使用",
                "Status provider unavailable",
            )
            .into(),
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
        let notification_snapshot = self.notifications.clone();
        let notification_error = self.notification_error.clone();
        let presentation = self.presentation;
        let keyboard_settings_open = self.keyboard_settings_open;
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
        let wifi = snapshot
            .as_ref()
            .and_then(|snapshot| match &snapshot.network {
                StatusAvailability::Available(network) => Some(network.wifi.clone()),
                _ => None,
            });
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
                                .h(px(36.))
                                .flex()
                                .items_center()
                                .text_size(px(18.))
                                .mb_2()
                                .child(if keyboard_settings_open {
                                    localized(presentation, "鍵盤設定", "Keyboard settings")
                                } else {
                                    localized(presentation, "鍵盤配置", "Keyboard layout")
                                })
                                .when(!keyboard_settings_open, |heading| {
                                    heading.child(
                                        div()
                                            .ml_auto()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .text_size(px(12.))
                                            .text_color(rgb(tokens.secondary))
                                            .child(
                                                div()
                                                    .h(px(24.))
                                                    .px_2()
                                                    .rounded(px(4.))
                                                    .border_1()
                                                    .border_color(rgb(tokens.border))
                                                    .bg(rgb(tokens.card))
                                                    .flex()
                                                    .items_center()
                                                    .child("⊞"),
                                            )
                                            .child("+")
                                            .child(
                                                div()
                                                    .h(px(24.))
                                                    .px_2()
                                                    .rounded(px(4.))
                                                    .border_1()
                                                    .border_color(rgb(tokens.border))
                                                    .bg(rgb(tokens.card))
                                                    .flex()
                                                    .items_center()
                                                    .child(localized(
                                                        presentation,
                                                        "空格鍵",
                                                        "Space",
                                                    )),
                                            ),
                                    )
                                }),
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
                            let glyph = input_profile_glyph(&profile.language_tag);
                            let primary = input_profile_primary(
                                &profile.language_tag,
                                &profile.display_name,
                                presentation,
                            );
                            let subtitle =
                                input_profile_subtitle(&profile.language_tag, presentation);
                            let detail = if keyboard_settings_open {
                                format!("{subtitle} · {}", profile.id)
                            } else {
                                subtitle.into()
                            };
                            div()
                                .id(format!("owned-input-profile-{}", profile.id))
                                .role(gpui::Role::Button)
                                .aria_label(accessible_name)
                                .tab_index(0)
                                .relative()
                                .h(px(72.))
                                .px_4()
                                .rounded(px(8.))
                                .flex()
                                .items_center()
                                .gap_3()
                                .cursor_pointer()
                                .when(active, |entry| {
                                    entry.bg(rgb(tokens.selected)).border_2().border_color(rgb(
                                        if presentation.theme == SystemFlyoutTheme::HighContrast {
                                            tokens.accent
                                        } else {
                                            tokens.selected
                                        },
                                    ))
                                })
                                .when(active, |entry| {
                                    entry.child(
                                        div()
                                            .absolute()
                                            .left_0()
                                            .top(px(22.))
                                            .w(px(4.))
                                            .h(px(28.))
                                            .rounded_full()
                                            .bg(rgb(tokens.accent)),
                                    )
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
                                        .w(px(48.))
                                        .text_size(px(25.))
                                        .text_color(rgb(tokens.text))
                                        .child(glyph),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .whitespace_nowrap()
                                                .text_ellipsis()
                                                .text_size(px(15.))
                                                .child(primary),
                                        )
                                        .child(
                                            div()
                                                .whitespace_nowrap()
                                                .text_ellipsis()
                                                .text_size(px(12.))
                                                .text_color(rgb(tokens.secondary))
                                                .child(detail),
                                        ),
                                )
                        }))
                        .child(
                            div()
                                .id("owned-input-settings-footer")
                                .role(gpui::Role::Button)
                                .aria_label(if keyboard_settings_open {
                                    localized(
                                        presentation,
                                        "返回鍵盤配置",
                                        "Back to keyboard layouts",
                                    )
                                } else {
                                    localized(
                                        presentation,
                                        "更多鍵盤設定",
                                        "More keyboard settings",
                                    )
                                })
                                .tab_index(0)
                                .h(px(46.))
                                .mt_2()
                                .border_t_1()
                                .border_color(rgb(tokens.border))
                                .flex()
                                .items_center()
                                .px_3()
                                .rounded(px(6.))
                                .cursor_pointer()
                                .hover(move |style| style.bg(rgb(tokens.hover)))
                                .active(move |style| style.bg(rgb(tokens.pressed)))
                                .focus_visible(move |style| {
                                    style.border_2().border_color(rgb(tokens.focus))
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.keyboard_settings_open = !this.keyboard_settings_open;
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &gpui::KeyDownEvent, _, cx| {
                                        if activates_button(&event.keystroke.key) {
                                            this.keyboard_settings_open =
                                                !this.keyboard_settings_open;
                                            cx.notify();
                                        }
                                    },
                                ))
                                .child(if keyboard_settings_open {
                                    localized(presentation, "返回", "Back")
                                } else {
                                    localized(
                                        presentation,
                                        "更多鍵盤設定",
                                        "More keyboard settings",
                                    )
                                }),
                        ),
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
                                    .aria_label(localized(presentation, "降低音量", "Lower volume"))
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
                                    .aria_label(localized(presentation, "提高音量", "Raise volume"))
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
                let refresh = action.clone();
                let refresh_key = action.clone();
                root.child(
                    div()
                        .flex()
                        .items_center()
                        .mb_1()
                        .child(
                            div()
                                .id("owned-wifi-heading")
                                .role(gpui::Role::Heading)
                                .text_size(px(18.))
                                .child(localized(presentation, "Wi-Fi 網路", "Wi-Fi networks")),
                        )
                        .child(
                            div()
                                .id("owned-wifi-refresh")
                                .role(gpui::Role::Button)
                                .aria_label(localized(
                                    presentation,
                                    "重新整理 Wi-Fi 網路",
                                    "Refresh Wi-Fi networks",
                                ))
                                .tab_index(0)
                                .ml_auto()
                                .px_3()
                                .h(px(32.))
                                .rounded(px(6.))
                                .border_1()
                                .border_color(rgb(tokens.border))
                                .bg(rgb(tokens.card))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .hover(move |style| style.bg(rgb(tokens.hover)))
                                .active(move |style| style.bg(rgb(tokens.pressed)))
                                .focus_visible(move |style| {
                                    style.border_2().border_color(rgb(tokens.focus))
                                })
                                .on_click(move |_, _, cx| {
                                    refresh(SystemStatusAction::RefreshWifi, cx)
                                })
                                .on_key_down(move |event, _, cx| {
                                    if activates_button(&event.keystroke.key) {
                                        refresh_key(SystemStatusAction::RefreshWifi, cx);
                                    }
                                })
                                .child(localized(presentation, "重新整理", "Refresh")),
                        ),
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
                            div().flex().flex_col().gap_1().child(network_name).child(
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
                .child(match wifi {
                    Some(StatusAvailability::Available(wifi)) => {
                        let enabled = wifi.enabled;
                        let network_count = wifi.networks.len();
                        let action = action.clone();
                        div()
                            .id("owned-wifi-network-list")
                            .role(gpui::Role::Group)
                            .aria_label(localized(
                                presentation,
                                "可用的 Wi-Fi 網路",
                                "Available Wi-Fi networks",
                            ))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .when(network_count == 0, |list| {
                                list.child(
                                    div()
                                        .id("owned-wifi-empty")
                                        .role(gpui::Role::Status)
                                        .p_3()
                                        .text_color(rgb(tokens.secondary))
                                        .child(if enabled {
                                            localized(
                                                presentation,
                                                "找不到 Wi-Fi 網路",
                                                "No Wi-Fi networks found",
                                            )
                                        } else {
                                            localized(presentation, "Wi-Fi 已關閉", "Wi-Fi is off")
                                        }),
                                )
                            })
                            .children(wifi.networks.into_iter().enumerate().map(
                                |(index, network)| {
                                    let row_action = action.clone();
                                    let row_action_key = action.clone();
                                    let detail = if network.connected {
                                        localized(presentation, "已連線", "Connected").to_owned()
                                    } else if network.profile_name.is_some() {
                                        localized(presentation, "已儲存", "Saved").to_owned()
                                    } else if network.secure {
                                        localized(presentation, "需要密碼", "Password required")
                                            .to_owned()
                                    } else {
                                        localized(presentation, "開放式網路", "Open network")
                                            .to_owned()
                                    };
                                    let detail = format!("{detail} · {}%", network.signal_quality);
                                    let command = wifi_row_action(&network);
                                    let button_command = command.clone();
                                    let key_command = command.clone();
                                    div()
                                        .id(("owned-wifi-network", index))
                                        .role(gpui::Role::Group)
                                        .aria_label(format!("{}. {detail}", network.ssid))
                                        .min_h(px(64.))
                                        .p_2()
                                        .rounded(px(8.))
                                        .border_1()
                                        .border_color(rgb(tokens.border))
                                        .bg(rgb(if network.connected {
                                            tokens.selected
                                        } else {
                                            tokens.card
                                        }))
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            svg()
                                                .external_path(concat!(
                                                    env!("CARGO_MANIFEST_DIR"),
                                                    "/assets/network-status.svg"
                                                ))
                                                .w(px(24.))
                                                .h(px(24.))
                                                .text_color(rgb(if network.connected {
                                                    tokens.accent
                                                } else {
                                                    tokens.text
                                                })),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .flex()
                                                .flex_col()
                                                .child(div().line_clamp(1).child(network.ssid))
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(rgb(tokens.secondary))
                                                        .child(detail),
                                                ),
                                        )
                                        .when_some(button_command, move |row, command| {
                                            let label = if matches!(
                                                command,
                                                SystemStatusAction::DisconnectWifi { .. }
                                            ) {
                                                localized(presentation, "中斷連線", "Disconnect")
                                            } else {
                                                localized(presentation, "連線", "Connect")
                                            };
                                            row.child(
                                                div()
                                                    .id(("owned-wifi-action", index))
                                                    .role(gpui::Role::Button)
                                                    .aria_label(label)
                                                    .tab_index(0)
                                                    .px_3()
                                                    .h(px(32.))
                                                    .rounded(px(6.))
                                                    .border_1()
                                                    .border_color(rgb(tokens.border))
                                                    .cursor_pointer()
                                                    .hover(move |style| style.bg(rgb(tokens.hover)))
                                                    .active(move |style| {
                                                        style.bg(rgb(tokens.pressed))
                                                    })
                                                    .focus_visible(move |style| {
                                                        style
                                                            .border_2()
                                                            .border_color(rgb(tokens.focus))
                                                    })
                                                    .on_click(move |_, _, cx| {
                                                        row_action(command.clone(), cx)
                                                    })
                                                    .on_key_down(move |event, _, cx| {
                                                        if activates_button(&event.keystroke.key)
                                                            && let Some(command) = &key_command
                                                        {
                                                            row_action_key(command.clone(), cx);
                                                        }
                                                    })
                                                    .child(label),
                                            )
                                        })
                                },
                            ))
                    }
                    Some(StatusAvailability::NotPresent) => div()
                        .id("owned-wifi-not-present")
                        .role(gpui::Role::Status)
                        .p_3()
                        .text_color(rgb(tokens.unavailable))
                        .child(localized(
                            presentation,
                            "找不到 Wi-Fi 網路介面卡",
                            "No Wi-Fi adapter found",
                        )),
                    Some(StatusAvailability::Unavailable { .. }) | None => div()
                        .id("owned-wifi-unavailable")
                        .role(gpui::Role::Status)
                        .p_3()
                        .text_color(rgb(tokens.unavailable))
                        .child(localized(
                            presentation,
                            "Wi-Fi 提供者無法使用",
                            "Wi-Fi provider unavailable",
                        )),
                })
                .child(
                    div()
                        .id("owned-network-quick-tiles")
                        .role(gpui::Role::Group)
                        .aria_label(localized(
                            presentation,
                            "快速設定狀態",
                            "Quick settings status",
                        ))
                        .flex()
                        .gap_2()
                        .children(
                            [
                                localized(presentation, "Wi-Fi", "Wi-Fi"),
                                localized(
                                    presentation,
                                    "飛航模式（無法使用）",
                                    "Airplane mode (unavailable)",
                                ),
                                localized(
                                    presentation,
                                    "行動熱點（無法使用）",
                                    "Mobile hotspot (unavailable)",
                                ),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(index, label)| {
                                div()
                                    .id(("owned-network-quick-tile", index))
                                    .role(gpui::Role::Status)
                                    .flex_1()
                                    .min_h(px(42.))
                                    .p_2()
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(rgb(tokens.border))
                                    .bg(rgb(tokens.card))
                                    .text_size(px(11.))
                                    .text_color(rgb(if index == 0 {
                                        tokens.text
                                    } else {
                                        tokens.unavailable
                                    }))
                                    .child(label)
                            }),
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
                            .id("owned-calendar-unavailable")
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
                let notification_generation = notification_snapshot
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.generation);
                let notification_count = notification_snapshot
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.notifications.len());
                root.child(
                    div()
                        .id("owned-notification-center-heading")
                        .h(px(40.))
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .id("owned-notification-center-title")
                                .role(gpui::Role::Heading)
                                .flex_1()
                                .text_size(px(18.))
                                .child(localized(presentation, "通知", "Notifications")),
                        )
                        .when(notification_count > 0, |heading| {
                            heading.child(
                                div()
                                    .id("owned-notification-clear-all")
                                    .role(gpui::Role::Button)
                                    .aria_label(localized(
                                        presentation,
                                        "全部清除通知",
                                        "Clear all notifications",
                                    ))
                                    .tab_index(0)
                                    .px_2()
                                    .h(px(32.))
                                    .rounded(px(6.))
                                    .cursor_pointer()
                                    .hover(move |style| style.bg(rgb(tokens.hover)))
                                    .active(move |style| style.bg(rgb(tokens.pressed)))
                                    .focus_visible(move |style| {
                                        style.border_2().border_color(rgb(tokens.focus))
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.apply_notification_action(
                                            NotificationCenterAction::ClearAll {
                                                expected_generation: notification_generation,
                                            },
                                            cx,
                                        );
                                    }))
                                    .on_key_down(cx.listener(
                                        move |this, event: &gpui::KeyDownEvent, _, cx| {
                                            if activates_button(&event.keystroke.key) {
                                                this.apply_notification_action(
                                                    NotificationCenterAction::ClearAll {
                                                        expected_generation:
                                                            notification_generation,
                                                    },
                                                    cx,
                                                );
                                            }
                                        },
                                    ))
                                    .child(localized(presentation, "全部清除", "Clear all")),
                            )
                        }),
                )
                .when_some(notification_error, |root, error| {
                    root.child(
                        div()
                            .id("owned-notification-action-error")
                            .role(gpui::Role::Alert)
                            .text_size(px(12.))
                            .text_color(rgb(tokens.unavailable))
                            .child(error),
                    )
                })
                .child(match notification_snapshot.clone() {
                    None => div()
                        .id("owned-notification-provider-unavailable")
                        .role(gpui::Role::Status)
                        .h(px(72.))
                        .p_3()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(rgb(tokens.border))
                        .text_color(rgb(tokens.unavailable))
                        .child(localized(
                            presentation,
                            "通知提供者目前無法使用",
                            "Notification provider unavailable",
                        )),
                    Some(snapshot) if snapshot.notifications.is_empty() => div()
                        .id("owned-notification-empty")
                        .role(gpui::Role::Status)
                        .h(px(72.))
                        .p_3()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(rgb(tokens.border))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(tokens.secondary))
                        .child(localized(
                            presentation,
                            "沒有新的通知",
                            "No new notifications",
                        )),
                    Some(snapshot) => div()
                        .id("owned-notification-list")
                        .role(gpui::Role::List)
                        .aria_label(localized(presentation, "通知", "Notifications"))
                        .max_h(px(264.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(snapshot.notifications.into_iter().map(|notification| {
                            let notification_id = notification.notification_id.clone();
                            let dismiss_id = notification.notification_id.clone();
                            let expected_generation = snapshot.generation;
                            let key_expected_generation = snapshot.generation;
                            let icon = notification.icon.as_ref().and_then(icon_render_image);
                            let accessible_name = format!(
                                "{}. {}. {}. {}",
                                notification.application_label,
                                notification.title,
                                notification.body,
                                notification_time_label(notification.admitted_unix_ms)
                            );
                            div()
                                .id(format!("owned-notification-{notification_id}"))
                                .role(gpui::Role::ListItem)
                                .aria_label(accessible_name)
                                .tab_index(0)
                                .min_h(px(92.))
                                .p_3()
                                .rounded(px(8.))
                                .border_1()
                                .border_color(rgb(tokens.border))
                                .bg(rgb(tokens.card))
                                .flex()
                                .gap_2()
                                .focus_visible(move |style| {
                                    style.border_2().border_color(rgb(tokens.focus))
                                })
                                .on_key_down(cx.listener(
                                    move |this, event: &gpui::KeyDownEvent, _, cx| {
                                        if event.keystroke.key == "delete" {
                                            this.apply_notification_action(
                                                NotificationCenterAction::Dismiss {
                                                    notification_id: notification_id.clone(),
                                                    expected_generation: key_expected_generation,
                                                },
                                                cx,
                                            );
                                        }
                                    },
                                ))
                                .when_some(icon, |row, icon| {
                                    row.child(
                                        img(icon)
                                            .w(px(32.))
                                            .h(px(32.))
                                            .flex_none()
                                            .object_fit(ObjectFit::Contain),
                                    )
                                })
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .flex()
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .text_size(px(12.))
                                                        .text_ellipsis()
                                                        .child(notification.application_label),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(tokens.secondary))
                                                        .child(notification_time_label(
                                                            notification.admitted_unix_ms,
                                                        )),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .min_w_0()
                                                .text_ellipsis()
                                                .child(notification.title),
                                        )
                                        .child(
                                            div()
                                                .min_w_0()
                                                .text_size(px(12.))
                                                .text_color(rgb(tokens.secondary))
                                                .text_ellipsis()
                                                .line_clamp(3)
                                                .child(notification.body),
                                        ),
                                )
                                .child(
                                    div()
                                        .id(format!("owned-notification-dismiss-{dismiss_id}"))
                                        .role(gpui::Role::Button)
                                        .aria_label(localized(
                                            presentation,
                                            "關閉通知",
                                            "Dismiss notification",
                                        ))
                                        .tab_index(0)
                                        .w(px(32.))
                                        .h(px(32.))
                                        .flex_none()
                                        .rounded(px(6.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(rgb(tokens.hover)))
                                        .active(move |style| style.bg(rgb(tokens.pressed)))
                                        .focus_visible(move |style| {
                                            style.border_2().border_color(rgb(tokens.focus))
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.apply_notification_action(
                                                NotificationCenterAction::Dismiss {
                                                    notification_id: dismiss_id.clone(),
                                                    expected_generation,
                                                },
                                                cx,
                                            );
                                        }))
                                        .child("×"),
                                )
                        })),
                })
                .child(div().h(px(1.)).w_full().bg(rgb(tokens.border)))
                .child(
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
                                            presentation.theme == SystemFlyoutTheme::HighContrast,
                                            |cell| cell.border_2().border_color(rgb(tokens.focus)),
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
    use shell_provider_protocol::{PowerStatus, StatusAvailability, SystemStatusSnapshot};

    #[test]
    fn windows_style_calendar_grid_handles_leap_year_and_selected_day() {
        let month = super::calendar_month("2028/02/29").unwrap();
        assert_eq!(month.cells.iter().flatten().count(), 29);
        assert!(month.cells.contains(&Some(29)));
        assert_eq!(month.selected_day, 29);
        assert!(super::calendar_month("2026/13/01").is_none());
        assert_eq!(super::calendar_month_heading(&month, true), "2028年2月");
        assert_eq!(
            super::calendar_month_heading(&month, false),
            "February 2028"
        );
        assert_eq!(
            super::calendar_weekdays(true),
            ["一", "二", "三", "四", "五", "六", "日"]
        );
    }

    #[test]
    fn flyout_tokens_locale_and_profile_tags_are_bounded_and_distinct() {
        let light = super::SystemFlyoutChromeTokens::new(super::SystemFlyoutTheme::Light);
        let dark = super::SystemFlyoutChromeTokens::new(super::SystemFlyoutTheme::Dark);
        let contrast = super::SystemFlyoutChromeTokens::new(super::SystemFlyoutTheme::HighContrast);
        assert_ne!(light.panel, dark.panel);
        assert_ne!(dark.panel, contrast.panel);
        assert_ne!(contrast.focus, contrast.accent);
        let zh = super::SystemFlyoutPresentation::new(super::SystemFlyoutTheme::Light, true);
        let en = super::SystemFlyoutPresentation::new(super::SystemFlyoutTheme::Dark, false);
        assert_eq!(super::localized(zh, "音量", "Volume"), "音量");
        assert_eq!(super::localized(en, "音量", "Volume"), "Volume");
        assert_eq!(super::compact_profile_tag("zh-TW"), "中");
        assert_eq!(super::compact_profile_tag("en_US"), "ENG");
        assert_eq!(super::compact_profile_tag("de-DE"), "DE");
        assert_eq!(super::compact_profile_tag(""), "—");
        assert!(super::compact_profile_tag("abcdef").chars().count() <= 3);
        assert_eq!(super::input_profile_glyph("zh-TW"), "ㄅ");
        assert_eq!(super::input_profile_glyph("zh-CN"), "拼");
        assert_eq!(
            super::input_profile_primary("zh-TW", "zh-TW", zh),
            "繁體中文（台灣）"
        );
        assert_eq!(super::input_profile_subtitle("zh-CN", zh), "微軟拼音");
        assert_eq!(super::notification_time_label(0), "00:00");
        assert_eq!(
            super::notification_time_label(23 * 60 * 60 * 1_000),
            "23:00"
        );
    }

    fn status_snapshot(power: StatusAvailability<PowerStatus>) -> SystemStatusSnapshot {
        SystemStatusSnapshot {
            host_generation: 1,
            snapshot_generation: 1,
            network: StatusAvailability::NotPresent,
            audio: StatusAvailability::NotPresent,
            power,
            clock: StatusAvailability::NotPresent,
            input: StatusAvailability::NotPresent,
            overflowed: false,
        }
    }

    #[test]
    fn network_and_power_summaries_distinguish_not_present_from_failure() {
        let presentation =
            super::SystemFlyoutPresentation::new(super::SystemFlyoutTheme::Light, false);
        let not_present = status_snapshot(StatusAvailability::NotPresent);
        let (network, _, network_available) =
            super::network_summary(Some(&not_present), presentation);
        let (power, power_available) = super::power_summary(Some(&not_present), presentation);
        assert!(network.contains("No network"));
        assert!(!network_available);
        assert!(power.contains("no battery"));
        assert!(power_available);

        let no_battery = status_snapshot(StatusAvailability::Available(PowerStatus {
            ac_online: true,
            charging: false,
            battery_percent: None,
        }));
        assert!(
            super::power_summary(Some(&no_battery), presentation)
                .0
                .contains("No battery")
        );

        let failed = status_snapshot(StatusAvailability::Unavailable {
            reason: "fixture".into(),
        });
        let (_, available) = super::power_summary(Some(&failed), presentation);
        assert!(!available);
    }

    #[test]
    fn wifi_rows_cover_empty_single_and_maximum_lists_with_exact_action_gating() {
        let network = |index: usize| shell_provider_protocol::WifiNetwork {
            interface_id: format!("interface-{index}"),
            ssid: format!("network-{index:02}"),
            profile_name: Some(format!("profile-{index}")),
            signal_quality: 80,
            secure: true,
            connected: false,
            connectable: true,
        };
        let empty: Vec<shell_provider_protocol::WifiNetwork> = Vec::new();
        assert!(empty.is_empty());
        let one = [network(0)];
        assert_eq!(one.len(), 1);
        assert_eq!(
            super::wifi_row_action(&one[0]),
            Some(crate::SystemStatusAction::ConnectWifi {
                interface_id: "interface-0".into(),
                profile_name: "profile-0".into(),
            })
        );
        let maximum = (0..shell_provider_protocol::MAX_WIFI_NETWORKS)
            .map(network)
            .collect::<Vec<_>>();
        assert_eq!(maximum.len(), shell_provider_protocol::MAX_WIFI_NETWORKS);

        let mut unsaved = maximum[1].clone();
        unsaved.profile_name = None;
        assert_eq!(super::wifi_row_action(&unsaved), None);
        let mut disconnected_unavailable = maximum[2].clone();
        disconnected_unavailable.connectable = false;
        assert_eq!(super::wifi_row_action(&disconnected_unavailable), None);
        let mut connected = maximum[3].clone();
        connected.connected = true;
        assert_eq!(
            super::wifi_row_action(&connected),
            Some(crate::SystemStatusAction::DisconnectWifi {
                interface_id: "interface-3".into(),
            })
        );
    }

    #[test]
    fn owned_flyout_contract_is_keyboard_accessible_and_has_no_fake_unavailable_action() {
        let source = include_str!("system_flyout.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "owned-system-flyout",
            "event.keystroke.key == \"escape\"",
            "owned-input-profile-",
            "owned-input-settings-footer",
            "More keyboard settings",
            "⊞",
            "input_profile_glyph",
            "input_profile_primary",
            "input_profile_subtitle",
            "owned-volume-actions",
            "owned-volume-slider",
            "Role::Slider",
            "\"home\" => Some(0)",
            "\"end\" => Some(100)",
            "observe_window_activation",
            "owned-input-unavailable",
            "owned-volume-unavailable",
            "owned-network-card",
            "owned-wifi-refresh",
            "owned-wifi-network-list",
            "owned-wifi-network",
            "owned-wifi-action",
            "owned-wifi-empty",
            "owned-wifi-not-present",
            "owned-wifi-unavailable",
            "owned-network-quick-tiles",
            ".overflow_y_scroll()",
            "SystemStatusAction::RefreshWifi",
            "SystemStatusAction::ConnectWifi",
            "SystemStatusAction::DisconnectWifi",
            "Password required",
            "Airplane mode (unavailable)",
            "Mobile hotspot (unavailable)",
            "owned-calendar-grid",
            "owned-notification-center-heading",
            "owned-notification-clear-all",
            "owned-notification-list",
            "owned-notification-empty",
            "owned-notification-provider-unavailable",
            "NotificationCenterAction::Dismiss",
            "NotificationCenterAction::ClearAll",
            "event.keystroke.key == \"delete\"",
            ".line_clamp(3)",
            "SystemFlyoutChromeTokens",
            ".hover(move |style|",
            ".active(move |style|",
            ".focus_visible(move |style|",
            "Keyboard layout",
        ] {
            assert!(
                source.contains(required),
                "missing owned flyout contract: {required}"
            );
        }
        for forbidden in [
            "explorer.exe",
            "Shell_TrayWnd",
            "ms-settings:",
            "StartMenuExperienceHost",
            "ShellExperienceHost",
        ] {
            assert!(!production.contains(forbidden));
        }
    }
}
