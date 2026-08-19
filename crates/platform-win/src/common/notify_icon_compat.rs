//! Bounded, owned ingress for documented `NOTIFYICONDATAW` fields.

// SAFETY: Every Win32 pointer is either an owned allocation installed in this module's window
// userdata slot, a callback payload copied completely before the callback returns, or a handle
// validated for the current process/session before use. Teardown clears userdata before freeing
// it, the worker thread owns its HWND, and no borrowed shell memory escapes this module.

use std::{
    collections::VecDeque,
    io::Write,
    mem::{offset_of, size_of},
    ptr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

use shell_provider_protocol::{
    NotificationEventKind, NotificationSeverity, NotifyIconCallbackRoute, NotifyIconClientIdentity,
    NotifyIconIdentity, NotifyIconLayoutVersion, OwnedNotificationContent, OwnedNotifyIcon,
    Validate,
};
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    System::DataExchange::COPYDATASTRUCT,
    UI::{
        Shell::{
            NIF_GUID, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_REALTIME, NIF_SHOWTIP, NIF_STATE,
            NIF_TIP, NOTIFYICONDATAW,
        },
        WindowsAndMessaging::{
            CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, FindWindowW, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW,
            GetWindowThreadProcessId, HWND_BROADCAST, IsWindow, MSG, PostMessageW, PostQuitMessage,
            RegisterClassW, RegisterWindowMessageW, SetWindowLongPtrW, TranslateMessage,
            UnregisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_CONTEXTMENU, WM_COPYDATA,
            WM_DESTROY, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE, WNDCLASSW,
        },
    },
};
use windows::core::PCWSTR;

const SUPPORTED_FLAGS: u32 = NIF_MESSAGE.0
    | NIF_ICON.0
    | NIF_TIP.0
    | NIF_STATE.0
    | NIF_INFO.0
    | NIF_GUID.0
    | NIF_REALTIME.0
    | NIF_SHOWTIP.0;
const ICON_EDGE: u32 = 32;
const COPYDATA_SIGNATURE: usize = 1;
const MAX_COPYDATA_BYTES: usize = 2_048;
const PACKED_V1_SIZE: u32 = 152;
const PACKED_V2_SIZE: u32 = 936;
const PACKED_V3_SIZE: u32 = 952;
const PACKED_V4_SIZE: u32 = 956;
pub const MAX_NOTIFY_ICON_INGRESS: usize = 512;
const COMPATIBILITY_CLASS: &[u16] = &[
    b'S' as u16,
    b'h' as u16,
    b'e' as u16,
    b'l' as u16,
    b'l' as u16,
    b'_' as u16,
    b'T' as u16,
    b'r' as u16,
    b'a' as u16,
    b'y' as u16,
    b'W' as u16,
    b'n' as u16,
    b'd' as u16,
    0,
];
const TASKBAR_CREATED: &[u16] = &[
    b'T' as u16,
    b'a' as u16,
    b's' as u16,
    b'k' as u16,
    b'b' as u16,
    b'a' as u16,
    b'r' as u16,
    b'C' as u16,
    b'r' as u16,
    b'e' as u16,
    b'a' as u16,
    b't' as u16,
    b'e' as u16,
    b'd' as u16,
    0,
];
const NIN_SELECT: u32 = 0x0400;
const NIN_KEYSELECT: u32 = 0x0401;

#[derive(Clone, Debug)]
pub struct NotifyIconIngress {
    pub message: u32,
    pub input: NotifyIconCopyInput,
}

struct WindowContext {
    queue: Arc<Mutex<NotifyIconIngressQueue>>,
    accepting: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
pub struct NotifyIconIngressQueue {
    events: VecDeque<NotifyIconIngress>,
    overflowed: bool,
}

impl NotifyIconIngressQueue {
    pub fn push(&mut self, event: NotifyIconIngress) -> bool {
        if event.message == 1
            && let Some(existing) = self.events.iter_mut().rev().find(|existing| {
                existing.message == 1
                    && existing.input.process_id == event.input.process_id
                    && existing.input.window_identity == event.input.window_identity
                    && existing.input.numeric_id == event.input.numeric_id
                    && existing.input.guid == event.input.guid
            })
        {
            *existing = event;
            return true;
        }
        if self.events.len() >= MAX_NOTIFY_ICON_INGRESS {
            self.overflowed = true;
            if event.message >= 2
                && let Some(index) = self
                    .events
                    .iter()
                    .position(|existing| existing.message == 1)
            {
                self.events.remove(index);
            } else {
                return false;
            }
        }
        self.events.push_back(event);
        true
    }

