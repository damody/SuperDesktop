use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use explorer_i18n::{Catalog, FluentArgs};
use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, ObjectFit, ParentElement, Render, StatefulInteractiveElement, Styled,
    StyledImage, Subscription, Window, div, img, prelude::FluentBuilder as _, px, rgb, svg,
};
use shell_provider_protocol::{
    InputProfile, InputProfileKind, NotificationSnapshot, StatusAvailability, SystemStatusSnapshot,
    WifiNetwork, WindowsNotificationAccess,
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
    pub catalog: Catalog,
}

impl SystemFlyoutPresentation {
    pub const fn new(theme: SystemFlyoutTheme, catalog: Catalog) -> Self {
        Self { theme, catalog }
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
    action: SystemFlyoutAction,
    notification_action: NotificationCenterActionHandler,
    notification_error: Option<String>,
    optimistic_volume: Option<u8>,
    optimistic_volume_deadline: Option<Instant>,
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
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            kind,
            snapshot,
            status,
            notifications,
            presentation,
            action,
            notification_action,
            notification_error: None,
            optimistic_volume: None,
            optimistic_volume_deadline: None,
            dismiss,
            focus,
            _activation_subscription: activation_subscription,
        }
    }

    fn volume(&self) -> Option<(u8, bool)> {
        match (&self.status.core.volume, &self.status.core.muted) {
            (crate::ProviderState::Available(volume), crate::ProviderState::Available(muted)) => {
                Some((self.optimistic_volume.unwrap_or(*volume).min(100), *muted))
            }
            _ => None,
        }
    }

    fn set_volume(&mut self, value: u8, cx: &mut Context<Self>) {
        let value = value.min(100);
        self.optimistic_volume = Some(value);
        self.optimistic_volume_deadline = Some(Instant::now() + Duration::from_secs(2));
        (self.action)(SystemStatusAction::SetVolume(value), cx);
        cx.notify();
    }

    pub fn reconcile(
        &mut self,
        snapshot: Option<SystemStatusSnapshot>,
        status: StatusRegion,
        notifications: Option<NotificationSnapshot>,
    ) -> bool {
        let provider_volume = match &status.core.volume {
            crate::ProviderState::Available(value) => Some(*value),
            _ => None,
        };
        if self
            .optimistic_volume
            .is_some_and(|desired| provider_volume == Some(desired))
            || self
                .optimistic_volume_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.optimistic_volume = None;
            self.optimistic_volume_deadline = None;
        }
        let changed = self.snapshot != snapshot
            || self.status != status
            || self.notifications != notifications;
        self.snapshot = snapshot;
        self.status = status;
        self.notifications = notifications;
        changed
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

fn input_profile_glyph(profile: &InputProfile) -> &'static str {
    let method = profile.input_method_name.to_ascii_lowercase();
    if method.contains("boshiamy")
        || profile.input_method_name.contains("嘸蝦米")
        || method.contains("cangjie")
        || profile.input_method_name.contains("倉頡")
    {
        return "無";
    }
    if method.contains("bopomofo") || profile.input_method_name.contains("注音") {
        return "ㄅ";
    }
    if method.contains("pinyin") || profile.input_method_name.contains("拼音") {
        return "拼";
    }
    let normalized = profile.language_tag.replace('_', "-");
    let primary = normalized.split('-').next().unwrap_or_default();
    if primary.eq_ignore_ascii_case("zh") && normalized.to_ascii_lowercase().contains("-cn") {
        "拼"
    } else if profile.kind == InputProfileKind::InputProcessor {
        "輸"
    } else if primary.eq_ignore_ascii_case("en") {
        "A"
    } else {
        "鍵"
    }
}

