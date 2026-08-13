//! Reversible AppBar and Shell Hook capability adapter for a caller-owned test HWND.
//!
//! It is deliberately preview-only: the adapter never creates a window, never
//! enumerates or changes Explorer, and never attempts a shell takeover. The caller
//! must supply a same-thread HWND it owns and must retain it until `teardown`.

use std::{marker::PhantomData, mem::size_of, rc::Rc};

use windows::Win32::{
    Foundation::{HWND, RECT},
    System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
    UI::{
        Shell::{
            ABE_BOTTOM, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA, SHAppBarMessage,
        },
        WindowsAndMessaging::{
            DeregisterShellHookWindow, GetWindowThreadProcessId, IsWindow, RegisterShellHookWindow,
            RegisterWindowMessageW,
        },
    },
};
use windows::core::w;

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
    _thread_affine: PhantomData<Rc<()>>,
}

impl ControlledShellCapability {
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
            // SAFETY: registers only this validated caller-owned testing HWND.
            if !unsafe { RegisterShellHookWindow(self.hwnd).as_bool() } {
                return Err("shell-hook-register-failed");
            }
            self.shell_hook_registered = true;
        }
        Ok(message)
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
        if self.appbar_registered {
            let mut data = self.appbar_data();
            // SAFETY: removes only the registration created for this controlled HWND.
            result.appbar_removed = unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) } != 0;
            if result.appbar_removed {
                self.appbar_registered = false;
            }
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
    use super::ScreenRect;

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
}