    pub fn pop(&mut self) -> Option<NotifyIconIngress> {
        self.events.pop_front()
    }
    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    pub fn overflowed(&self) -> bool {
        self.overflowed
    }
}

pub struct NotifyIconCompatibilityWindow {
    hwnd: isize,
    accepting: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl NotifyIconCompatibilityWindow {
    pub fn start() -> Result<(Self, Arc<Mutex<NotifyIconIngressQueue>>), &'static str> {
        if unsafe { FindWindowW(PCWSTR(COMPATIBILITY_CLASS.as_ptr()), PCWSTR::null()) }
            .is_ok_and(|window| !window.0.is_null())
        {
            return Err("notify-icon-shell-tray-window-exists");
        }
        let queue = Arc::new(Mutex::new(NotifyIconIngressQueue::default()));
        let thread_queue = Arc::clone(&queue);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let accepting = Arc::new(AtomicBool::new(true));
        let thread_accepting = Arc::clone(&accepting);
        let thread = thread::spawn(move || {
            run_compatibility_window(thread_queue, thread_accepting, ready_sender);
        });
        let hwnd = ready_receiver
            .recv()
            .map_err(|_| "notify-icon-window-thread")??;
        Ok((
            Self {
                hwnd,
                accepting,
                thread: Some(thread),
            },
            queue,
        ))
    }

    pub fn hwnd_identity(&self) -> isize {
        self.hwnd
    }

    pub fn teardown(&mut self) {
        if !self.accepting.swap(false, Ordering::AcqRel) {
            return;
        }
        if self.hwnd != 0 {
            let _ = unsafe {
                PostMessageW(
                    Some(HWND(self.hwnd as *mut _)),
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                )
            };
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.hwnd = 0;
    }
}

impl Drop for NotifyIconCompatibilityWindow {
    fn drop(&mut self) {
        self.teardown();
    }
}

fn run_compatibility_window(
    queue: Arc<Mutex<NotifyIconIngressQueue>>,
    accepting: Arc<AtomicBool>,
    ready: mpsc::SyncSender<Result<isize, &'static str>>,
) {
    let instance = HINSTANCE::default();
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(notify_icon_window_proc),
        hInstance: instance,
        lpszClassName: PCWSTR(COMPATIBILITY_CLASS.as_ptr()),
        ..WNDCLASSW::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        let _ = ready.send(Err("notify-icon-register-class"));
        return;
    }
    let context = Box::new(WindowContext { queue, accepting });
    let context_ptr = Box::into_raw(context);
    let hwnd = match unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(COMPATIBILITY_CLASS.as_ptr()),
            PCWSTR::null(),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            Some(context_ptr.cast()),
        )
    } {
        Ok(hwnd) => hwnd,
        Err(_) => {
            unsafe {
                drop(Box::from_raw(context_ptr));
            }
            let _ =
                unsafe { UnregisterClassW(PCWSTR(COMPATIBILITY_CLASS.as_ptr()), Some(instance)) };
            let _ = ready.send(Err("notify-icon-create-window"));
            return;
        }
    };
    let raw = hwnd.0 as isize;
    let _ = ready.send(Ok(raw));
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    let _ = unsafe { UnregisterClassW(PCWSTR(COMPATIBILITY_CLASS.as_ptr()), Some(instance)) };
}

unsafe extern "system" fn notify_icon_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    no_unwind(|| {
        if message == WM_NCCREATE {
            let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            }
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
        let context_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowContext;
        match message {
            WM_COPYDATA if !context_ptr.is_null() => {
                let context = unsafe { &*context_ptr };
                if !context.accepting.load(Ordering::Acquire) {
                    return LRESULT(0);
                }
                let copy = unsafe { &*(lparam.0 as *const COPYDATASTRUCT) };
                trace_copydata(copy);
                if let Ok(event) = unsafe { decode_copydata(copy, wparam.0 as isize) }
                    && context
                        .queue
                        .lock()
                        .is_ok_and(|mut queue| queue.push(event))
                {
                    LRESULT(1)
                } else {
                    LRESULT(0)
                }
            }
            WM_CLOSE => {
                let _ = unsafe { DestroyWindow(hwnd) };
                LRESULT(0)
            }
            WM_DESTROY => {
                if !context_ptr.is_null() {
                    unsafe {
                        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                        drop(Box::from_raw(context_ptr));
                    }
                }
                unsafe {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    })
}

fn trace_copydata(copy: &COPYDATASTRUCT) {
    let Some(path) = std::env::var_os("SUPERDESKTOP_NOTIFYICON_TRACE") else {
        return;
    };
    let length = (copy.cbData as usize).min(MAX_COPYDATA_BYTES).min(64);
    let bytes = if copy.lpData.is_null() {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(copy.lpData.cast::<u8>(), length) }
    };
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(
            file,
            "dwData={} cbData={} bytes={hex}",
            copy.dwData, copy.cbData
        );
    }
}

