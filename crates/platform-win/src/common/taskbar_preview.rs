//! DWM preview capability admission for taskbar flyouts.

use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION,
    DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE, DwmIsCompositionEnabled, DwmRegisterThumbnail,
    DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsWindow};
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
    source: isize,
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
        Ok(Self {
            handle,
            source: source.0 as isize,
        })
    }

    pub fn update_destination(&self, container: ThumbnailRect) -> Result<(), PreviewUnavailable> {
        let (source_width, source_height) = source_client_size(self.source)?;
        let destination = aspect_fit_thumbnail_rect(source_width, source_height, container)
            .ok_or(PreviewUnavailable::InvalidWindow)?;
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

pub fn source_client_size(source: isize) -> Result<(i32, i32), PreviewUnavailable> {
    if source == 0 {
        return Err(PreviewUnavailable::InvalidWindow);
    }
    let source = HWND(source as *mut c_void);
    // SAFETY: source is an opaque, query-only HWND and no ownership is assumed.
    if !unsafe { IsWindow(Some(source)).as_bool() } {
        return Err(PreviewUnavailable::InvalidWindow);
    }
    let mut source_client = RECT::default();
    // SAFETY: source is query-only and the output rectangle is local writable storage.
    unsafe { GetClientRect(source, &mut source_client) }
        .map_err(|_| PreviewUnavailable::RetiredWindow)?;
    let width = source_client.right - source_client.left;
    let height = source_client.bottom - source_client.top;
    if width <= 0 || height <= 0 {
        return Err(PreviewUnavailable::InvalidWindow);
    }
    Ok((width, height))
}

pub fn aspect_fit_thumbnail_rect(
    source_width: i32,
    source_height: i32,
    container: ThumbnailRect,
) -> Option<ThumbnailRect> {
    let container_width = container.right.checked_sub(container.left)?;
    let container_height = container.bottom.checked_sub(container.top)?;
    if source_width <= 0 || source_height <= 0 || container_width <= 0 || container_height <= 0 {
        return None;
    }
    let scale = (f64::from(container_width) / f64::from(source_width))
        .min(f64::from(container_height) / f64::from(source_height));
    let width = (f64::from(source_width) * scale)
        .round()
        .clamp(1.0, f64::from(container_width)) as i32;
    let height = (f64::from(source_height) * scale)
        .round()
        .clamp(1.0, f64::from(container_height)) as i32;
    let left = container.left + (container_width - width) / 2;
    let top = container.top + (container_height - height) / 2;
    Some(ThumbnailRect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    })
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

    #[test]
    fn thumbnail_destination_preserves_landscape_portrait_and_square_ratios() {
        let container = ThumbnailRect {
            left: 10,
            top: 20,
            right: 210,
            bottom: 220,
        };
        assert_eq!(
            aspect_fit_thumbnail_rect(1920, 1080, container),
            Some(ThumbnailRect {
                left: 10,
                top: 63,
                right: 210,
                bottom: 176,
            })
        );
        assert_eq!(
            aspect_fit_thumbnail_rect(1080, 1920, container),
            Some(ThumbnailRect {
                left: 53,
                top: 20,
                right: 166,
                bottom: 220,
            })
        );
        assert_eq!(
            aspect_fit_thumbnail_rect(200, 200, container),
            Some(container)
        );
        assert_eq!(aspect_fit_thumbnail_rect(0, 200, container), None);
    }
}
