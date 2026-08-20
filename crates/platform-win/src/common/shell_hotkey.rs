//! Shell-owned Windows-key routing with bounded thread and hook lifetime.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, RPC_E_CHANGED_MODE, WPARAM},
    System::{
        Com::{
            CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        },
        Threading::GetCurrentThreadId,
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        },
        Shell::{AO_NONE, ApplicationActivationManager, IApplicationActivationManager},
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE,
            PeekMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
            WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};
use windows::core::{Error as WindowsError, w};

const VK_A: u32 = 0x41;
const VK_D: u32 = 0x44;
const VK_E: u32 = 0x45;
const VK_N: u32 = 0x4e;
const VK_S: u32 = 0x53;
const VK_SPACE: u32 = 0x20;
const VK_TAB: u32 = 0x09;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ShellHotkeyAction {
    OpenExplorer = 1 << 0,
    ShowDesktop = 1 << 1,
    CycleInput = 1 << 2,
    OpenSearch = 1 << 3,
    OpenTaskView = 1 << 4,
    OpenNetworkPower = 1 << 5,
    OpenNotifications = 1 << 6,
    CycleInputPrevious = 1 << 7,
    AltTabForward = 1 << 8,
    AltTabBackward = 1 << 9,
    AltTabCommit = 1 << 10,
    AltTabCancel = 1 << 11,
    OpenScreenSnip = 1 << 12,
}

impl ShellHotkeyAction {
    fn from_code(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::OpenExplorer),
            2 => Some(Self::ShowDesktop),
            4 => Some(Self::CycleInput),
            8 => Some(Self::OpenSearch),
            16 => Some(Self::OpenTaskView),
            32 => Some(Self::OpenNetworkPower),
            64 => Some(Self::OpenNotifications),
            128 => Some(Self::CycleInputPrevious),
            256 => Some(Self::AltTabForward),
            512 => Some(Self::AltTabBackward),
            1024 => Some(Self::AltTabCommit),
            2048 => Some(Self::AltTabCancel),
            4096 => Some(Self::OpenScreenSnip),
            _ => None,
        }
    }
}

static ACTIVE_KEY: AtomicU32 = AtomicU32::new(0);
static REQUESTED: AtomicU32 = AtomicU32::new(0);
static ALT_TAB_ACTIVE: AtomicBool = AtomicBool::new(false);
static ALT_TAB_DELTA: AtomicI32 = AtomicI32::new(0);

fn request(action: ShellHotkeyAction) {
    match action {
        ShellHotkeyAction::AltTabForward => {
            ALT_TAB_DELTA.fetch_add(1, Ordering::AcqRel);
        }
        ShellHotkeyAction::AltTabBackward => {
            ALT_TAB_DELTA.fetch_sub(1, Ordering::AcqRel);
        }
        _ => {
            REQUESTED.fetch_or(action as u32, Ordering::AcqRel);
        }
    }
}

fn action_for_key(
    vk_code: u32,
    control: bool,
    alt: bool,
    shift: bool,
) -> Option<ShellHotkeyAction> {
    if control || alt {
        return None;
    }
    match vk_code {
        VK_E if !shift => Some(ShellHotkeyAction::OpenExplorer),
        VK_D if !shift => Some(ShellHotkeyAction::ShowDesktop),
        VK_SPACE if shift => Some(ShellHotkeyAction::CycleInputPrevious),
        VK_SPACE => Some(ShellHotkeyAction::CycleInput),
        VK_S if shift => Some(ShellHotkeyAction::OpenScreenSnip),
        VK_S => Some(ShellHotkeyAction::OpenSearch),
        VK_TAB if !shift => Some(ShellHotkeyAction::OpenTaskView),
        VK_A if !shift => Some(ShellHotkeyAction::OpenNetworkPower),
        VK_N if !shift => Some(ShellHotkeyAction::OpenNotifications),
        _ => None,
    }
}

