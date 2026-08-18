//! Reversible AppBar and Shell Hook capability adapter for a caller-owned test HWND.
//!
//! It is deliberately preview-only: the adapter never creates a window, never
//! enumerates or changes Explorer, and never attempts a shell takeover. The caller
//! must supply a same-thread HWND it owns and must retain it until `teardown`.

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    marker::PhantomData,
    mem::size_of,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
    UI::{
        Shell::{
            ABE_BOTTOM, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA, DefSubclassProc,
            RemoveWindowSubclass, SHAppBarMessage, SetWindowSubclass,
        },
        WindowsAndMessaging::{
            DeregisterShellHookWindow, GetCaretBlinkTime, GetWindowThreadProcessId, IsWindow,
            RegisterShellHookWindow, RegisterWindowMessageW,
        },
    },
};
use windows::core::w;

const SHELL_HOOK_SUBCLASS_ID: usize = 0x5348_4f4f;
const SHELL_HOOK_QUEUE_CAPACITY: usize = 256;

thread_local! {
    static SHELL_HOOK_MESSAGE: Cell<u32> = const { Cell::new(0) };
    static SHELL_HOOK_EVENTS: RefCell<VecDeque<OwnedShellHookEvent>> =
        RefCell::new(VecDeque::with_capacity(SHELL_HOOK_QUEUE_CAPACITY));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnedShellHookEvent {
    pub code: u32,
    pub hwnd_identity: isize,
    pub process_id: u32,
    pub session_id: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
}

fn owned_shell_hook_event(code: u32, hwnd_identity: isize) -> Option<OwnedShellHookEvent> {
    let hwnd = HWND(hwnd_identity as _);
    let mut process_id = 0;
    // SAFETY: query-only lookup for the copied numeric HWND delivered by Windows.
    if unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) } == 0 || process_id == 0 {
        return (code == 2).then_some(OwnedShellHookEvent {
            code,
            hwnd_identity,
            process_id: 0,
            session_id: 0,
        });
    }
    let mut session_id = 0;
    let mut current_session = 0;
    // SAFETY: both APIs copy scalar session IDs into local storage.
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0
        || unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut current_session) } == 0
        || !same_session(session_id, current_session)
    {
        return None;
    }
    Some(OwnedShellHookEvent {
        code,
        hwnd_identity,
        process_id,
        session_id,
    })
}

const fn same_session(owner_session: u32, current_session: u32) -> bool {
    owner_session != 0 && owner_session == current_session
}

/// Uses the system caret cadence as Windows' bounded default flash interval.
/// USER_TIMER_MAXIMUM/no-blink and implausible values fall back to 500 ms.
pub fn system_attention_cadence_ms() -> u32 {
    // SAFETY: read-only process-independent system timing query.
    let value = unsafe { GetCaretBlinkTime() };
    if (100..=5_000).contains(&value) {
        value
    } else {
        500
    }
}

fn enqueue_shell_hook_event(event: OwnedShellHookEvent) {
    SHELL_HOOK_EVENTS.with(|events| {
        let mut events = events.borrow_mut();
        if events.back().copied() == Some(event) {
            return;
        }
        if events.len() == SHELL_HOOK_QUEUE_CAPACITY {
            events.pop_front();
        }
        events.push_back(event);
    });
}

unsafe extern "system" fn shell_hook_subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _ref_data: usize,
) -> LRESULT {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        SHELL_HOOK_MESSAGE.with(|registered| {
            if registered.get() == message
                && let Some(event) = owned_shell_hook_event(wparam.0 as u32, lparam.0)
            {
                enqueue_shell_hook_event(event);
            }
        });
    }));
    // SAFETY: no Rust borrow or lock is held while forwarding the original message.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityTeardown {
    pub appbar_removed: bool,
    pub shell_hook_unregistered: bool,
}

/// A same-thread capability lease over a caller-owned testing window. It is not
/// `Send`, `Sync`, `Copy`, or `Clone`; the lease's `Drop` only attempts reversible
/// deregistration and never destroys the supplied HWND.
pub struct ControlledShellCapability {
    hwnd: HWND,
    owner_thread: u32,
    appbar_registered: bool,
    shell_hook_registered: bool,
    shell_hook_subclass_installed: bool,
    _thread_affine: PhantomData<Rc<()>>,
}

impl ControlledShellCapability {
    pub fn owns_window(&self, hwnd_identity: isize) -> bool {
        self.hwnd.0 as isize == hwnd_identity
    }

    pub const fn appbar_registered(&self) -> bool {
        self.appbar_registered
    }

