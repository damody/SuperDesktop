//! Owned taskbar window snapshots and validated foreground effects.

use std::{
    ffi::c_void,
    mem::size_of,
    os::windows::fs::MetadataExt,
    process::{Command, Stdio},
};

use windows::Win32::{
    Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    System::SystemInformation::GetWindowsDirectoryW,
    System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GWL_EXSTYLE, GetClientRect, GetForegroundWindow, GetWindow,
        GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, HWND_TOPMOST, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
        SW_MINIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetForegroundWindow,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_CLOSE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    },
};
use windows::core::{BOOL, PWSTR};

use super::ffi_boundary::{CallbackFence, CallbackResult};

const DWMWA_CLOAKED: u32 = 14;
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWCP_DONOTROUND: u32 = 1;
const DWMWA_COLOR_NONE: u32 = 0xffff_fffe;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmGetWindowAttribute(hwnd: HWND, attribute: u32, value: *mut c_void, size: u32) -> i32;
    fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32;
}

/// Resolves the inbox Task Manager from the Windows directory and launches the
/// exact canonical non-reparse file without shell parsing or environment lookup.
pub fn task_manager_path() -> Result<std::path::PathBuf, &'static str> {
    let mut buffer = vec![0_u16; 32_768];
    // SAFETY: Windows copies at most the supplied writable buffer length.
    let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err("task-manager-windows-directory");
    }
    let windows_directory = std::path::PathBuf::from(String::from_utf16_lossy(&buffer[..length]));
    let expected = windows_directory.join("System32").join("Taskmgr.exe");
    let canonical = expected
        .canonicalize()
        .map_err(|_| "task-manager-canonical")?;
    let canonical_system32 = windows_directory
        .join("System32")
        .canonicalize()
        .map_err(|_| "task-manager-system32")?;
    if canonical.parent() != Some(canonical_system32.as_path()) {
        return Err("task-manager-outside-system32");
    }
    let metadata = std::fs::symlink_metadata(&canonical).map_err(|_| "task-manager-metadata")?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err("task-manager-file-identity");
    }
    Ok(canonical)
}

pub fn launch_task_manager() -> Result<u32, &'static str> {
    let canonical = task_manager_path()?;
    let child = Command::new(&canonical)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "task-manager-spawn")?;
    Ok(child.id())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedTaskWindow {
    pub hwnd_identity: isize,
    pub process_id: u32,
    pub window_identity: String,
    pub application_identity: String,
    pub title: String,
    pub visible: bool,
    pub tool_window: bool,
    pub cloaked: bool,
    pub owned_transient: bool,
    pub minimized: bool,
    pub foreground: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedShellHookEvent {
    pub code: u32,
    pub hwnd_identity: isize,
}

pub fn invoke_shell_hook_callback(
    fence: &CallbackFence,
    code: u32,
    hwnd_identity: isize,
) -> CallbackResult<OwnedShellHookEvent> {
    fence.invoke(|| OwnedShellHookEvent {
        code,
        hwnd_identity,
    })
}

unsafe extern "system" fn enum_callback(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: pointer is supplied by the synchronous snapshot_task_windows call.
        let output = unsafe { &mut *(parameter.0 as *mut Vec<OwnedTaskWindow>) };
        if let Some(window) = snapshot_one(hwnd) {
            output.push(window)
        }
    }));
    BOOL(i32::from(outcome.is_ok()))
}

pub fn snapshot_task_windows() -> Result<Vec<OwnedTaskWindow>, String> {
    let mut windows: Vec<OwnedTaskWindow> = Vec::new();
    // SAFETY: callback and output pointer live for this synchronous enumeration.
    unsafe {
        EnumWindows(
            Some(enum_callback),
            LPARAM((&mut windows as *mut Vec<OwnedTaskWindow>) as isize),
        )
    }
    .map_err(|error| error.to_string())?;
    windows.sort_by(|a, b| a.window_identity.cmp(&b.window_identity));
    Ok(windows)
}

fn snapshot_one(hwnd: HWND) -> Option<OwnedTaskWindow> {
    let mut process_id = 0;
    // SAFETY: observation-only queries on an HWND supplied by EnumWindows.
    if unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) } == 0 || process_id == 0 {
        return None;
    }
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    let mut buffer = vec![0_u16; (length.max(0) as usize) + 1];
    let read = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    let title = String::from_utf16_lossy(&buffer[..read.max(0) as usize]);
    let application_identity =
        process_image(process_id).unwrap_or_else(|| format!("pid:{process_id}"));
    let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    let owner = unsafe { GetWindow(hwnd, GW_OWNER) }.ok();
    let mut cloaked = 0_u32;
    let cloak_result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            size_of::<u32>() as u32,
        )
    };
    let hwnd_identity = hwnd.0 as isize;
    Some(OwnedTaskWindow {
        hwnd_identity,
        process_id,
        window_identity: format!("win:{process_id}:{:X}", hwnd_identity as usize),
        application_identity,
        title,
        visible,
        tool_window: ex_style & WS_EX_TOOLWINDOW.0 != 0,
        cloaked: cloak_result == 0 && cloaked != 0,
        owned_transient: owner.is_some_and(|owner| !owner.is_invalid()),
        minimized: unsafe { IsIconic(hwnd).as_bool() },
        foreground: unsafe { GetForegroundWindow() } == hwnd,
    })
}

