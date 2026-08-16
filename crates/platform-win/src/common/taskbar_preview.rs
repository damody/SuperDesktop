//! DWM preview capability admission for taskbar flyouts.

use std::ffi::c_void;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::DwmIsCompositionEnabled;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

pub const PREVIEW_DEADLINE_MS: u64 = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewUnavailable {
    InvalidWindow,
    RetiredWindow,
    CompositionDisabled,
    DeadlineExpired,
    ProbeFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewAdmission {
    Available { hwnd_value: isize },
    Unavailable(PreviewUnavailable),
}

pub fn admit_live_preview(
    hwnd_value: isize,
    retired: bool,
    requested_at_ms: u64,
    now_ms: u64,
) -> PreviewAdmission {
    if retired {
        return PreviewAdmission::Unavailable(PreviewUnavailable::RetiredWindow);
    }
    if now_ms.saturating_sub(requested_at_ms) > PREVIEW_DEADLINE_MS {
        return PreviewAdmission::Unavailable(PreviewUnavailable::DeadlineExpired);
    }
    let hwnd = HWND(hwnd_value as *mut c_void);
    // SAFETY: Observation-only checks against an opaque HWND value. No handle
    // ownership is assumed and no pointer is retained.
    if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return PreviewAdmission::Unavailable(PreviewUnavailable::InvalidWindow);
    }
    match unsafe { DwmIsCompositionEnabled() } {
        Ok(enabled) if enabled.as_bool() => PreviewAdmission::Available { hwnd_value },
        Ok(_) => PreviewAdmission::Unavailable(PreviewUnavailable::CompositionDisabled),
        Err(_) => PreviewAdmission::Unavailable(PreviewUnavailable::ProbeFailed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_retired_and_expired_requests_fail_closed() {
        assert_eq!(
            admit_live_preview(0, false, 0, 0),
            PreviewAdmission::Unavailable(PreviewUnavailable::InvalidWindow)
        );
        assert_eq!(
            admit_live_preview(1, true, 0, 0),
            PreviewAdmission::Unavailable(PreviewUnavailable::RetiredWindow)
        );
        assert_eq!(
            admit_live_preview(1, false, 0, 501),
            PreviewAdmission::Unavailable(PreviewUnavailable::DeadlineExpired)
        );
    }
}