/// Opens the Windows-registered built-in image-snipping overlay.
pub fn open_screen_snipping_overlay() -> Result<(), String> {
    let initialize = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let initialized = match initialize {
        result if result.is_ok() => true,
        RPC_E_CHANGED_MODE => false,
        error => {
            return Err(format!(
                "screen-snipping COM initialization failed: {}",
                WindowsError::from_hresult(error)
            ));
        }
    };
    let result = (|| {
        // SAFETY: the AUMID and launch arguments are compile-time fixed, and
        // the local-server activation manager owns the activation arguments
        // for the returned packaged-app process.
        let activation: IApplicationActivationManager =
            unsafe { CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER) }
                .map_err(|error| format!("screen-snipping activation manager failed: {error}"))?;
        let process_id = unsafe {
            activation.ActivateApplication(
                w!("Microsoft.ScreenSketch_8wekyb3d8bbwe!App"),
                w!("ms-screenclip:///?source=HotKey"),
                AO_NONE,
            )
        }
        .map_err(|error| format!("screen-snipping protocol activation failed: {error}"))?;
        (process_id != 0)
            .then_some(())
            .ok_or_else(|| "screen-snipping activation returned no process".to_owned())
    })();
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

fn reduce_shell_hotkey(
    vk_code: u32,
    key_down: bool,
    key_up: bool,
    windows_down: bool,
    control: bool,
    alt: bool,
    shift: bool,
    active_key: u32,
) -> (bool, Option<ShellHotkeyAction>, u32) {
    if key_down
        && windows_down
        && let Some(action) = action_for_key(vk_code, control, alt, shift)
    {
        return (true, (active_key != vk_code).then_some(action), vk_code);
    }
    if key_up && active_key == vk_code {
        return (true, None, 0);
    }
    (false, None, active_key)
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    catch_unwind(AssertUnwindSafe(|| {
        if code != HC_ACTION as i32 || lparam.0 == 0 {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }
        // SAFETY: HC_ACTION guarantees lparam points to a KBDLLHOOKSTRUCT for this callback.
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        let key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        let key_up = message == WM_KEYUP || message == WM_SYSKEYUP;
        let windows_down = unsafe { GetAsyncKeyState(i32::from(VK_LWIN.0)) } < 0
            || unsafe { GetAsyncKeyState(i32::from(VK_RWIN.0)) } < 0;
        let control = unsafe { GetAsyncKeyState(i32::from(VK_CONTROL.0)) } < 0;
        let alt = unsafe { GetAsyncKeyState(i32::from(VK_MENU.0)) } < 0;
        let shift = unsafe { GetAsyncKeyState(i32::from(VK_SHIFT.0)) } < 0;
        let active_key = ACTIVE_KEY.load(Ordering::Acquire);
        let alt_tab_active = ALT_TAB_ACTIVE.load(Ordering::Acquire);
        if (alt_tab_active || (key_down && alt && !windows_down && !control))
            && event.vkCode == VK_TAB
        {
            ALT_TAB_ACTIVE.store(true, Ordering::Release);
            if key_down {
                request(if shift {
                    ShellHotkeyAction::AltTabBackward
                } else {
                    ShellHotkeyAction::AltTabForward
                });
            }
            return LRESULT(1);
        }
        if alt_tab_active && key_down && event.vkCode == 0x1b {
            ALT_TAB_ACTIVE.store(false, Ordering::Release);
            request(ShellHotkeyAction::AltTabCancel);
            return LRESULT(1);
        }
        if alt_tab_active && key_up && matches!(event.vkCode, 0x12 | 0xa4 | 0xa5) {
            ALT_TAB_ACTIVE.store(false, Ordering::Release);
            request(ShellHotkeyAction::AltTabCommit);
        }
        let (consume, request, next_key) = reduce_shell_hotkey(
            event.vkCode,
            key_down,
            key_up,
            windows_down,
            control,
            alt,
            shift,
            active_key,
        );
        ACTIVE_KEY.store(next_key, Ordering::Release);
        if let Some(request) = request {
            self::request(request);
        }
        if consume {
            return LRESULT(1);
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }))
    .unwrap_or(LRESULT(0))
}

pub struct ShellHotkeys {
    thread_id: u32,
    worker: Option<JoinHandle<()>>,
}

impl ShellHotkeys {
    pub fn start() -> Result<Self, String> {
        REQUESTED.store(0, Ordering::Release);
        ACTIVE_KEY.store(0, Ordering::Release);
        ALT_TAB_ACTIVE.store(false, Ordering::Release);
        ALT_TAB_DELTA.store(0, Ordering::Release);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name("superdesktop-win-e".into())
            .spawn(move || {
                let thread_id = unsafe { GetCurrentThreadId() };
                let mut message = MSG::default();
                // SAFETY: a no-remove peek creates this thread's message queue.
                let _ = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE) };
                let hook = match unsafe {
                    SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0)
                } {
                    Ok(hook) => hook,
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!(
                            "Shell hotkey hook registration failed: {error}"
                        )));
                        return;
                    }
                };
                if ready_tx.send(Ok(thread_id)).is_err() {
                    let _ = unsafe { UnhookWindowsHookEx(hook) };
                    return;
                }
                loop {
                    let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
                    if result.0 <= 0 {
                        break;
                    }
                }
                let _ = unsafe { UnhookWindowsHookEx(hook) };
                ACTIVE_KEY.store(0, Ordering::Release);
                ALT_TAB_ACTIVE.store(false, Ordering::Release);
                ALT_TAB_DELTA.store(0, Ordering::Release);
            })
            .map_err(|error| format!("Shell hotkey hook thread failed: {error}"))?;
        let thread_id = ready_rx
            .recv()
            .map_err(|_| "Shell hotkey hook startup channel closed".to_owned())??;
        Ok(Self {
            thread_id,
            worker: Some(worker),
        })
    }

    pub fn take_requested(&self) -> Option<ShellHotkeyAction> {
        loop {
            let delta = ALT_TAB_DELTA.load(Ordering::Acquire);
            if delta != 0 {
                if ALT_TAB_DELTA
                    .compare_exchange(
                        delta,
                        delta - delta.signum(),
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    return Some(if delta > 0 {
                        ShellHotkeyAction::AltTabForward
                    } else {
                        ShellHotkeyAction::AltTabBackward
                    });
                }
                continue;
            }
            let pending = REQUESTED.load(Ordering::Acquire);
            if pending == 0 {
                return None;
            }
            let code = 1_u32 << pending.trailing_zeros();
            if REQUESTED
                .compare_exchange(
                    pending,
                    pending & !code,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return ShellHotkeyAction::from_code(code);
            }
        }
    }
}

