//! Documented `IVirtualDesktopManager` adapter with owned results.

use std::ffi::c_void;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::Win32::UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager};
use windows::Win32::UI::WindowsAndMessaging::IsWindow;
use windows::core::GUID;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtualDesktopCapabilities {
    pub query_window: bool,
    pub move_window: bool,
    pub enumerate: bool,
    pub switch: bool,
    pub create: bool,
    pub remove: bool,
    pub rename: bool,
}

impl VirtualDesktopCapabilities {
    pub const UNAVAILABLE: Self = Self {
        query_window: false,
        move_window: false,
        enumerate: false,
        switch: false,
        create: false,
        remove: false,
        rename: false,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VirtualDesktopError {
    InvalidWindow,
    RetiredWindow,
    ManagerUnavailable(String),
    OperationFailed(String),
}

pub fn probe_capabilities() -> VirtualDesktopCapabilities {
    if manager().is_ok() {
        VirtualDesktopCapabilities {
            query_window: true,
            move_window: true,
            ..VirtualDesktopCapabilities::UNAVAILABLE
        }
    } else {
        VirtualDesktopCapabilities::UNAVAILABLE
    }
}

pub fn is_window_on_current_desktop(
    hwnd_value: isize,
    retired: bool,
) -> Result<bool, VirtualDesktopError> {
    let hwnd = admitted_window(hwnd_value, retired)?;
    let manager = manager()?;
    // SAFETY: The admitted HWND is observation-only and the COM interface owns
    // its own lifetime. No native pointer escapes this function.
    unsafe { manager.IsWindowOnCurrentVirtualDesktop(hwnd) }
        .map(|value| value.as_bool())
        .map_err(|error| VirtualDesktopError::OperationFailed(error.to_string()))
}

pub fn window_desktop_id(hwnd_value: isize, retired: bool) -> Result<u128, VirtualDesktopError> {
    let hwnd = admitted_window(hwnd_value, retired)?;
    let manager = manager()?;
    unsafe { manager.GetWindowDesktopId(hwnd) }
        .map(|id| id.to_u128())
        .map_err(|error| VirtualDesktopError::OperationFailed(error.to_string()))
}

pub fn move_window_to_desktop(
    hwnd_value: isize,
    retired: bool,
    desktop_id: u128,
) -> Result<(), VirtualDesktopError> {
    let hwnd = admitted_window(hwnd_value, retired)?;
    let manager = manager()?;
    let desktop_id = GUID::from_u128(desktop_id);
    unsafe { manager.MoveWindowToDesktop(hwnd, &desktop_id) }
        .map_err(|error| VirtualDesktopError::OperationFailed(error.to_string()))
}

fn manager() -> Result<IVirtualDesktopManager, VirtualDesktopError> {
    unsafe { CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_ALL) }
        .map_err(|error| VirtualDesktopError::ManagerUnavailable(error.to_string()))
}

fn admitted_window(hwnd_value: isize, retired: bool) -> Result<HWND, VirtualDesktopError> {
    if retired {
        return Err(VirtualDesktopError::RetiredWindow);
    }
    let hwnd = HWND(hwnd_value as *mut c_void);
    if unsafe { IsWindow(Some(hwnd)).as_bool() } {
        Ok(hwnd)
    } else {
        Err(VirtualDesktopError::InvalidWindow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_and_retired_windows_fail_before_com() {
        assert_eq!(
            window_desktop_id(0, false),
            Err(VirtualDesktopError::InvalidWindow)
        );
        assert_eq!(
            move_window_to_desktop(1, true, 1),
            Err(VirtualDesktopError::RetiredWindow)
        );
        let capabilities = probe_capabilities();
        assert!(
            !capabilities.enumerate
                && !capabilities.switch
                && !capabilities.create
                && !capabilities.remove
                && !capabilities.rename
        );
    }
}
