//! Owned taskbar window snapshots and validated foreground effects.

use std::{
    cell::RefCell,
    collections::BTreeSet,
    ffi::c_void,
    mem::size_of,
    os::windows::fs::MetadataExt,
    panic::{AssertUnwindSafe, catch_unwind},
    process::{Command, Stdio},
    sync::OnceLock,
};

use windows::Win32::{
    Foundation::{CloseHandle, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::ClientToScreen,
    System::SystemInformation::GetWindowsDirectoryW,
    System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    },
    UI::{
        HiDpi::GetDpiForWindow,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            EnumWindows, GA_ROOT, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GetAncestor, GetClientRect,
            GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect,
            GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, HTCLIENT, HTTOP,
            HWND_TOPMOST, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
            RegisterWindowMessageW, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
            SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_CLOSE,
            WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCDESTROY, WM_NCHITTEST, WM_SIZING, WMSZ_TOP,
            WMSZ_TOPLEFT, WMSZ_TOPRIGHT, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_THICKFRAME,
            WindowFromPoint,
        },
    },
};
use windows::core::{BOOL, PWSTR, w};

use super::ffi_boundary::{CallbackFence, CallbackResult};

const DWMWA_CLOAKED: u32 = 14;
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWCP_DONOTROUND: u32 = 1;
const DWMWA_COLOR_NONE: u32 = 0xffff_fffe;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const TASKBAR_RESIZE_SUBCLASS_ID: usize = 0x5253_5a45;
static TASKBAR_AUTO_HIDE_REVEAL_MESSAGE: OnceLock<u32> = OnceLock::new();
thread_local! {
    static TASKBAR_RESIZE_SESSIONS: RefCell<BTreeSet<isize>> = const { RefCell::new(BTreeSet::new()) };
}

fn quantized_taskbar_outer_height(proposed_height: i32, dpi: u32) -> (u8, i32) {
    let row_height = ((40u32.saturating_mul(dpi.max(96)) + 48) / 96).max(1) as i32;
    let rows = ((proposed_height.max(1) + row_height / 2) / row_height).clamp(1, 3) as u8;
    (rows, row_height * i32::from(rows))
}
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmGetWindowAttribute(hwnd: HWND, attribute: u32, value: *mut c_void, size: u32) -> i32;
    fn DwmSetWindowAttribute(hwnd: HWND, attribute: u32, value: *const c_void, size: u32) -> i32;
}

unsafe extern "system" fn taskbar_resize_subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    ref_data: usize,
) -> LRESULT {
    if TASKBAR_AUTO_HIDE_REVEAL_MESSAGE
        .get()
        .is_some_and(|registered| *registered != 0 && message == *registered)
    {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let mut client = RECT::default();
            if unsafe { GetClientRect(hwnd, &mut client) }.is_ok() {
                let _ = configure_and_show_taskbar_window(
                    hwnd.0 as isize,
                    wparam.0 as i32,
                    lparam.0 as i32,
                    client.right - client.left,
                    client.bottom - client.top,
                );
            }
        }));
        return LRESULT(0);
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        TASKBAR_RESIZE_SESSIONS.with(|sessions| match message {
            WM_ENTERSIZEMOVE => {
                sessions.borrow_mut().insert(hwnd.0 as isize);
            }
            WM_EXITSIZEMOVE | WM_NCDESTROY => {
                sessions.borrow_mut().remove(&(hwnd.0 as isize));
            }
            _ => {}
        });
    }));
    if message == WM_NCDESTROY {
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(taskbar_resize_subclass_proc), id);
        }
    }
    if message == WM_NCHITTEST && ref_data == 1 {
        return LRESULT(HTCLIENT as isize);
    }
    if message == WM_NCHITTEST && ref_data == 0 {
        let mut window_rect = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut window_rect) }.is_ok() {
            let cursor_y = (lparam.0 >> 16) as i16 as i32;
            let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
            let resize_band = ((12u32.saturating_mul(dpi) + 48) / 96).max(1) as i32;
            if cursor_y <= window_rect.top + resize_band {
                return LRESULT(HTTOP as isize);
            }
        }
    }
    if message == WM_SIZING
        && matches!(wparam.0 as u32, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT)
        && lparam.0 != 0
    {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: WM_SIZING supplies a writable RECT for the duration of the callback.
            let proposed = unsafe { &mut *(lparam.0 as *mut RECT) };
            let dpi = unsafe { GetDpiForWindow(hwnd) };
            let proposed_height = (proposed.bottom - proposed.top).max(1);
            let (_, outer_height) = quantized_taskbar_outer_height(proposed_height, dpi);
            proposed.top = proposed.bottom - outer_height;
        }));
        return LRESULT(1);
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