    /// Attaches only to a caller-owned HWND in this process and on this thread.
    pub fn attach_controlled_window(hwnd: isize) -> Result<Self, &'static str> {
        if hwnd == 0 {
            return Err("controlled-hwnd-null");
        }
        let hwnd = HWND(hwnd as _);
        // SAFETY: query-only validation of the caller-supplied numeric HWND.
        if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return Err("controlled-hwnd-invalid");
        }
        let mut pid = 0;
        // SAFETY: query-only ownership lookup with local output storage.
        let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        // SAFETY: read-only current process/thread identity queries.
        if thread == 0
            || pid != unsafe { GetCurrentProcessId() }
            || thread != unsafe { GetCurrentThreadId() }
        {
            return Err("controlled-hwnd-not-current-owner");
        }
        Ok(Self {
            hwnd,
            owner_thread: thread,
            appbar_registered: false,
            shell_hook_registered: false,
            shell_hook_subclass_installed: false,
            _thread_affine: PhantomData,
        })
    }

    fn validate_active(&self) -> Result<(), &'static str> {
        // SAFETY: read-only current thread identity query.
        if unsafe { GetCurrentThreadId() } != self.owner_thread {
            return Err("controlled-hwnd-thread-drift");
        }
        // SAFETY: query-only validity check of the borrowed caller-owned HWND.
        if !unsafe { IsWindow(Some(self.hwnd)).as_bool() } {
            return Err("controlled-hwnd-retired");
        }
        Ok(())
    }

    fn appbar_data(&self) -> APPBARDATA {
        APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            hWnd: self.hwnd,
            ..Default::default()
        }
    }

    /// Registers this controlled HWND as an AppBar. It is idempotent.
    pub fn register_appbar(&mut self) -> Result<(), &'static str> {
        self.validate_active()?;
        if self.appbar_registered {
            return Ok(());
        }
        let mut data = self.appbar_data();
        // SAFETY: APPBARDATA points to local initialized storage and references
        // only this validated caller-owned testing HWND.
        if unsafe { SHAppBarMessage(ABM_NEW, &mut data) } == 0 {
            return Err("appbar-register-failed");
        }
        self.appbar_registered = true;
        Ok(())
    }

    /// Removes only this lease's AppBar reservation while retaining the owned
    /// Shell Hook subscription. This is used by owned taskbar auto-hide.
    pub fn remove_appbar(&mut self) -> Result<bool, &'static str> {
        self.validate_active()?;
        if !self.appbar_registered {
            return Ok(false);
        }
        let mut data = self.appbar_data();
        // SAFETY: removes only the registration created for this controlled HWND.
        if unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) } == 0 {
            return Err("appbar-remove-failed");
        }
        self.appbar_registered = false;
        Ok(true)
    }

    /// Asks the shell for a bottom reservation, then applies its exact returned
    /// rectangle to the controlled test AppBar. No Explorer HWND is touched.
    pub fn reserve_bottom(
        &mut self,
        monitor: ScreenRect,
        thickness: i32,
    ) -> Result<ScreenRect, &'static str> {
        self.validate_active()?;
        if !self.appbar_registered
            || thickness <= 0
            || monitor.right <= monitor.left
            || monitor.bottom <= monitor.top
        {
            return Err("appbar-reserve-precondition");
        }
        let mut data = self.appbar_data();
        data.uEdge = ABE_BOTTOM;
        data.rc = RECT {
            left: monitor.left,
            top: monitor.bottom.saturating_sub(thickness),
            right: monitor.right,
            bottom: monitor.bottom,
        };
        // SAFETY: data is local and describes only the registered controlled HWND.
        unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut data) };
        // SAFETY: applies the shell-negotiated geometry to the same controlled HWND.
        if unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) } == 0 {
            return Err("appbar-setpos-failed");
        }
        Ok(ScreenRect {
            left: data.rc.left,
            top: data.rc.top,
            right: data.rc.right,
            bottom: data.rc.bottom,
        })
    }

    /// Registers this controlled HWND for Shell Hook delivery. The returned message
    /// ID is copied as an integer; this adapter does not inspect or mutate Explorer.
    pub fn register_shell_hook(&mut self) -> Result<u32, &'static str> {
        self.validate_active()?;
        // SAFETY: registers a process-wide message name and does not mutate a window.
        let message = unsafe { RegisterWindowMessageW(w!("SHELLHOOK")) };
        if message == 0 {
            return Err("shell-hook-message-register-failed");
        }
        if !self.shell_hook_registered {
            if !unsafe {
                SetWindowSubclass(
                    self.hwnd,
                    Some(shell_hook_subclass_proc),
                    SHELL_HOOK_SUBCLASS_ID,
                    0,
                )
                .as_bool()
            } {
                return Err("shell-hook-subclass-install-failed");
            }
            self.shell_hook_subclass_installed = true;
            // SAFETY: registers only this validated caller-owned testing HWND.
            if !unsafe { RegisterShellHookWindow(self.hwnd).as_bool() } {
                unsafe {
                    let _ = RemoveWindowSubclass(
                        self.hwnd,
                        Some(shell_hook_subclass_proc),
                        SHELL_HOOK_SUBCLASS_ID,
                    );
                };
                self.shell_hook_subclass_installed = false;
                return Err("shell-hook-register-failed");
            }
            self.shell_hook_registered = true;
            SHELL_HOOK_MESSAGE.with(|registered| registered.set(message));
        }
        Ok(message)
    }

    /// Drains owned copies captured by the same-thread subclass. Duplicate events
    /// are coalesced and the queue is bounded so a misbehaving sender cannot grow it.
    pub fn drain_shell_hook_events() -> Vec<OwnedShellHookEvent> {
        SHELL_HOOK_EVENTS.with(|events| events.borrow_mut().drain(..).collect())
    }

    /// Reverses registrations in hook-first order. Calling this repeatedly is safe.
    pub fn teardown(&mut self) -> CapabilityTeardown {
        let mut result = CapabilityTeardown {
            appbar_removed: false,
            shell_hook_unregistered: false,
        };
        // A retired window cannot be safely used for deregistration. Preserve the
        // state so the caller records a typed incomplete cleanup instead of risking
        // an HWND-reuse mutation.
        if self.validate_active().is_err() {
            return result;
        }
        if self.shell_hook_registered {
            // SAFETY: same owner thread and validated caller-owned HWND.
            result.shell_hook_unregistered =
                unsafe { DeregisterShellHookWindow(self.hwnd).as_bool() };
            if result.shell_hook_unregistered {
                self.shell_hook_registered = false;
            }
        }
        if self.shell_hook_subclass_installed {
            // SAFETY: removes only the callback installed by this lease on its HWND.
            if unsafe {
                RemoveWindowSubclass(
                    self.hwnd,
                    Some(shell_hook_subclass_proc),
                    SHELL_HOOK_SUBCLASS_ID,
                )
                .as_bool()
            } {
                self.shell_hook_subclass_installed = false;
            }
        }
        if self.appbar_registered {
            result.appbar_removed = self.remove_appbar().unwrap_or(false);
        }
        result
    }
}

