use explorer_i18n::{Catalog, FluentArgs};

fn strip_isolates(value: String) -> String {
    value
        .chars()
        .filter(|ch| !matches!(*ch, '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}'))
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestClock {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl TestClock {
    pub fn format(self, catalog: Catalog) -> (String, String) {
        let hour_12 = match self.hour % 12 {
            0 => 12,
            hour => hour,
        };
        let period = if self.hour < 12 {
            catalog.t("desktop-clock-am")
        } else {
            catalog.t("desktop-clock-pm")
        };
        let mut args = FluentArgs::new();
        args.set("hour-12", format!("{hour_12:02}"));
        args.set("hour-24", format!("{:02}", self.hour));
        args.set("minute", format!("{:02}", self.minute));
        args.set("second", format!("{:02}", self.second));
        args.set("period", period);
        args.set("year", i64::from(self.year));
        args.set("month", i64::from(self.month));
        args.set("day", i64::from(self.day));
        (
            strip_isolates(catalog.t_args("desktop-clock-time", &args)),
            strip_isolates(catalog.t_args("desktop-clock-date", &args)),
        )
    }

    pub fn weekday(self, catalog: Catalog) -> String {
        const KEYS: [&str; 7] = [
            "desktop-weekday-sunday",
            "desktop-weekday-monday",
            "desktop-weekday-tuesday",
            "desktop-weekday-wednesday",
            "desktop-weekday-thursday",
            "desktop-weekday-friday",
            "desktop-weekday-saturday",
        ];
        weekday_sunday_zero(self.year, self.month, self.day)
            .map_or_else(|| "—".into(), |index| strip_isolates(catalog.t(KEYS[index])))
    }
    pub fn next_tick_delay_ms(second: u8, millisecond: u16) -> u64 {
        let second = second.min(59);
        let millisecond = millisecond.min(999);
        u64::from(59 - second) * 1_000 + u64::from(1_000 - millisecond)
    }
}

fn weekday_sunday_zero(year: i32, month: u8, day: u8) -> Option<usize> {
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    let adjusted_year = if month < 3 { year - 1 } else { year };
    Some(
        (adjusted_year + adjusted_year / 4 - adjusted_year / 100
            + adjusted_year / 400
            + OFFSETS[usize::from(month - 1)]
            + i32::from(day))
        .rem_euclid(7) as usize,
    )
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderState<T> {
    Available(T),
    Unavailable(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemFlyoutKind {
    Input,
    Volume,
    NetworkPower,
    Calendar,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SystemStatusAction {
    ActivateInputProfile(String),
    OpenLanguagePreferences,
    SetVolume(u8),
    SetMute(bool),
    RefreshWifi,
    ConnectWifi {
        interface_id: String,
        profile_name: String,
    },
    DisconnectWifi {
        interface_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreStatus {
    pub network: ProviderState<String>,
    pub volume: ProviderState<u8>,
    pub muted: ProviderState<bool>,
    pub input_language: ProviderState<String>,
    pub battery: ProviderState<u8>,
    pub notifications: ProviderState<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusRegion {
    pub time: String,
    pub weekday: String,
    pub date: String,
    pub core: CoreStatus,
    pub fake_tray_icons: Vec<String>,
}
impl StatusRegion {
    pub fn new(clock: TestClock, catalog: Catalog, core: CoreStatus) -> Self {
        let (time, date) = clock.format(catalog);
        Self {
            time,
            weekday: clock.weekday(catalog),
            date,
            core,
            fake_tray_icons: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use explorer_i18n::{AppLocale, Catalog};

    use super::*;

    fn zh() -> Catalog {
        Catalog::new(AppLocale::ZhTw)
    }

    fn en() -> Catalog {
        Catalog::new(AppLocale::En)
    }

    fn core() -> CoreStatus {
        CoreStatus {
            network: ProviderState::Available("online".into()),
            volume: ProviderState::Available(40),
            muted: ProviderState::Available(false),
            input_language: ProviderState::Available("zh-TW".into()),
            battery: ProviderState::Unavailable("desktop"),
            notifications: ProviderState::Available(2),
        }
    }
    #[test]
    fn locale_clock_is_deterministic() {
        let c = TestClock {
            year: 2026,
            month: 8,
            day: 19,
            hour: 15,
            minute: 30,
            second: 23,
        };
        assert_eq!(
            c.format(zh()),
            ("下午 03:30:23".into(), "2026/8/19".into())
        );
        assert_eq!(c.weekday(zh()), "星期三");
        assert_eq!(
            c.format(en()),
            ("03:30:23 PM".into(), "8/19/2026".into())
        );
        assert_eq!(c.weekday(en()), "Wednesday");
        let midnight = TestClock { hour: 0, ..c };
        let noon = TestClock { hour: 12, ..c };
        assert!(midnight.format(zh()).0.starts_with("上午 12:"));
        let noon_en = noon.format(en()).0;
        assert!(noon_en.starts_with("12:") && noon_en.ends_with("PM"));
        for (day, weekday) in [
            (16, "Sunday"),
            (17, "Monday"),
            (18, "Tuesday"),
            (19, "Wednesday"),
            (20, "Thursday"),
            (21, "Friday"),
            (22, "Saturday"),
        ] {
            assert_eq!(TestClock { day, ..c }.weekday(en()), weekday);
        }
        assert_ne!(
            TestClock {
                year: 2028,
                month: 2,
                day: 29,
                ..c
            }
            .weekday(en()),
            "—"
        );
        assert_eq!(TestClock::next_tick_delay_ms(30, 500), 29_500);
    }
    #[test]
    fn unavailable_provider_is_independent_and_no_fake_tray_is_rendered() {
        let region = StatusRegion::new(
            TestClock {
                year: 2026,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            en(),
            core(),
        );
        assert!(matches!(region.core.battery, ProviderState::Unavailable(_)));
        assert!(matches!(region.core.network, ProviderState::Available(_)));
        assert!(region.fake_tray_icons.is_empty())
    }
}