fn no_unwind(callback: impl FnOnce() -> LRESULT) -> LRESULT {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback)).unwrap_or(LRESULT(0))
}

unsafe fn decode_copydata(
    copy: &COPYDATASTRUCT,
    sender_window: isize,
) -> Result<NotifyIconIngress, &'static str> {
    let length = copy.cbData as usize;
    if copy.lpData.is_null()
        || length < 8 + NotifyIconLayoutMatrix::current().v1_size as usize
        || length > MAX_COPYDATA_BYTES
    {
        return Err("notify-icon-copydata-size");
    }
    let bytes = unsafe { std::slice::from_raw_parts(copy.lpData.cast::<u8>(), length) };
    let embedded_message = u32::from_ne_bytes(bytes[4..8].try_into().unwrap());
    let message = if copy.dwData == COPYDATA_SIGNATURE {
        embedded_message
    } else {
        copy.dwData as u32
    };
    if message > 4 {
        return Err("notify-icon-copydata-message");
    }
    let native_offset = if copy.dwData == COPYDATA_SIGNATURE {
        8
    } else {
        0
    };
    let packed_size = (native_offset == 8)
        .then(|| u32::from_ne_bytes(bytes[8..12].try_into().unwrap()))
        .filter(|size| {
            matches!(
                *size,
                PACKED_V1_SIZE | PACKED_V2_SIZE | PACKED_V3_SIZE | PACKED_V4_SIZE
            )
        });
    if let Some(cb_size) = packed_size {
        let read_u32 = |offset: usize| {
            bytes
                .get(offset..offset + 4)
                .and_then(|value| value.try_into().ok())
                .map(u32::from_ne_bytes)
                .ok_or("notify-icon-copydata-truncated")
        };
        let owner_window = read_u32(12)? as isize;
        let numeric_id = read_u32(16)?;
        let flags = read_u32(20)?;
        let callback_message = read_u32(24)?;
        let borrowed_hicon = read_u32(28)? as isize;
        let tip_units = if cb_size == PACKED_V1_SIZE { 64 } else { 128 };
        let mut tooltip_utf16 = Vec::with_capacity(tip_units);
        for offset in (32..32 + tip_units * 2).step_by(2) {
            tooltip_utf16.push(u16::from_ne_bytes(
                bytes
                    .get(offset..offset + 2)
                    .ok_or("notify-icon-copydata-truncated")?
                    .try_into()
                    .unwrap(),
            ));
        }
        let state = if cb_size >= PACKED_V2_SIZE {
            read_u32(288)?
        } else {
            0
        };
        let read_utf16 = |start: usize, units: usize| -> Result<Vec<u16>, &'static str> {
            (start..start + units * 2)
                .step_by(2)
                .map(|offset| {
                    bytes
                        .get(offset..offset + 2)
                        .ok_or("notify-icon-copydata-truncated")
                        .map(|value| u16::from_ne_bytes(value.try_into().unwrap()))
                })
                .collect()
        };
        let (info_utf16, info_title_utf16, info_flags, info_timeout_ms) =
            if flags & NIF_INFO.0 != 0 && cb_size >= PACKED_V2_SIZE {
                (
                    read_utf16(296, 256)?,
                    read_utf16(812, 64)?,
                    read_u32(940)?,
                    read_u32(808)?,
                )
            } else {
                (Vec::new(), Vec::new(), 0, 0)
            };
        let guid = if flags & NIF_GUID.0 != 0 && cb_size >= PACKED_V3_SIZE {
            Some(
                bytes
                    .get(944..960)
                    .ok_or("notify-icon-copydata-truncated")?
                    .try_into()
                    .unwrap(),
            )
        } else {
            None
        };
        let requested_version = (message == 4).then_some(match read_u32(808)? {
            4 => NotifyIconLayoutVersion::V4,
            3 => NotifyIconLayoutVersion::V3,
            2 => NotifyIconLayoutVersion::V2,
            _ => NotifyIconLayoutVersion::V1,
        });
        let (process_id, session_id) = owner_process_session(owner_window);
        return Ok(NotifyIconIngress {
            message,
            input: NotifyIconCopyInput {
                cb_size,
                flags,
                process_id,
                session_id,
                window_identity: owner_window,
                numeric_id,
                guid,
                callback_message,
                requested_version,
                tooltip_utf16,
                info_utf16,
                info_title_utf16,
                info_flags,
                info_timeout_ms,
                realtime: flags & NIF_REALTIME.0 != 0,
                visible: state & 1 == 0,
                borrowed_hicon,
            },
        });
    }
    let mut native = NOTIFYICONDATAW::default();
    let available = length
        .saturating_sub(native_offset)
        .min(size_of::<NOTIFYICONDATAW>());
    unsafe {
        ptr::copy_nonoverlapping(
            bytes.as_ptr().add(native_offset),
            (&raw mut native).cast::<u8>(),
            available,
        );
    }
    let owner_window = if native.hWnd.0.is_null() {
        sender_window
    } else {
        native.hWnd.0 as isize
    };
    let (process_id, session_id) = owner_process_session(owner_window);
    let guid = (native.uFlags.0 & NIF_GUID.0 != 0).then(|| native.guidItem.to_u128().to_ne_bytes());
    let requested_version = (message == 4).then_some(match unsafe { native.Anonymous.uVersion } {
        4 => NotifyIconLayoutVersion::V4,
        3 => NotifyIconLayoutVersion::V3,
        2 => NotifyIconLayoutVersion::V2,
        _ => NotifyIconLayoutVersion::V1,
    });
    Ok(NotifyIconIngress {
        message,
        input: NotifyIconCopyInput {
            cb_size: native.cbSize,
            flags: native.uFlags.0,
            process_id,
            session_id,
            window_identity: owner_window,
            numeric_id: native.uID,
            guid,
            callback_message: native.uCallbackMessage,
            requested_version,
            tooltip_utf16: native.szTip.to_vec(),
            info_utf16: if native.uFlags.0 & NIF_INFO.0 != 0 {
                native.szInfo.to_vec()
            } else {
                Vec::new()
            },
            info_title_utf16: if native.uFlags.0 & NIF_INFO.0 != 0 {
                native.szInfoTitle.to_vec()
            } else {
                Vec::new()
            },
            info_flags: if native.uFlags.0 & NIF_INFO.0 != 0 {
                native.dwInfoFlags.0
            } else {
                0
            },
            info_timeout_ms: if native.uFlags.0 & NIF_INFO.0 != 0 {
                unsafe { native.Anonymous.uTimeout }
            } else {
                0
            },
            realtime: native.uFlags.0 & NIF_REALTIME.0 != 0,
            visible: native.dwState.0 & 1 == 0,
            borrowed_hicon: native.hIcon.0 as isize,
        },
    })
}

