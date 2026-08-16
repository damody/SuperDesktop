//! DWM preview capability admission for taskbar flyouts.

use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION,
    DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE, DwmIsCompositionEnabled, DwmRegisterThumbnail,
    DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
};
use windows::Win32::UI::WindowsAndMessaging::IsWindow;
use windows::core::BOOL;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThumbnailRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug)]
pub struct LiveThumbnail {
    handle: isize,
}

impl LiveThumbnail {
    pub fn register(destination: isize, source: isize) -> Result<Self, PreviewUnavailable> {
        if destination == source || destination == 0 || source == 0 {
            return Err(PreviewUnavailable::InvalidWindow);
        }
        let destination = HWND(destination as *mut c_void);
        let source = HWND(source as *mut c_void);
        // SAFETY: Observation and registration use live opaque HWND values. DWM
        // owns the returned thumbnail resource until our Drop implementation
        // unregisters it; neither window handle is owned or retained by Rust.
        if !unsafe { IsWindow(Some(destination)).as_bool() }
            || !unsafe { IsWindow(Some(source)).as_bool() }
        {
            return Err(PreviewUnavailable::InvalidWindow);
        }
        let handle = unsafe { DwmRegisterThumbnail(destination, source) }
            .map_err(|_| PreviewUnavailable::ProbeFailed)?;
        Ok(Self { handle })
    }

    pub fn update_destination(&self, destination: ThumbnailRect) -> Result<(), PreviewUnavailable> {
        if destination.right <= destination.left || destination.bottom <= destination.top {
            return Err(PreviewUnavailable::InvalidWindow);
        }
        let properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION
                | DWM_TNP_VISIBLE
                | DWM_TNP_OPACITY
                | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: RECT {
                left: destination.left,
                top: destination.top,
                right: destination.right,
                bottom: destination.bottom,
            },
            opacity: u8::MAX,
            fVisible: BOOL(1),
            fSourceClientAreaOnly: BOOL(1),
            ..Default::default()
        };
        // SAFETY: The resource is live for self's lifetime and the packed
        // properties value remains valid for the synchronous DWM call.
        unsafe { DwmUpdateThumbnailProperties(self.handle, &raw const properties) }
            .map_err(|_| PreviewUnavailable::ProbeFailed)
    }
}

impl Drop for LiveThumbnail {
    fn drop(&mut self) {
        // SAFETY: The thumbnail handle was returned by DwmRegisterThumbnail and
        // is unregistered exactly once by this owning Drop implementation.
        let _ = unsafe { DwmUnregisterThumbnail(self.handle) };
    }
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
        assert_eq!(
            LiveThumbnail::register(0, 0).unwrap_err(),
            PreviewUnavailable::InvalidWindow
        );
    }
}
