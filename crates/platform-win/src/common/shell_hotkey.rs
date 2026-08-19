//! Shell-owned Win+E keyboard routing with bounded thread and hook lifetime.

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LWIN, VK_RWIN},
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE,
            PeekMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
            WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

const VK_E: u32 = 0x45;
static WIN_E_DOWN: AtomicBool = AtomicBool::new(false);

fn reduce_win_e(
    vk_code: u32,
    key_down: bool,
    key_up: bool,
    windows_down: bool,
    was_down: bool,
) -> (bool, bool, bool) {
    if vk_code != VK_E {
        return (false, false, was_down);
    }
    if key_down && windows_down {
        return (true, !was_down, true);
    }
    if key_up && was_down {
        return (true, false, false);
    }
    (false, false, was_down)
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
        let was_down = WIN_E_DOWN.load(Ordering::Acquire);
        let (consume, request, next_down) =
            reduce_win_e(event.vkCode, key_down, key_up, windows_down, was_down);
        WIN_E_DOWN.store(next_down, Ordering::Release);
        if request {
            REQUESTED.store(true, Ordering::Release);
        }
        if consume {
            return LRESULT(1);
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }))
    .unwrap_or(LRESULT(0))
}

static REQUESTED: AtomicBool = AtomicBool::new(false);

pub struct WinEHotkey {
    thread_id: u32,
    worker: Option<JoinHandle<()>>,
}

impl WinEHotkey {
    pub fn start() -> Result<Self, String> {
        REQUESTED.store(false, Ordering::Release);
        WIN_E_DOWN.store(false, Ordering::Release);
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
                        let _ =
                            ready_tx.send(Err(format!("Win+E hook registration failed: {error}")));
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
                WIN_E_DOWN.store(false, Ordering::Release);
            })
            .map_err(|error| format!("Win+E hook thread failed: {error}"))?;
        let thread_id = ready_rx
            .recv()
            .map_err(|_| "Win+E hook startup channel closed".to_owned())??;
        Ok(Self {
            thread_id,
            worker: Some(worker),
        })
    }

    pub fn take_requested(&self) -> bool {
        REQUESTED.swap(false, Ordering::AcqRel)
    }
}

impl Drop for WinEHotkey {
    fn drop(&mut self) {
        let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            eprintln!("SuperDesktop error [Win+E]: hook thread panicked during shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_e_reducer_suppresses_one_request_per_physical_press() {
        assert_eq!(
            reduce_win_e(VK_E, true, false, true, false),
            (true, true, true)
        );
        assert_eq!(
            reduce_win_e(VK_E, true, false, true, true),
            (true, false, true)
        );
        assert_eq!(
            reduce_win_e(VK_E, false, true, false, true),
            (true, false, false)
        );
        assert_eq!(
            reduce_win_e(VK_E, true, false, false, false),
            (false, false, false)
        );
        assert_eq!(
            reduce_win_e(0x46, true, false, true, false),
            (false, false, false)
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
            "REQUESTED.swap(false",
        ] {
            assert!(
                production.contains(required),
                "missing Win+E lifecycle token: {required}"
            );
        }
        assert!(!production.contains("RegisterHotKey"));
    }
}
