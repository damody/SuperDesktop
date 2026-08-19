//! Shell-owned Windows-key routing with bounded thread and hook lifetime.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        atomic::{AtomicU8, AtomicU32, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE,
            PeekMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
            WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

const VK_A: u32 = 0x41;
const VK_D: u32 = 0x44;
const VK_E: u32 = 0x45;
const VK_N: u32 = 0x4e;
const VK_S: u32 = 0x53;
const VK_SPACE: u32 = 0x20;
const VK_TAB: u32 = 0x09;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ShellHotkeyAction {
    OpenExplorer = 1,
    ShowDesktop = 2,
    CycleInput = 3,
    OpenSearch = 4,
    OpenTaskView = 5,
    OpenNetworkPower = 6,
    OpenNotifications = 7,
    CycleInputPrevious = 8,
}

impl ShellHotkeyAction {
    fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::OpenExplorer),
            2 => Some(Self::ShowDesktop),
            3 => Some(Self::CycleInput),
            4 => Some(Self::OpenSearch),
            5 => Some(Self::OpenTaskView),
            6 => Some(Self::OpenNetworkPower),
            7 => Some(Self::OpenNotifications),
            8 => Some(Self::CycleInputPrevious),
            _ => None,
        }
    }
}

static ACTIVE_KEY: AtomicU32 = AtomicU32::new(0);
static REQUESTED: AtomicU8 = AtomicU8::new(0);

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
        VK_S if !shift => Some(ShellHotkeyAction::OpenSearch),
        VK_TAB if !shift => Some(ShellHotkeyAction::OpenTaskView),
        VK_A if !shift => Some(ShellHotkeyAction::OpenNetworkPower),
        VK_N if !shift => Some(ShellHotkeyAction::OpenNotifications),
        _ => None,
    }
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
            let _ =
                REQUESTED.compare_exchange(0, request as u8, Ordering::AcqRel, Ordering::Acquire);
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
        ShellHotkeyAction::from_code(REQUESTED.swap(0, Ordering::AcqRel))
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
            "REQUESTED.swap(0",
        ] {
            assert!(
                production.contains(required),
                "missing Win+E lifecycle token: {required}"
            );
        }
        assert!(!production.contains("RegisterHotKey"));
    }
}
