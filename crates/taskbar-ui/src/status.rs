#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockLocale {
    ZhTw,
    En,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestClock {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl TestClock {
    pub fn format(self, locale: ClockLocale) -> (String, String) {
        match locale {
            ClockLocale::ZhTw => (
                format!("{:02}:{:02}", self.hour, self.minute),
                format!("{:04}/{:02}/{:02}", self.year, self.month, self.day),
            ),
            ClockLocale::En => (
                format!("{:02}:{:02}", self.hour, self.minute),
                format!("{:02}/{:02}/{:04}", self.month, self.day, self.year),
            ),
        }
    }
    pub fn next_tick_delay_ms(second: u8, millisecond: u16) -> u64 {
        let second = second.min(59);
        let millisecond = millisecond.min(999);
        u64::from(59 - second) * 1_000 + u64::from(1_000 - millisecond)
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
    pub date: String,
    pub core: CoreStatus,
    pub fake_tray_icons: Vec<String>,
}
impl StatusRegion {
    pub fn new(clock: TestClock, locale: ClockLocale, core: CoreStatus) -> Self {
        let (time, date) = clock.format(locale);
        Self {
            time,
            date,
            core,
            fake_tray_icons: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            day: 14,
            hour: 9,
            minute: 7,
        };
        assert_eq!(
            c.format(ClockLocale::ZhTw),
            ("09:07".into(), "2026/08/14".into())
        );
        assert_eq!(c.format(ClockLocale::En).1, "08/14/2026");
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
            },
            ClockLocale::En,
            core(),
        );
        assert!(matches!(region.core.battery, ProviderState::Unavailable(_)));
        assert!(matches!(region.core.network, ProviderState::Available(_)));
        assert!(region.fake_tray_icons.is_empty())
    }
}