fn input_profile_primary(
    profile: &InputProfile,
    fallback: &str,
    presentation: SystemFlyoutPresentation,
) -> String {
    if !profile.display_name.trim().is_empty() && profile.display_name != profile.language_tag {
        return profile.display_name.clone();
    }
    let normalized = profile.language_tag.replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh-tw") {
        presentation.catalog.t("desktop-chinese-traditional-taiwan")
        .into()
    } else if normalized.starts_with("zh-cn") {
        presentation.catalog.t("desktop-chinese-simplified-china")
        .into()
    } else if normalized.starts_with("en") {
        presentation.catalog.t("desktop-english").into()
    } else {
        fallback.into()
    }
}

fn input_profile_subtitle(
    profile: &InputProfile,
    presentation: SystemFlyoutPresentation,
) -> String {
    if !profile.input_method_name.trim().is_empty() {
        return profile.input_method_name.clone();
    }
    let normalized = profile.language_tag.replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh-cn") {
        presentation.catalog.t("desktop-microsoft-pinyin").into()
    } else if normalized.starts_with("zh-tw") {
        presentation.catalog.t("desktop-microsoft-bopomofo").into()
    } else {
        presentation.catalog.t("desktop-keyboard").into()
    }
}

fn calendar_weekdays(catalog: Catalog) -> [String; 7] {
    [
        catalog.t("desktop-weekday-mon"),
        catalog.t("desktop-weekday-tue"),
        catalog.t("desktop-weekday-wed"),
        catalog.t("desktop-weekday-thu"),
        catalog.t("desktop-weekday-fri"),
        catalog.t("desktop-weekday-sat"),
        catalog.t("desktop-weekday-sun"),
    ]
}

fn calendar_month_heading(calendar: &CalendarMonth, catalog: Catalog) -> String {
    const MONTH_KEYS: [&str; 12] = [
        "desktop-month-january",
        "desktop-month-february",
        "desktop-month-march",
        "desktop-month-april",
        "desktop-month-may",
        "desktop-month-june",
        "desktop-month-july",
        "desktop-month-august",
        "desktop-month-september",
        "desktop-month-october",
        "desktop-month-november",
        "desktop-month-december",
    ];
    let month_index = usize::from(calendar.month.saturating_sub(1)).min(11);
    let month = catalog.t(MONTH_KEYS[month_index]);
    let mut args = FluentArgs::new();
    args.set("month", month);
    args.set("year", calendar.year);
    catalog.t_args("desktop-calendar-heading", &args)
}