fn owner_process_session(owner_window: isize) -> (u32, u32) {
    let mut process_id = 0u32;
    if owner_window != 0 {
        unsafe {
            GetWindowThreadProcessId(HWND(owner_window as *mut _), Some(&mut process_id));
        }
    }
    let mut session_id = 0u32;
    if process_id != 0 {
        unsafe {
            ProcessIdToSessionId(process_id, &mut session_id);
        }
    }
    (process_id, session_id)
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifyIconLayoutMatrix {
    pub v1_size: u32,
    pub v2_size: u32,
    pub v3_size: u32,
    pub v4_size: u32,
}

impl NotifyIconLayoutMatrix {
    pub fn current() -> Self {
        Self {
            v1_size: (offset_of!(NOTIFYICONDATAW, szTip) + 64 * size_of::<u16>()) as u32,
            v2_size: offset_of!(NOTIFYICONDATAW, guidItem) as u32,
            v3_size: offset_of!(NOTIFYICONDATAW, hBalloonIcon) as u32,
            v4_size: size_of::<NOTIFYICONDATAW>() as u32,
        }
    }

    pub fn version_for_size(self, size: u32) -> Option<NotifyIconLayoutVersion> {
        match size {
            value if value == self.v1_size || value == PACKED_V1_SIZE => {
                Some(NotifyIconLayoutVersion::V1)
            }
            value if value == self.v2_size || value == PACKED_V2_SIZE => {
                Some(NotifyIconLayoutVersion::V2)
            }
            value if value == self.v3_size || value == PACKED_V3_SIZE => {
                Some(NotifyIconLayoutVersion::V3)
            }
            value if value == self.v4_size || value == PACKED_V4_SIZE => {
                Some(NotifyIconLayoutVersion::V4)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotifyIconCopyInput {
    pub cb_size: u32,
    pub flags: u32,
    pub process_id: u32,
    pub session_id: u32,
    pub window_identity: isize,
    pub numeric_id: u32,
    pub guid: Option<[u8; 16]>,
    pub callback_message: u32,
    pub requested_version: Option<NotifyIconLayoutVersion>,
    pub tooltip_utf16: Vec<u16>,
    pub info_utf16: Vec<u16>,
    pub info_title_utf16: Vec<u16>,
    pub info_flags: u32,
    pub info_timeout_ms: u32,
    pub realtime: bool,
    pub visible: bool,
    pub borrowed_hicon: isize,
}

pub fn copy_notify_icon(
    input: &NotifyIconCopyInput,
    generation: u64,
    validate_live_window: bool,
) -> Result<OwnedNotifyIcon, &'static str> {
    let layout = NotifyIconLayoutMatrix::current()
        .version_for_size(input.cb_size)
        .ok_or("notify-icon-unsupported-layout")?;
    if input.flags & !SUPPORTED_FLAGS != 0 {
        return Err("notify-icon-unsupported-flags");
    }
    if input.tooltip_utf16.len() > 128 {
        return Err("notify-icon-tooltip-capacity");
    }
    if input.info_utf16.len() > 256 || input.info_title_utf16.len() > 64 {
        return Err("notify-icon-info-capacity");
    }
    if input.tooltip_utf16.contains(&0) && input.tooltip_utf16.last().copied() != Some(0) {
        return Err("notify-icon-tooltip-interior-nul");
    }
    if validate_live_window {
        validate_window_owner(input.window_identity, input.process_id, input.session_id)?;
    }
    let tooltip_length = input
        .tooltip_utf16
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(input.tooltip_utf16.len());
    let tooltip = String::from_utf16(&input.tooltip_utf16[..tooltip_length])
        .map_err(|_| "notify-icon-tooltip-utf16")?;
    let decode_info = |units: &[u16]| -> Result<String, &'static str> {
        let length = units
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(units.len());
        String::from_utf16(&units[..length]).map_err(|_| "notify-icon-info-utf16")
    };
    let notification = if input.flags & NIF_INFO.0 != 0 {
        let body = decode_info(&input.info_utf16)?;
        let title = decode_info(&input.info_title_utf16)?;
        (!title.is_empty() || !body.is_empty()).then_some(OwnedNotificationContent {
            title,
            body,
            severity: match input.info_flags & 0xf {
                2 => NotificationSeverity::Warning,
                3 => NotificationSeverity::Error,
                4 => NotificationSeverity::User,
                _ => NotificationSeverity::Information,
            },
            realtime: input.realtime,
            timeout_ms: input.info_timeout_ms,
        })
    } else {
        None
    };
    let pixels = if input.flags & NIF_ICON.0 != 0 {
        Some(
            super::icon::borrowed_hicon_rgba(input.borrowed_hicon, ICON_EDGE)
                .ok_or("notify-icon-copy-failed")?,
        )
    } else {
        None
    };
    let icon = OwnedNotifyIcon {
        client: NotifyIconClientIdentity {
            process_id: input.process_id,
            session_id: input.session_id,
            window_identity: input.window_identity as i64,
        },
        identity: NotifyIconIdentity {
            numeric_id: input.numeric_id,
            guid: (input.flags & NIF_GUID.0 != 0)
                .then_some(input.guid)
                .flatten(),
        },
        callback: NotifyIconCallbackRoute {
            message_id: input.callback_message,
            negotiated_version: input.requested_version.unwrap_or(layout),
        },
        tooltip,
        visible: input.visible,
        pixels,
        notification,
        generation,
    };
    icon.validate().map_err(|_| "notify-icon-owned-invalid")?;
    Ok(icon)
}

pub fn validate_window_owner(
    window_identity: isize,
    expected_process_id: u32,
    expected_session_id: u32,
) -> Result<(), &'static str> {
    if window_identity == 0 || expected_process_id == 0 {
        return Err("notify-icon-owner-null");
    }
    let hwnd = HWND(window_identity as *mut core::ffi::c_void);
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return Err("notify-icon-owner-window-dead");
    }
    let mut process_id = 0u32;
    if unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) } == 0
        || process_id != expected_process_id
    {
        return Err("notify-icon-owner-process-mismatch");
    }
    let mut session_id = 0u32;
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0
        || session_id != expected_session_id
    {
        return Err("notify-icon-owner-session-mismatch");
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifyIconCallbackPayload {
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
}

pub fn callback_payload(
    icon: &OwnedNotifyIcon,
    kind: NotificationEventKind,
) -> NotifyIconCallbackPayload {
    let event = match (icon.callback.negotiated_version, kind) {
        (NotifyIconLayoutVersion::V4, NotificationEventKind::Activate) => NIN_SELECT,
        (NotifyIconLayoutVersion::V4, NotificationEventKind::Focus) => NIN_KEYSELECT,
        (_, NotificationEventKind::Activate) => WM_LBUTTONUP,
        (_, NotificationEventKind::Context) => WM_CONTEXTMENU,
        (_, NotificationEventKind::Hover) => WM_MOUSEMOVE,
        (_, NotificationEventKind::Focus) => NIN_KEYSELECT,
    };
    let numeric_id = icon.identity.numeric_id;
    if icon.callback.negotiated_version == NotifyIconLayoutVersion::V4 {
        NotifyIconCallbackPayload {
            message: icon.callback.message_id,
            wparam: 0,
            lparam: isize::try_from((event & 0xffff) | (numeric_id << 16)).unwrap_or_default(),
        }
    } else {
        NotifyIconCallbackPayload {
            message: icon.callback.message_id,
            wparam: numeric_id as usize,
            lparam: event as isize,
        }
    }
}

pub fn deliver_callback(
    icon: &OwnedNotifyIcon,
    kind: NotificationEventKind,
) -> Result<(), &'static str> {
    validate_window_owner(
        icon.client.window_identity as isize,
        icon.client.process_id,
        icon.client.session_id,
    )?;
    let payload = callback_payload(icon, kind);
    unsafe {
        PostMessageW(
            Some(HWND(icon.client.window_identity as isize as *mut _)),
            payload.message,
            WPARAM(payload.wparam),
            LPARAM(payload.lparam),
        )
    }
    .map_err(|_| "notify-icon-callback-post")
}