fn taskbar_auto_hide_reveal_message() -> Result<u32, &'static str> {
    if let Some(message) = TASKBAR_AUTO_HIDE_REVEAL_MESSAGE.get().copied() {
        return (message != 0)
            .then_some(message)
            .ok_or("taskbar-reveal-message-zero");
    }
    let message = unsafe { RegisterWindowMessageW(w!("SuperDesktop.Taskbar.AutoHideReveal")) };
    if message == 0 {
        return Err("taskbar-reveal-message-register");
    }
    let _ = TASKBAR_AUTO_HIDE_REVEAL_MESSAGE.set(message);
    Ok(TASKBAR_AUTO_HIDE_REVEAL_MESSAGE
        .get()
        .copied()
        .unwrap_or(message))
}

pub fn owned_taskbar_resize_active() -> bool {
    TASKBAR_RESIZE_SESSIONS.with(|sessions| !sessions.borrow().is_empty())
}

pub fn post_owned_taskbar_reveal(
    hwnd_identity: isize,
    client_left: i32,
    client_top: i32,
) -> Result<(), String> {
    if hwnd_identity == 0 {
        return Err("taskbar-reveal-hwnd-zero".into());
    }
    let hwnd = HWND(hwnd_identity as *mut c_void);
    let mut owner_pid = 0;
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("taskbar-reveal-hwnd-retired".into());
        }
        if GetWindowThreadProcessId(hwnd, Some(&mut owner_pid)) == 0
            || owner_pid != GetCurrentProcessId()
        {
            return Err("taskbar-reveal-hwnd-foreign".into());
        }
        PostMessageW(
            Some(hwnd),
            taskbar_auto_hide_reveal_message().map_err(str::to_owned)?,
            WPARAM(client_left as usize),
            LPARAM(client_top as isize),
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
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

pub fn set_owned_taskbar_resizable(hwnd_identity: isize, resizable: bool) -> Result<bool, String> {
    if hwnd_identity == 0 {
        return Err("taskbar-resize-hwnd-zero".into());
    }
    let hwnd = HWND(hwnd_identity as *mut c_void);
    let _ = taskbar_auto_hide_reveal_message().map_err(str::to_owned)?;
    let mut owner_pid = 0;
    // SAFETY: all mutation follows liveness and current-process ownership checks.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("taskbar-resize-hwnd-invalid".into());
        }
        if GetWindowThreadProcessId(hwnd, Some(&mut owner_pid)) == 0
            || owner_pid != GetCurrentProcessId()
        {
            return Err("taskbar-resize-hwnd-foreign".into());
        }
        if !SetWindowSubclass(
            hwnd,
            Some(taskbar_resize_subclass_proc),
            TASKBAR_RESIZE_SUBCLASS_ID,
            usize::from(!resizable),
        )
        .as_bool()
        {
            return Err("taskbar-resize-subclass-install".into());
        }
        let before = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let thick_frame = WS_THICKFRAME.0 as isize;
        let after = if resizable {
            before | thick_frame
        } else {
            before & !thick_frame
        };
        if before == after {
            return Ok(false);
        }
        SetWindowLongPtrW(hwnd, GWL_STYLE, after);
        if GetWindowLongPtrW(hwnd, GWL_STYLE) != after {
            return Err("taskbar-resize-style-not-observed".into());
        }
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(true)
}

pub fn physical_cursor_position() -> Result<(i32, i32), String> {
    let mut point = POINT::default();
    // SAFETY: GetCursorPos writes one initialized POINT and mutates no window.
    if unsafe { GetCursorPos(&mut point) }.is_ok() {
        Ok((point.x, point.y))
    } else {
        Err("taskbar-cursor-unavailable".into())
    }
}

pub fn move_owned_taskbar_client(
    hwnd_identity: isize,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> Result<bool, String> {
    if hwnd_identity == 0 || width <= 0 || height <= 0 {
        return Err("taskbar-endpoint-invalid".into());
    }
    let hwnd = HWND(hwnd_identity as *mut c_void);
    let mut owner_pid = 0;
    let mut client = RECT::default();
    let mut origin = POINT::default();
    // SAFETY: queries use local output storage and occur before any mutation.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("taskbar-endpoint-retired".into());
        }
        if GetWindowThreadProcessId(hwnd, Some(&mut owner_pid)) == 0
            || owner_pid != GetCurrentProcessId()
        {
            return Err("taskbar-endpoint-foreign".into());
        }
        GetClientRect(hwnd, &mut client).map_err(|error| error.to_string())?;
        if !ClientToScreen(hwnd, &mut origin).as_bool() {
            return Err("taskbar-endpoint-origin-unavailable".into());
        }
    }
    if origin.x == left
        && origin.y == top
        && client.right - client.left == width
        && client.bottom - client.top == height
    {
        return Ok(false);
    }
    configure_and_show_taskbar_window(hwnd_identity, left, top, width, height)?;
    Ok(true)
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