fn process_image(process_id: u32) -> Option<String> {
    // SAFETY: query-only process handle, closed exactly once below.
    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let mut buffer = vec![0_u16; 1024];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    let _ = unsafe { CloseHandle(process) };
    result.ok()?;
    Some(String::from_utf16_lossy(&buffer[..length as usize]).to_ascii_lowercase())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowAction {
    Activate,
    Minimize,
    RestoreAndActivate,
    Close,
}

/// Applies the non-activating topmost taskbar style to a GPUI-owned HWND and
/// shows it at the caller's DPI-adjusted monitor rectangle.
pub fn configure_and_show_taskbar_window(
    hwnd_identity: isize,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    let hwnd = HWND(hwnd_identity as *mut c_void);
    let mut owner_pid = 0;
    // SAFETY: style and position changes are admitted only for a live HWND
    // owned by this process; ownership and destruction stay with GPUI.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("taskbar-hwnd-invalid".into());
        }
        GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
        if owner_pid != GetCurrentProcessId() {
            return Err("taskbar-hwnd-foreign".into());
        }
        let existing = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            existing | (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW).0 as isize,
        );
        let corner_preference = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner_preference as *const u32).cast(),
            size_of::<u32>() as u32,
        );
        let border_color = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            (&border_color as *const u32).cast(),
            size_of::<u32>() as u32,
        );
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            left,
            top,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|error| error.to_string())?;

        // SetWindowPos consumes outer-window bounds. GPUI's popup still has
        // DPI-dependent non-client frame insets, while the shell contract is
        // expressed in client coordinates. Measure the live frame and make a
        // second placement so the rendered taskbar exactly owns the requested
        // monitor edge without clipping the status region.
        let mut window_rect = RECT::default();
        let mut client_rect = RECT::default();
        GetWindowRect(hwnd, &mut window_rect).map_err(|error| error.to_string())?;
        GetClientRect(hwnd, &mut client_rect).map_err(|error| error.to_string())?;
        let mut client_origin = POINT::default();
        if !ClientToScreen(hwnd, &mut client_origin).as_bool() {
            return Err("taskbar-client-origin-unavailable".into());
        }
        let frame_left = client_origin.x - window_rect.left;
        let frame_top = client_origin.y - window_rect.top;
        let client_width = client_rect.right - client_rect.left;
        let client_height = client_rect.bottom - client_rect.top;
        let frame_width = (window_rect.right - window_rect.left) - client_width;
        let frame_height = (window_rect.bottom - window_rect.top) - client_height;
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            left - frame_left,
            top - frame_top,
            width + frame_width,
            height + frame_height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn configure_and_show_popup_window(
    hwnd_identity: isize,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    if hwnd_identity == 0 || width <= 0 || height <= 0 {
        return Err("popup-window-invalid-geometry".into());
    }
    let hwnd = HWND(hwnd_identity as *mut c_void);
    if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return Err("popup-window-retired".into());
    }
    // SAFETY: applies validated outer-window geometry to the caller-owned live popup.
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            left,
            top,
            width,
            height,
            SWP_SHOWWINDOW,
        )
    }
    .map_err(|error| error.to_string())
}

pub fn apply_window_action(hwnd_identity: isize, action: WindowAction) -> Result<(), String> {
    let hwnd = HWND(hwnd_identity as *mut c_void);
    // SAFETY: mutation is admitted only for a currently valid top-level HWND.
    if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return Err("task-window-retired".into());
    }
    match action {
        WindowAction::Minimize => {
            let _ = unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
            Ok(())
        }
        WindowAction::Activate => unsafe { SetForegroundWindow(hwnd) }
            .ok()
            .map_err(|e| e.to_string()),
        WindowAction::RestoreAndActivate => {
            let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
            unsafe { SetForegroundWindow(hwnd) }
                .ok()
                .map_err(|e| e.to_string())
        }
        WindowAction::Close => {
            unsafe { PostMessageW(Some(hwnd), WM_CLOSE, Default::default(), Default::default()) }
                .map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ffi_boundary::CallbackFence;
    #[test]
    fn shell_hook_callback_is_owned_and_no_unwind() {
        let fence = CallbackFence::default();
        let event = invoke_shell_hook_callback(&fence, 1, 42);
        assert!(matches!(
            event,
            CallbackResult::Returned(OwnedShellHookEvent {
                code: 1,
                hwnd_identity: 42
            })
        ))
    }
    #[test]
    fn real_snapshot_has_owned_stable_fields() {
        for window in snapshot_task_windows().unwrap() {
            assert!(!window.window_identity.is_empty());
            assert!(!window.application_identity.is_empty())
        }
    }
    #[test]
    fn retired_window_action_fails_closed() {
        assert!(apply_window_action(1, WindowAction::Activate).is_err())
    }
    #[test]
    fn task_manager_path_is_canonical_system32_regular_file() {
        let path = task_manager_path().unwrap();
        assert!(path.is_absolute());
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("Taskmgr.exe")
        );
        let metadata = std::fs::symlink_metadata(path).unwrap();
        assert!(metadata.is_file());
        assert!(!metadata.file_type().is_symlink());
        assert_eq!(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT, 0);
    }
}
