//! Owned, query-only taskbar status values.

use windows::Win32::{
    Globalization::GetUserDefaultLocaleName, System::SystemInformation::GetLocalTime,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

pub fn local_date_time() -> LocalDateTime {
    // SAFETY: GetLocalTime returns an owned SYSTEMTIME value and has no pointer lifetime.
    let value = unsafe { GetLocalTime() };
    LocalDateTime {
        year: i32::from(value.wYear),
        month: value.wMonth as u8,
        day: value.wDay as u8,
        hour: value.wHour as u8,
        minute: value.wMinute as u8,
    }
}

pub fn user_locale_name() -> Option<String> {
    let mut buffer = [0_u16; 85];
    // SAFETY: Windows writes a NUL-terminated locale name into fixed local storage.
    let length = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    (length > 1).then(|| String::from_utf16_lossy(&buffer[..length as usize - 1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_time_is_owned_and_within_windows_calendar_bounds() {
        let value = local_date_time();
        assert!((2020..=9999).contains(&value.year));
        assert!((1..=12).contains(&value.month));
        assert!((1..=31).contains(&value.day));
        assert!(value.hour <= 23);
        assert!(value.minute <= 59);
    }
}