pub fn broadcast_taskbar_created() -> Result<u32, &'static str> {
    let message = taskbar_created_message()?;
    unsafe { PostMessageW(Some(HWND_BROADCAST), message, WPARAM(0), LPARAM(0)) }
        .map_err(|_| "notify-icon-taskbar-created-broadcast")?;
    Ok(message)
}

pub fn taskbar_created_message() -> Result<u32, &'static str> {
    let message = unsafe { RegisterWindowMessageW(PCWSTR(TASKBAR_CREATED.as_ptr())) };
    if message == 0 {
        return Err("notify-icon-taskbar-created-register");
    }
    Ok(message)
}

pub fn current_console_owner() -> Option<(u32, u32, isize)> {
    use windows::Win32::System::{Console::GetConsoleWindow, Threading::GetCurrentProcessId};
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    let process_id = unsafe { GetCurrentProcessId() };
    let mut session_id = 0u32;
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0 {
        return None;
    }
    Some((process_id, session_id, hwnd.0 as isize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{
        System::{Console::GetConsoleWindow, Threading::GetCurrentProcessId},
        UI::WindowsAndMessaging::{IDI_APPLICATION, LoadIconW},
    };

    fn fixture() -> NotifyIconCopyInput {
        NotifyIconCopyInput {
            cb_size: NotifyIconLayoutMatrix::current().v4_size,
            flags: NIF_MESSAGE.0 | NIF_TIP.0,
            process_id: 42,
            session_id: 1,
            window_identity: 100,
            numeric_id: 7,
            guid: None,
            callback_message: 0x500,
            requested_version: Some(NotifyIconLayoutVersion::V4),
            tooltip_utf16: "Owned tooltip".encode_utf16().collect(),
            info_utf16: Vec::new(),
            info_title_utf16: Vec::new(),
            info_flags: 0,
            info_timeout_ms: 0,
            realtime: false,
            visible: true,
            borrowed_hicon: 0,
        }
    }

    fn ingress(message: u32, id: u32) -> NotifyIconIngress {
        let mut input = fixture();
        input.numeric_id = id;
        NotifyIconIngress { message, input }
    }

    #[test]
    fn supported_layout_matrix_is_strict_and_monotonic() {
        let matrix = NotifyIconLayoutMatrix::current();
        assert!(matrix.v1_size < matrix.v2_size);
        assert!(matrix.v2_size < matrix.v3_size);
        assert!(matrix.v3_size < matrix.v4_size);
        for (size, version) in [
            (matrix.v1_size, NotifyIconLayoutVersion::V1),
            (matrix.v2_size, NotifyIconLayoutVersion::V2),
            (matrix.v3_size, NotifyIconLayoutVersion::V3),
            (matrix.v4_size, NotifyIconLayoutVersion::V4),
        ] {
            assert_eq!(matrix.version_for_size(size), Some(version));
        }
        assert_eq!(matrix.version_for_size(matrix.v4_size - 1), None);
    }

    #[test]
    fn owned_copy_preserves_identity_tooltip_callback_and_state() {
        let icon = copy_notify_icon(&fixture(), 9, false).unwrap();
        assert_eq!(icon.client.process_id, 42);
        assert_eq!(icon.identity.numeric_id, 7);
        assert_eq!(icon.tooltip, "Owned tooltip");
        assert!(icon.visible);
        assert_eq!(icon.generation, 9);
        assert!(icon.pixels.is_none());
    }

    #[test]
    fn malformed_layout_flags_tooltip_owner_and_icon_fail_closed() {
        let mut input = fixture();
        input.cb_size = 1;
        assert_eq!(
            copy_notify_icon(&input, 1, false),
            Err("notify-icon-unsupported-layout")
        );
        let mut input = fixture();
        input.flags |= 1 << 31;
        assert_eq!(
            copy_notify_icon(&input, 1, false),
            Err("notify-icon-unsupported-flags")
        );
        let mut input = fixture();
        input.tooltip_utf16 = vec![b'x' as u16; 129];
        assert_eq!(
            copy_notify_icon(&input, 1, false),
            Err("notify-icon-tooltip-capacity")
        );
        let mut input = fixture();
        input.process_id = 0;
        assert_eq!(
            copy_notify_icon(&input, 1, true),
            Err("notify-icon-owner-null")
        );
        let mut input = fixture();
        input.flags |= NIF_ICON.0;
        assert_eq!(
            copy_notify_icon(&input, 1, false),
            Err("notify-icon-copy-failed")
        );
        let mut input = fixture();
        input.flags |= NIF_INFO.0;
        input.info_utf16 = vec![0xd800];
        assert_eq!(
            copy_notify_icon(&input, 1, false),
            Err("notify-icon-info-utf16")
        );
        let mut input = fixture();
        input.flags |= NIF_INFO.0;
        input.info_utf16 = vec![b'x' as u16; 257];
        assert_eq!(
            copy_notify_icon(&input, 1, false),
            Err("notify-icon-info-capacity")
        );
    }

    #[test]
    fn live_owner_session_and_reuse_validation_fail_closed() {
        assert_eq!(
            validate_window_owner(1, 1, 1),
            Err("notify-icon-owner-window-dead")
        );
        let hwnd = unsafe { GetConsoleWindow() };
        if hwnd.0.is_null() {
            return;
        }
        let process_id = unsafe { GetCurrentProcessId() };
        let mut session_id = 0u32;
        assert_ne!(
            unsafe { ProcessIdToSessionId(process_id, &mut session_id) },
            0
        );
        validate_window_owner(hwnd.0 as isize, process_id, session_id).unwrap();
        assert_eq!(
            validate_window_owner(hwnd.0 as isize, process_id.wrapping_add(1), session_id),
            Err("notify-icon-owner-process-mismatch")
        );
        assert_eq!(
            validate_window_owner(hwnd.0 as isize, process_id, session_id.wrapping_add(1)),
            Err("notify-icon-owner-session-mismatch")
        );
    }

    #[test]
    fn borrowed_system_hicon_copies_pixels_without_transferring_ownership() {
        let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap();
        let first = super::super::icon::borrowed_hicon_rgba(icon.0 as isize, ICON_EDGE).unwrap();
        let second = super::super::icon::borrowed_hicon_rgba(icon.0 as isize, ICON_EDGE).unwrap();
        assert_eq!(first, second);
        assert!(super::super::icon::valid_icon_data(&first));
    }

    #[test]
    fn copydata_transport_copies_supported_payload_before_return() {
        let mut native = NOTIFYICONDATAW {
            cbSize: NotifyIconLayoutMatrix::current().v4_size,
            hWnd: HWND(100usize as *mut _),
            uID: 77,
            uFlags: windows::Win32::UI::Shell::NIF_MESSAGE | NIF_TIP | NIF_INFO | NIF_REALTIME,
            uCallbackMessage: 0x501,
            ..NOTIFYICONDATAW::default()
        };
        for (destination, source) in native.szTip.iter_mut().zip("Transport tip".encode_utf16()) {
            *destination = source;
        }
        for (destination, source) in native
            .szInfoTitle
            .iter_mut()
            .zip("Build warning".encode_utf16())
        {
            *destination = source;
        }
        for (destination, source) in native
            .szInfo
            .iter_mut()
            .zip("Attention required".encode_utf16())
        {
            *destination = source;
        }
        native.dwInfoFlags = windows::Win32::UI::Shell::NIIF_WARNING;
        native.Anonymous.uTimeout = 7_000;
        let mut payload = vec![0u8; 8 + size_of::<NOTIFYICONDATAW>()];
        payload[..4].copy_from_slice(&1u32.to_ne_bytes());
        payload[4..8].copy_from_slice(&0u32.to_ne_bytes());
        unsafe {
            ptr::copy_nonoverlapping(
                (&raw const native).cast::<u8>(),
                payload.as_mut_ptr().add(8),
                size_of::<NOTIFYICONDATAW>(),
            );
        }
        let copy = COPYDATASTRUCT {
            dwData: COPYDATA_SIGNATURE,
            cbData: payload.len() as u32,
            lpData: payload.as_mut_ptr().cast(),
        };
        let mut ingress = unsafe { decode_copydata(&copy, 100) }.unwrap();
        assert_eq!(ingress.message, 0);
        assert_eq!(ingress.input.numeric_id, 77);
        assert_eq!(ingress.input.callback_message, 0x501);
        assert_eq!(ingress.input.tooltip_utf16[0], b'T' as u16);
        assert_eq!(ingress.input.info_title_utf16[0], b'B' as u16);
        assert_eq!(ingress.input.info_utf16[0], b'A' as u16);
        assert_eq!(ingress.input.info_flags & 0xf, 2);
        assert_eq!(ingress.input.info_timeout_ms, 7_000);
        assert!(ingress.input.realtime);
        ingress.input.process_id = 42;
        ingress.input.session_id = 1;
        ingress.input.window_identity = 100;
        let icon = copy_notify_icon(&ingress.input, 9, false).unwrap();
        let notification = icon.notification.unwrap();
        assert_eq!(notification.title, "Build warning");
        assert_eq!(notification.body, "Attention required");
        assert_eq!(notification.severity, NotificationSeverity::Warning);
    }

    #[test]
    fn explorer_owned_shell_tray_identity_blocks_preview_collision() {
        let existing = unsafe { FindWindowW(PCWSTR(COMPATIBILITY_CLASS.as_ptr()), PCWSTR::null()) };
        if existing.is_ok_and(|window| !window.0.is_null()) {
            assert!(matches!(
                NotifyIconCompatibilityWindow::start(),
                Err("notify-icon-shell-tray-window-exists")
            ));
        }
    }

    #[test]
    fn ingress_queue_coalesces_modifies_and_preserves_protected_delete() {
        let mut queue = NotifyIconIngressQueue::default();
        assert!(queue.push(ingress(1, 1)));
        assert!(queue.push(ingress(1, 1)));
        assert_eq!(queue.len(), 1);
        for id in 2..=MAX_NOTIFY_ICON_INGRESS as u32 {
            assert!(queue.push(ingress(1, id)));
        }
        assert_eq!(queue.len(), MAX_NOTIFY_ICON_INGRESS);
        assert!(queue.push(ingress(2, 999)));
        assert!(queue.overflowed());
        assert_eq!(queue.len(), MAX_NOTIFY_ICON_INGRESS);
        assert!(std::iter::from_fn(|| queue.pop()).any(|event| event.message == 2));
    }

    #[test]
    fn callback_panic_and_repeated_teardown_are_fenced() {
        assert_eq!(no_unwind(|| panic!("fixture callback panic")), LRESULT(0));
        let mut window = NotifyIconCompatibilityWindow {
            hwnd: 0,
            accepting: Arc::new(AtomicBool::new(true)),
            thread: None,
        };
        window.teardown();
        window.teardown();
        assert_eq!(window.hwnd_identity(), 0);
    }

    #[test]
    fn callback_payload_negotiates_v4_and_legacy_without_borrowed_state() {
        let mut input = fixture();
        input.numeric_id = 77;
        let mut icon = copy_notify_icon(&input, 1, false).unwrap();
        let v4 = callback_payload(&icon, NotificationEventKind::Activate);
        assert_eq!(v4.message, input.callback_message);
        assert_eq!(v4.wparam, 0);
        assert_eq!(v4.lparam as u32 & 0xffff, NIN_SELECT);
        assert_eq!(v4.lparam as u32 >> 16, 77);
        icon.callback.negotiated_version = NotifyIconLayoutVersion::V2;
        let legacy = callback_payload(&icon, NotificationEventKind::Context);
        assert_eq!(legacy.wparam, 77);
        assert_eq!(legacy.lparam as u32, WM_CONTEXTMENU);
        assert_eq!(
            deliver_callback(&icon, NotificationEventKind::Activate),
            Err("notify-icon-owner-window-dead")
        );
        assert!(taskbar_created_message().is_ok_and(|message| message >= 0xc000));
    }
}