impl Drop for ControlledShellCapability {
    fn drop(&mut self) {
        let _ = self.teardown();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ControlledShellCapability, OwnedShellHookEvent, SHELL_HOOK_QUEUE_CAPACITY, ScreenRect,
        enqueue_shell_hook_event, owned_shell_hook_event, same_session,
    };

    #[test]
    fn screen_rect_is_owned_and_has_no_native_handle() {
        let rect = ScreenRect {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        };
        assert_eq!(rect.right - rect.left, 2);
        assert_eq!(rect.bottom - rect.top, 2);
    }

    #[test]
    fn shell_hook_queue_coalesces_duplicates_and_is_bounded() {
        let _ = ControlledShellCapability::drain_shell_hook_events();
        let duplicate = OwnedShellHookEvent {
            code: 0x8006,
            hwnd_identity: 41,
            process_id: 7,
            session_id: 3,
        };
        enqueue_shell_hook_event(duplicate);
        enqueue_shell_hook_event(duplicate);
        for hwnd_identity in 0..SHELL_HOOK_QUEUE_CAPACITY as isize + 10 {
            enqueue_shell_hook_event(OwnedShellHookEvent {
                code: 4,
                hwnd_identity,
                process_id: 7,
                session_id: 3,
            });
        }
        let events = ControlledShellCapability::drain_shell_hook_events();
        assert_eq!(events.len(), SHELL_HOOK_QUEUE_CAPACITY);
        assert_eq!(events.last().map(|event| event.hwnd_identity), Some(265));
    }

    #[test]
    fn wrong_session_and_retired_flash_identity_fail_closed() {
        assert!(same_session(4, 4));
        assert!(!same_session(4, 5));
        assert!(!same_session(0, 0));
        assert_eq!(owned_shell_hook_event(0x8006, 0), None);
        assert!(include_str!("appbar_shell_hook.rs").contains("catch_unwind(AssertUnwindSafe"));
    }
}