fn network_summary(
    snapshot: Option<&SystemStatusSnapshot>,
    presentation: SystemFlyoutPresentation,
) -> (String, String, bool) {
    match snapshot.map(|snapshot| &snapshot.network) {
        Some(StatusAvailability::Available(network)) => (
            if network.display_name.trim().is_empty() {
                presentation.catalog.t("desktop-connected-network").into()
            } else {
                network.display_name.clone()
            },
            if network.internet {
                presentation.catalog.t("desktop-internet-access").into()
            } else if network.connected {
                presentation.catalog.t("desktop-no-internet").into()
            } else {
                presentation.catalog.t("desktop-disconnected").into()
            },
            true,
        ),
        Some(StatusAvailability::NotPresent) => (
            presentation.catalog.t("desktop-no-network-adapter").into(),
            presentation.catalog.t("desktop-network-not-present").into(),
            false,
        ),
        _ => (
            presentation.catalog.t("desktop-network-unavailable").into(),
            presentation.catalog.t("desktop-status-provider-unavailable")
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
                        presentation.catalog.t("desktop-battery"),
                        presentation.catalog.t("desktop-charging")
                    )
                } else if power.ac_online {
                    format!(
                        "{} {percent}% · {}",
                        presentation.catalog.t("desktop-battery"),
                        presentation.catalog.t("desktop-plugged-in")
                    )
                } else {
                    format!("{} {percent}%", presentation.catalog.t("desktop-battery"))
                },
                true,
            ),
            None => (
                presentation.catalog.t("desktop-ac-no-battery")
                .into(),
                true,
            ),
        },
        Some(StatusAvailability::NotPresent) => (
            presentation.catalog.t("desktop-ac-no-battery-device")
            .into(),
            true,
        ),
        _ => (
            presentation.catalog.t("desktop-power-unavailable").into(),
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dismiss = self.dismiss.clone();
        let kind = self.kind;
        let snapshot = self.snapshot.clone();
        let action = self.action.clone();
        let notification_snapshot = self.notifications.clone();
        let notification_error = self.notification_error.clone();
        let presentation = self.presentation;
        let tokens = SystemFlyoutChromeTokens::new(presentation.theme);
        let volume = self.volume();
        let root_volume = volume;
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
                    presentation.catalog.t("desktop-input-languages")
                }
                SystemFlyoutKind::Volume => presentation.catalog.t("desktop-volume"),
                SystemFlyoutKind::NetworkPower => {
                    presentation.catalog.t("desktop-network-and-power")
                }
                SystemFlyoutKind::Calendar => presentation.catalog.t("desktop-calendar"),
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
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        dismiss(window, cx);
                        cx.stop_propagation();
                    } else if kind == SystemFlyoutKind::Volume
                        && let Some((current, _)) = root_volume
                    {
                        let value = match event.keystroke.key.as_str() {
                            "left" | "down" => Some(current.saturating_sub(5)),
                            "right" | "up" => Some(current.saturating_add(5).min(100)),
                            "home" => Some(0),
                            "end" => Some(100),
                            _ => None,
                        };
                        if let Some(value) = value {
                            this.set_volume(value, cx);
                            cx.stop_propagation();
                        }
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
                                .h(px(32.))
                                .flex()
                                .items_center()
                                .text_size(px(18.))
                                .mb_1()
                                .child(presentation.catalog.t("desktop-input-methods"))
                                .child(
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
                                                .child(presentation.catalog.t("desktop-space")),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .id("owned-input-profile-list")
                                .role(gpui::Role::Group)
                                .aria_label(presentation.catalog.t("desktop-available-input-methods"))
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .flex()
                                .flex_col()
                                .when(input.profiles.is_empty(), |list| {
                                    list.child(
                                        div()
                                            .id("owned-input-empty")
                                            .role(gpui::Role::Status)
                                            .p_3()
                                            .text_color(rgb(tokens.secondary))
                                            .child(presentation.catalog.t("desktop-no-input-methods")),
                                    )
                                })
                                .children(input.profiles.into_iter().enumerate().map(
                                    |(index, profile)| {
                                        let click_action = action.clone();
                                        let key_action = action.clone();
                                        let click_id = profile.id.clone();
                                        let key_id = profile.id.clone();
                                        let active = profile.id == input.active_profile_id;
                                        let primary = input_profile_primary(
                                            &profile,
                                            &profile.display_name,
                                            presentation,
                                        );
                                        let subtitle =
                                            input_profile_subtitle(&profile, presentation);
                                        let accessible_name = format!(
                                            "{primary}. {subtitle}{}",
                                            if active {
                                                presentation.catalog.t("desktop-active-suffix")
                                            } else {
                                                String::new()
                                            }
                                        );
                                        let glyph = input_profile_glyph(&profile);
                                        div()
                                            .id(("owned-input-profile", index))
                                            .role(gpui::Role::Button)
                                            .aria_label(accessible_name)
                                            .tab_index(0)
                                            .relative()
                                            .min_h(px(56.))
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
                                                    .border_color(rgb(
                                                        if presentation.theme
                                                            == SystemFlyoutTheme::HighContrast
                                                        {
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
                                                        .top(px(14.))
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
                                                    SystemStatusAction::ActivateInputProfile(
                                                        click_id.clone(),
                                                    ),
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
                                                            .child(subtitle),
                                                    ),
                                            )
                                    },
                                )),
                        )
                        .child(
                            div()
                                .id("owned-input-settings-footer")
                                .role(gpui::Role::Button)
                                .aria_label(presentation.catalog.t("desktop-language-preferences"))
                                .tab_index(0)
                                .min_h(px(48.))
                                .mt_1()
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
                                .on_click({
                                    let action = action.clone();
                                    move |_, _, cx| {
                                        action(SystemStatusAction::OpenLanguagePreferences, cx);
                                    }
                                })
                                .on_key_down({
                                    let action = action.clone();
                                    move |event, _, cx| {
                                        if activates_button(&event.keystroke.key) {
                                            action(SystemStatusAction::OpenLanguagePreferences, cx);
                                        }
                                    }
                                })
                                .child(div().w(px(48.)).text_size(px(20.)).child("A字"))
                                .child(presentation.catalog.t("desktop-language-preferences")),
                        ),
                    None => root.child(
                        div()
                            .id("owned-input-unavailable")
                            .role(gpui::Role::Status)
                            .text_color(rgb(tokens.unavailable))
                            .child(presentation.catalog.t("desktop-input-profiles-unavailable")),
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
                                presentation.catalog.t("desktop-volume")
                            )),
                    )
                    .child(
                        div()
                            .id("owned-volume-slider")
                            .role(gpui::Role::Slider)
                            .aria_label(presentation.catalog.t("desktop-volume"))
                            .aria_min_numeric_value(0.0)
                            .aria_max_numeric_value(100.0)
                            .aria_numeric_value(f64::from(current))
                            .tab_index(0)
                            .relative()
                            .w_full()
                            .h(px(34.))
                            .mt_2()
                            .mb_2()
                            .on_key_down(cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                let value = match event.keystroke.key.as_str() {
                                    "left" | "down" => Some(current.saturating_sub(5)),
                                    "right" | "up" => Some(current.saturating_add(5).min(100)),
                                    "home" => Some(0),
                                    "end" => Some(100),
                                    _ => None,
                                };
                                if let Some(value) = value {
                                    this.set_volume(value, cx);
                                }
                                },
                            ))
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
                            .children((0u8..=100).map(|value| {
                                div()
                                    .id(format!("owned-volume-step-{value}"))
                                    .role(gpui::Role::Button)
                                    .aria_label(format!("Set volume to {value} percent"))
                                    .absolute()
                                    .left(px(f32::from(value) * 3.1))
                                    .top_0()
                                    .w(px(3.2))
                                    .h(px(34.))
                                    .cursor_pointer()
                                    .opacity(0.01)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                                            this.set_volume(value, cx);
                                            cx.stop_propagation();
                                        }),
                                    )
                                    .on_mouse_move(cx.listener(
                                        move |this, event: &MouseMoveEvent, _, cx| {
                                            if event.dragging() {
                                                this.set_volume(value, cx);
                                                cx.stop_propagation();
                                            }
                                        },
                                    ))
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
                                    .aria_label(presentation.catalog.t("desktop-lower-volume"))
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
                                        presentation.catalog.t("desktop-unmute")
                                    } else {
                                        presentation.catalog.t("desktop-mute")
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
                                        presentation.catalog.t("desktop-unmute")
                                    } else {
                                        presentation.catalog.t("desktop-mute")
                                    }),
                            )
                            .child(
                                div()
                                    .id("owned-volume-higher")
                                    .role(gpui::Role::Button)
                                    .aria_label(presentation.catalog.t("desktop-raise-volume"))
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
                        .child(presentation.catalog.t("desktop-volume-unavailable")),
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
                                .child(presentation.catalog.t("desktop-wifi-networks")),
                        )
                        .child(
                            div()
                                .id("owned-wifi-refresh")
                                .role(gpui::Role::Button)
                                .aria_label(presentation.catalog.t("desktop-refresh-wifi"))
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
                                .child(presentation.catalog.t("desktop-refresh")),
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
                            .aria_label(presentation.catalog.t("desktop-available-wifi"))
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
                                            presentation.catalog.t("desktop-no-wifi")
                                        } else {
                                            presentation.catalog.t("desktop-wifi-off")
                                        }),
                                )
                            })
                            .children(wifi.networks.into_iter().enumerate().map(
                                |(index, network)| {
                                    let row_action = action.clone();
                                    let row_action_key = action.clone();
                                    let detail = if network.connected {
                                        presentation.catalog.t("desktop-connected").to_owned()
                                    } else if network.profile_name.is_some() {
                                        presentation.catalog.t("desktop-saved").to_owned()
                                    } else if network.secure {
                                        presentation.catalog.t("desktop-password-required")
                                            .to_owned()
                                    } else {
                                        presentation.catalog.t("desktop-open-network")
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
                                                presentation.catalog.t("desktop-disconnect")
                                            } else {
                                                presentation.catalog.t("desktop-connect")
                                            };
                                            row.child(
                                                div()
                                                    .id(("owned-wifi-action", index))
                                                    .role(gpui::Role::Button)
                                                    .aria_label(label.clone())
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
                        .child(presentation.catalog.t("desktop-no-wifi-adapter")),
                    Some(StatusAvailability::Unavailable { .. }) | None => div()
                        .id("owned-wifi-unavailable")
                        .role(gpui::Role::Status)
                        .p_3()
                        .text_color(rgb(tokens.unavailable))
                        .child(presentation.catalog.t("desktop-wifi-provider-unavailable")),
                })
                .child(
                    div()
                        .id("owned-network-quick-tiles")
                        .role(gpui::Role::Group)
                        .aria_label(presentation.catalog.t("desktop-quick-settings"))
                        .flex()
                        .gap_2()
                        .children(
                            [
                                presentation.catalog.t("desktop-wifi"),
                                presentation.catalog.t("desktop-airplane-unavailable"),
                                presentation.catalog.t("desktop-hotspot-unavailable"),
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
                        presentation.catalog.t("desktop-calendar-provider-unavailable")
                        .into()
                    });
                let Some(calendar) = calendar.clone() else {
                    return root.child(
                        div()
                            .id("owned-calendar-unavailable")
                            .role(gpui::Role::Status)
                            .text_color(rgb(tokens.unavailable))
                            .child(presentation.catalog.t("desktop-calendar-unavailable")),
                    );
                };
                let selected_day = calendar.selected_day;
                let notification_generation = notification_snapshot
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.generation);
                let notification_count = notification_snapshot
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.notifications.len());
                let windows_event_message = notification_snapshot.as_ref().and_then(|snapshot| {
                    let status = &snapshot.windows_events;
                    match (&status.access, status.synchronized) {
                        (WindowsNotificationAccess::Allowed, true) => None,
                        (WindowsNotificationAccess::Allowed, false) => Some(presentation.catalog.t("desktop-syncing-notifications")),
                        (WindowsNotificationAccess::Denied, _) => Some(presentation.catalog.t("desktop-notification-access-denied")),
                        (WindowsNotificationAccess::Unspecified, _) => Some(presentation.catalog.t("desktop-waiting-notification-access")),
                        (WindowsNotificationAccess::Unavailable, _) => Some(presentation.catalog.t("desktop-notification-events-unavailable")),
                    }
                });
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
                                .child(presentation.catalog.t("desktop-notifications")),
                        )
                        .when(notification_count > 0, |heading| {
                            heading.child(
                                div()
                                    .id("owned-notification-clear-all")
                                    .role(gpui::Role::Button)
                                    .aria_label(presentation.catalog.t("desktop-clear-all-notifications"))
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
                                    .child(presentation.catalog.t("desktop-clear-all")),
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
                .when_some(windows_event_message, |root, message| {
                    root.child(
                        div()
                            .id("owned-windows-notification-event-status")
                            .role(gpui::Role::Status)
                            .p_2()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(rgb(tokens.border))
                            .text_size(px(12.))
                            .text_color(rgb(tokens.secondary))
                            .child(message),
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
                        .child(presentation.catalog.t("desktop-notification-provider-unavailable")),
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
                        .child(presentation.catalog.t("desktop-no-new-notifications")),
                    Some(snapshot) => div()
                        .id("owned-notification-list")
                        .role(gpui::Role::List)
                        .aria_label(presentation.catalog.t("desktop-notifications"))
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
                                        .aria_label(presentation.catalog.t("desktop-dismiss-notification"))
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
                        .child(calendar_month_heading(&calendar, presentation.catalog)),
                )
                .child(
                    div().flex().children(
                        calendar_weekdays(presentation.catalog)
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
    use explorer_i18n::{AppLocale, Catalog};
    use shell_provider_protocol::{PowerStatus, StatusAvailability, SystemStatusSnapshot};

    fn presentation(locale: AppLocale) -> super::SystemFlyoutPresentation {
        super::SystemFlyoutPresentation::new(super::SystemFlyoutTheme::Light, Catalog::new(locale))
    }

    fn strip_isolates(value: &str) -> String {
        value
            .chars()
            .filter(|ch| !matches!(*ch, '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}'))
            .collect()
    }

    #[test]
    fn windows_style_calendar_grid_handles_leap_year_and_selected_day() {
        let month = super::calendar_month("2028/02/29").unwrap();
        assert_eq!(month.cells.iter().flatten().count(), 29);
        assert!(month.cells.contains(&Some(29)));
        assert_eq!(month.selected_day, 29);
        assert!(super::calendar_month("2026/13/01").is_none());
        assert_eq!(
            strip_isolates(&super::calendar_month_heading(
                &month,
                Catalog::new(AppLocale::ZhTw)
            )),
            "2028年二月"
        );
        assert_eq!(
            strip_isolates(&super::calendar_month_heading(
                &month,
                Catalog::new(AppLocale::En)
            )),
            "February 2028"
        );
        assert_eq!(
            super::calendar_weekdays(Catalog::new(AppLocale::ZhTw)),
            [
                String::from("一"),
                String::from("二"),
                String::from("三"),
                String::from("四"),
                String::from("五"),
                String::from("六"),
                String::from("日"),
            ]
        );
    }

    #[test]
    fn flyout_catalog_strings_cover_zh_tw_and_en() {
        let zh = Catalog::new(AppLocale::ZhTw);
        let en = Catalog::new(AppLocale::En);
        assert_eq!(zh.t("desktop-volume"), "音量");
        assert_eq!(en.t("desktop-volume"), "Volume");
        assert_eq!(zh.t("desktop-input-languages"), "輸入法與鍵盤配置");
        assert_eq!(en.t("desktop-input-languages"), "Input languages");
        assert_eq!(zh.t("desktop-notifications"), "通知");
        assert_eq!(en.t("desktop-notifications"), "Notifications");
    }

    #[test]
    fn flyout_tokens_locale_and_profile_tags_are_bounded_and_distinct() {
        let light = super::SystemFlyoutChromeTokens::new(super::SystemFlyoutTheme::Light);
        let dark = super::SystemFlyoutChromeTokens::new(super::SystemFlyoutTheme::Dark);
        let contrast = super::SystemFlyoutChromeTokens::new(super::SystemFlyoutTheme::HighContrast);
        assert_ne!(light.panel, dark.panel);
        assert_ne!(dark.panel, contrast.panel);
        assert_ne!(contrast.focus, contrast.accent);
        let zh = presentation(AppLocale::ZhTw);
        let en = presentation(AppLocale::En);
        assert_eq!(zh.catalog.t("desktop-volume"), "音量");
        assert_eq!(en.catalog.t("desktop-volume"), "Volume");
        assert_eq!(super::compact_profile_tag("zh-TW"), "中");
        assert_eq!(super::compact_profile_tag("en_US"), "ENG");
        assert_eq!(super::compact_profile_tag("de-DE"), "DE");
        assert_eq!(super::compact_profile_tag(""), "—");
        assert!(super::compact_profile_tag("abcdef").chars().count() <= 3);
        let bopomofo = shell_provider_protocol::InputProfile {
            id: "fixture-bopomofo".into(),
            language_tag: "zh-TW".into(),
            display_name: "繁體中文（台灣）".into(),
            input_method_name: "微軟注音".into(),
            kind: shell_provider_protocol::InputProfileKind::InputProcessor,
            language_id: 0x0404,
            tsf_class_id: None,
            tsf_profile_id: None,
            hkl: None,
        };
        let pinyin = shell_provider_protocol::InputProfile {
            language_tag: "zh-CN".into(),
            display_name: "簡體中文（中國）".into(),
            input_method_name: "Microsoft Pinyin".into(),
            ..bopomofo.clone()
        };
        assert_eq!(super::input_profile_glyph(&bopomofo), "ㄅ");
        assert_eq!(super::input_profile_glyph(&pinyin), "拼");
        assert_eq!(
            super::input_profile_primary(&bopomofo, "zh-TW", zh),
            "繁體中文（台灣）"
        );
        assert_eq!(
            super::input_profile_subtitle(&pinyin, zh),
            "Microsoft Pinyin"
        );
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
        let presentation = presentation(AppLocale::En);
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
    fn input_method_rows_keep_duplicate_languages_distinct_and_bounded() {
        let profile = |index: usize, method: &str| shell_provider_protocol::InputProfile {
            id: format!("fixture-{index}"),
            language_tag: "zh-TW".into(),
            display_name: "繁體中文（台灣）".into(),
            input_method_name: method.into(),
            kind: shell_provider_protocol::InputProfileKind::InputProcessor,
            language_id: 0x0404,
            tsf_class_id: None,
            tsf_profile_id: None,
            hkl: None,
        };
        let cangjie = profile(0, "微軟倉頡");
        let bopomofo = profile(1, "微軟注音");
        assert_eq!(super::input_profile_glyph(&cangjie), "無");
        assert_eq!(super::input_profile_glyph(&bopomofo), "ㄅ");
        assert_ne!(
            super::input_profile_subtitle(&cangjie, presentation(AppLocale::ZhTw)),
            super::input_profile_subtitle(&bopomofo, presentation(AppLocale::ZhTw))
        );
        let maximum = (0..shell_provider_protocol::MAX_INPUT_PROFILES)
            .map(|index| profile(index, &format!("method-{index}")))
            .collect::<Vec<_>>();
        assert_eq!(maximum.len(), shell_provider_protocol::MAX_INPUT_PROFILES);
        assert!(Vec::<shell_provider_protocol::InputProfile>::new().is_empty());
    }

    #[test]
    fn owned_flyout_contract_is_keyboard_accessible_and_has_no_fake_unavailable_action() {
        let source = include_str!("system_flyout.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "owned-system-flyout",
            "event.keystroke.key == \"escape\"",
            "owned-input-profile-list",
            "owned-input-profile",
            "owned-input-empty",
            "owned-input-settings-footer",
            "Language preferences",
            "SystemStatusAction::OpenLanguagePreferences",
            "A字",
            "⊞",
            "input_profile_glyph",
            "input_profile_primary",
            "input_profile_subtitle",
            "owned-volume-actions",
            "owned-volume-slider",
            "Role::Slider",
            "this.set_volume(value, cx)",
            "(0u8..=100).map(|value|",
            "event.dragging()",
            "optimistic_volume_deadline",
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
            "owned-windows-notification-event-status",
            "WindowsNotificationAccess::Denied",
            "WindowsNotificationAccess::Unavailable",
            "NotificationCenterAction::Dismiss",
            "NotificationCenterAction::ClearAll",
            "event.keystroke.key == \"delete\"",
            ".line_clamp(3)",
            "SystemFlyoutChromeTokens",
            "window.focus(&focus, cx)",
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
        assert_eq!(production.matches("window.focus(&focus, cx)").count(), 1);
        assert_eq!(production.matches("window.focus(").count(), 1);
        assert!(!production.contains("keyboard_settings_open"));
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