pub fn cursor_root_window() -> Result<isize, String> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.map_err(|error| error.to_string())?;
    let window = unsafe { WindowFromPoint(point) };
    if window.0.is_null() {
        return Ok(0);
    }
    let root = unsafe { GetAncestor(window, GA_ROOT) };
    Ok(if root.0.is_null() {
        window.0 as isize
    } else {
        root.0 as isize
    })
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
    Maximize,
    Restore,
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

        for _ in 0..3 {
            let mut final_window = RECT::default();
            let mut final_client = RECT::default();
            let mut final_origin = POINT::default();
            GetWindowRect(hwnd, &mut final_window).map_err(|error| error.to_string())?;
            GetClientRect(hwnd, &mut final_client).map_err(|error| error.to_string())?;
            if !ClientToScreen(hwnd, &mut final_origin).as_bool() {
                return Err("taskbar-final-client-origin-unavailable".into());
            }
            let final_client_width = final_client.right - final_client.left;
            let final_client_height = final_client.bottom - final_client.top;
            let delta_x = left - final_origin.x;
            let delta_y = top - final_origin.y;
            let delta_width = width - final_client_width;
            let delta_height = height - final_client_height;
            if delta_x == 0 && delta_y == 0 && delta_width == 0 && delta_height == 0 {
                break;
            }
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                final_window.left + delta_x,
                final_window.top + delta_y,
                final_window.right - final_window.left + delta_width,
                final_window.bottom - final_window.top + delta_height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
            .map_err(|error| error.to_string())?;
        }
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

/// Promotes a live popup owned by this process into the topmost band without
/// moving, resizing, showing, or activating it.
pub fn promote_owned_popup_topmost(hwnd_identity: isize) -> Result<(), String> {
    if hwnd_identity == 0 {
        return Err("popup-window-invalid".into());
    }
    let hwnd = HWND(hwnd_identity as *mut c_void);
    let mut owner_pid = 0;
    // SAFETY: the z-order mutation is admitted only for a currently live HWND
    // owned by this process. GPUI retains ownership and destruction authority.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("popup-window-retired".into());
        }
        GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
        if owner_pid != GetCurrentProcessId() {
            return Err("popup-window-foreign".into());
        }
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
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
        WindowAction::Maximize => {
            let _ = unsafe { ShowWindow(hwnd, SW_MAXIMIZE) };
            Ok(())
        }
        WindowAction::Activate => unsafe { SetForegroundWindow(hwnd) }
            .ok()
            .map_err(|e| e.to_string()),
        WindowAction::Restore => {
            let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
            Ok(())
        }
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