impl Drop for ShellHotkeys {
    fn drop(&mut self) {
        let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            eprintln!("SuperDesktop error [shell hotkeys]: hook thread panicked during shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_routes_supported_shell_keys_once_and_passes_unsupported_chords() {
        assert_eq!(
            reduce_shell_hotkey(VK_E, true, false, true, false, false, false, 0),
            (true, Some(ShellHotkeyAction::OpenExplorer), VK_E)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_E, true, false, true, false, false, false, VK_E),
            (true, None, VK_E)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_E, false, true, false, false, false, false, VK_E),
            (true, None, 0)
        );
        for (key, action) in [
            (VK_D, ShellHotkeyAction::ShowDesktop),
            (VK_SPACE, ShellHotkeyAction::CycleInput),
            (VK_S, ShellHotkeyAction::OpenSearch),
            (VK_TAB, ShellHotkeyAction::OpenTaskView),
            (VK_A, ShellHotkeyAction::OpenNetworkPower),
            (VK_N, ShellHotkeyAction::OpenNotifications),
        ] {
            assert_eq!(
                reduce_shell_hotkey(key, true, false, true, false, false, false, 0),
                (true, Some(action), key)
            );
        }
        assert_eq!(
            reduce_shell_hotkey(VK_SPACE, true, false, true, false, false, true, 0),
            (true, Some(ShellHotkeyAction::CycleInputPrevious), VK_SPACE)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_S, true, false, true, false, false, true, 0),
            (true, Some(ShellHotkeyAction::OpenScreenSnip), VK_S)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_S, true, false, true, false, false, true, VK_S),
            (true, None, VK_S)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_S, false, true, false, false, false, false, VK_S),
            (true, None, 0)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_S, true, false, true, true, false, true, 0),
            (false, None, 0)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_S, true, false, true, false, true, true, 0),
            (false, None, 0)
        );
        assert_eq!(
            reduce_shell_hotkey(VK_D, true, false, true, true, false, false, 0),
            (false, None, 0)
        );
        assert_eq!(
            reduce_shell_hotkey(0x46, true, false, true, false, false, false, 0),
            (false, None, 0)
        );
    }

    #[test]
    fn production_hook_is_shell_scoped_bounded_and_has_no_registerhotkey_fallback() {
        let source = include_str!("shell_hotkey.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "SetWindowsHookExW(WH_KEYBOARD_LL",
            "CallNextHookEx",
            "PostThreadMessageW(self.thread_id, WM_QUIT",
            "UnhookWindowsHookEx",
            "catch_unwind(AssertUnwindSafe",
            "pending & !code",
            "ALT_TAB_DELTA",
            "ShellHotkeyAction::AltTabCommit",
        ] {
            assert!(
                production.contains(required),
                "missing Win+E lifecycle token: {required}"
            );
        }
        assert!(!production.contains("RegisterHotKey"));
    }

    #[test]
    fn alt_tab_queue_preserves_repeated_cycles_before_commit() {
        REQUESTED.store(0, Ordering::Release);
        ALT_TAB_DELTA.store(0, Ordering::Release);
        request(ShellHotkeyAction::AltTabForward);
        request(ShellHotkeyAction::AltTabForward);
        request(ShellHotkeyAction::AltTabCommit);
        assert_eq!(ALT_TAB_DELTA.swap(0, Ordering::AcqRel), 2);
        assert_ne!(
            REQUESTED.load(Ordering::Acquire) & ShellHotkeyAction::AltTabCommit as u32,
            0
        );
        REQUESTED.store(0, Ordering::Release);
    }

    #[test]
    fn screen_snip_action_round_trips_through_the_bounded_queue() {
        REQUESTED.store(0, Ordering::Release);
        request(ShellHotkeyAction::OpenScreenSnip);
        let code = REQUESTED.swap(0, Ordering::AcqRel);
        assert_eq!(code, ShellHotkeyAction::OpenScreenSnip as u32);
        assert_eq!(
            ShellHotkeyAction::from_code(code),
            Some(ShellHotkeyAction::OpenScreenSnip)
        );
    }

    #[test]
    fn screen_snip_activation_is_fixed_fallible_and_has_no_fallback() {
        let source = include_str!("shell_hotkey.rs");
        let helper = source
            .split("pub fn open_screen_snipping_overlay")
            .nth(1)
            .and_then(|tail| tail.split("unsafe extern").next())
            .expect("screen snip helper");
        for required in [
            "IApplicationActivationManager",
            "ActivateApplication",
            "w!(\"Microsoft.ScreenSketch_8wekyb3d8bbwe!App\")",
            "w!(\"ms-screenclip:///?source=HotKey\")",
            "CLSCTX_LOCAL_SERVER",
            "AO_NONE",
        ] {
            assert!(
                helper.contains(required),
                "missing fixed protocol contract: {required}"
            );
        }
        for forbidden in [
            "explorer.exe",
            "SnippingTool.exe",
            "ShellExecuteExW",
            "ActivateForProtocol",
            "CreateProcess",
            "keybd_event",
            "SendInput",
        ] {
            assert!(
                !helper.contains(forbidden),
                "forbidden screen snip fallback: {forbidden}"
            );
        }
    }
}