pub fn apply_window_action_to_owned_identity(
    hwnd_identity: isize,
    process_id: u32,
    window_identity: &str,
    action: WindowAction,
) -> Result<(), String> {
    let hwnd = HWND(hwnd_identity as *mut c_void);
    let observed = snapshot_one(hwnd).ok_or("task-window-retired")?;
    if observed.process_id != process_id || observed.window_identity != window_identity {
        return Err("task-window-identity-mismatch".into());
    }
    apply_window_action(hwnd_identity, action)
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
    fn owned_popup_topmost_promotion_is_nonactivating_and_fails_closed() {
        use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};

        assert!(promote_owned_popup_topmost(0).is_err());
        assert!(promote_owned_popup_topmost(1).is_err());

        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                w!("SuperDesktop popup topmost test"),
                WS_POPUP,
                -20_000,
                -20_000,
                32,
                32,
                None,
                None,
                None,
                None,
            )
        }
        .expect("owned popup test hwnd");
        let foreground_before = unsafe { GetForegroundWindow() };
        promote_owned_popup_topmost(hwnd.0 as isize).expect("promote owned popup");
        let foreground_after = unsafe { GetForegroundWindow() };
        assert_eq!(foreground_after, foreground_before);
        unsafe { DestroyWindow(hwnd).expect("destroy owned popup") };
        assert!(promote_owned_popup_topmost(hwnd.0 as isize).is_err());

        let production = include_str!("taskbar.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for required in [
            "pub fn promote_owned_popup_topmost",
            "owner_pid != GetCurrentProcessId()",
            "Some(HWND_TOPMOST)",
            "SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
    }
    #[test]
    fn retired_exact_identity_action_fails_closed() {
        assert!(
            apply_window_action_to_owned_identity(1, 99, "win:99:1", WindowAction::Restore)
                .is_err()
        )
    }
    #[test]
    fn taskbar_resize_style_rejects_invalid_and_foreign_windows() {
        assert!(set_owned_taskbar_resizable(0, true).is_err());
        assert!(set_owned_taskbar_resizable(1, true).is_err());
        assert!(set_owned_taskbar_resizable(1, false).is_err());
        let foreground = unsafe { GetForegroundWindow() };
        if !foreground.0.is_null() {
            let mut process_id = 0;
            unsafe { GetWindowThreadProcessId(foreground, Some(&mut process_id)) };
            if process_id != 0 && process_id != unsafe { GetCurrentProcessId() } {
                assert!(set_owned_taskbar_resizable(foreground.0 as isize, true).is_err());
            }
        }
        let production = include_str!("taskbar.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for required in [
            "owner_pid != GetCurrentProcessId()",
            "WS_THICKFRAME",
            "SWP_FRAMECHANGED",
            "GetWindowLongPtrW(hwnd, GWL_STYLE)",
            "SetWindowSubclass",
            "message == WM_NCHITTEST && ref_data == 1",
            "RemoveWindowSubclass",
            "WM_ENTERSIZEMOVE",
            "WM_EXITSIZEMOVE | WM_NCDESTROY",
            "catch_unwind(AssertUnwindSafe",
        ] {
            assert!(production.contains(required));
        }
        assert!(!production.contains("Shell_TrayWnd"));
    }

    #[test]
    fn taskbar_resize_quantizes_to_exact_physical_row_heights() {
        assert_eq!(quantized_taskbar_outer_height(1, 96), (1, 40));
        assert_eq!(quantized_taskbar_outer_height(59, 96), (1, 40));
        assert_eq!(quantized_taskbar_outer_height(60, 96), (2, 80));
        assert_eq!(quantized_taskbar_outer_height(100, 96), (3, 120));
        assert_eq!(quantized_taskbar_outer_height(90, 144), (2, 120));
        assert_eq!(quantized_taskbar_outer_height(10_000, 192), (3, 240));
        let production = include_str!("taskbar.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap_or("");
        for required in ["WM_SIZING", "WMSZ_TOP", "GetDpiForWindow", "proposed.top ="] {
            assert!(
                production.contains(required),
                "missing resize quantization: {required}"
            );
        }
    }
    #[test]
    fn taskbar_auto_hide_adapters_reject_invalid_and_foreign_windows() {
        assert!(move_owned_taskbar_client(0, 0, 0, 100, 40).is_err());
        assert!(move_owned_taskbar_client(1, 0, 0, 100, 40).is_err());
        assert!(move_owned_taskbar_client(0, 0, 0, 0, 40).is_err());
        let foreground = unsafe { GetForegroundWindow() };
        if !foreground.0.is_null() {
            let mut process_id = 0;
            unsafe { GetWindowThreadProcessId(foreground, Some(&mut process_id)) };
            if process_id != 0 && process_id != unsafe { GetCurrentProcessId() } {
                assert!(move_owned_taskbar_client(foreground.0 as isize, 0, 0, 100, 40).is_err());
            }
        }
        let production = include_str!("taskbar.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for token in [
            "GetCursorPos",
            "owner_pid != GetCurrentProcessId()",
            "taskbar-endpoint-foreign",
            "configure_and_show_taskbar_window(hwnd_identity, left, top, width, height)",
            "taskbar-reveal-message-register",
            "if message == 0",
            "PostMessageW",
        ] {
            assert!(production.contains(token));
        }
        assert!(!production.contains("Shell_TrayWnd"));
    }
    #[test]
    fn owned_taskbar_endpoint_move_is_exact_idempotent_and_rejects_retirement() {
        use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};
        use windows::core::w;

        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                w!("SuperDesktop owned auto-hide test"),
                WS_POPUP,
                -20_000,
                -20_000,
                32,
                32,
                None,
                None,
                None,
                None,
            )
        }
        .expect("owned test hwnd");
        assert!(move_owned_taskbar_client(hwnd.0 as isize, -1800, 900, 640, 70).unwrap());
        assert!(!move_owned_taskbar_client(hwnd.0 as isize, -1800, 900, 640, 70).unwrap());
        let mut client = RECT::default();
        let mut origin = POINT::default();
        unsafe {
            GetClientRect(hwnd, &mut client).unwrap();
            assert!(ClientToScreen(hwnd, &mut origin).as_bool());
        }
        assert_eq!((origin.x, origin.y), (-1800, 900));
        assert_eq!(
            (client.right - client.left, client.bottom - client.top),
            (640, 70)
        );
        unsafe { DestroyWindow(hwnd).unwrap() };
        assert!(move_owned_taskbar_client(hwnd.0 as isize, 0, 0, 640, 70).is_err());
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
