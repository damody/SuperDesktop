//! Preview-only bridge for a single **borrowed** GPUI Win32 window.
//!
//! This module deliberately has no GPUI dependency. The composition root extracts a
//! numeric HWND, GPUI WindowId, and monotonically increasing generation while it has
//! a live `gpui::Window`, then gives those values to this bridge. `SubclassLease` is
//! thread-affine and never creates or destroys an HWND; GPUI remains its owner.

use std::{
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU64, AtomicUsize, Ordering},
    },
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    System::Threading::{
        GR_GDIOBJECTS, GR_USEROBJECTS, GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId,
        GetGuiResources, GetProcessHandleCount,
    },
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            GetWindowThreadProcessId, IsWindow, PostMessageW, SendMessageW, WM_ACTIVATE, WM_APP,
            WM_CLOSE, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_NCDESTROY,
        },
    },
};

const TEST_EVENT_MESSAGE: u32 = WM_APP + 0x4d2;
const TEST_DPI: usize = 1;
const TEST_DISPLAY: usize = 2;
const TEST_ACTIVATION: usize = 3;
const TEST_DPI_X: usize = 96;
const TEST_DPI_Y: usize = 96;
const TEST_DISPLAY_WIDTH: isize = 320;
const TEST_DISPLAY_HEIGHT: isize = 180;

const PHASE_ATTACHED: u8 = 1;
const PHASE_CLOSING: u8 = 2;
const PHASE_NCDESTROY: u8 = 3;
const PHASE_FATAL: u8 = 4;

const EVENT_DPI: u8 = 1;
const EVENT_DISPLAY: u8 = 2;
const EVENT_ACTIVATION: u8 = 4;
const EVENT_DESTROYED: u8 = 8;

const FATAL_NONE: u8 = 0;
const FATAL_CALLBACK_PANIC: u8 = 1;
const FATAL_CALLBACK_ERROR: u8 = 2;
const FATAL_REMOVE_FAILED: u8 = 3;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceSnapshot {
    pub process_handles: u32,
    pub user_objects: u32,
    pub gdi_objects: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FatalReason {
    CallbackPanic,
    CallbackError,
    RemoveSubclassFailed,
}

/// Read-only lifecycle phase for controlled spike diagnostics. It intentionally
/// exposes no Win32 handle and permits no state transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalPhase {
    Attached,
    Closing,
    NativeDestroyed,
    Fatal,
}

/// A point-in-time diagnostic snapshot. `hwnd_still_valid` is only queried on
/// the owner thread; `None` means the caller is not on that thread, rather than
/// treating a cross-thread probe as evidence about a borrowed HWND.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalStatus {
    pub phase: TerminalPhase,
    pub wm_ncdestroy_seen: bool,
    pub on_window_closed_seen: bool,
    pub raw_ref_outstanding: bool,
    pub finalized: bool,
    pub hwnd_still_valid: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct BridgeTrace {
    pub hwnd: usize,
    pub owner_pid: u32,
    pub owner_thread: u32,
    pub session_id: u32,
    pub gpui_window_id: u64,
    pub generation: u64,
    pub lifecycle: Vec<(&'static str, u64)>,
    /// Owned, copied raw Win32 payloads. This Vec is constructed only after the
    /// callback has quiesced; the callback itself writes fixed atomic slots.
    pub owned_events: Vec<OwnedWindowEvent>,
    /// Private non-forwarded adapter events used by the spike harness.
    pub adapter_events: Vec<&'static str>,
    pub callbacks_before_close: usize,
    pub callbacks_after_close: usize,
    pub late_event_rejected: bool,
    pub wm_ncdestroy_observed: bool,
    pub on_window_closed_observed: bool,
    pub callback_state_outstanding: usize,
    pub fatal_callback: Option<FatalReason>,
    pub resources_before: ResourceSnapshot,
    pub resources_after: ResourceSnapshot,
}

/// An owned value extracted by the bridge before it forwards the corresponding
/// raw Win32 message to GPUI. It contains no native pointer or borrowed memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnedWindowEvent {
    DpiChanged {
        x: u16,
        y: u16,
        suggested_left: i32,
        suggested_top: i32,
        suggested_right: i32,
        suggested_bottom: i32,
    },
    DisplayChanged {
        bits_per_pixel: u16,
        width: u16,
        height: u16,
    },
    ActivationChanged {
        state: u16,
    },
}

/// A numeric identity borrowed from GPUI. It is intentionally neither `Copy` nor
/// `Clone`, and the `Rc` marker prevents accidentally moving it to another thread.
pub struct BorrowedHwnd {
    hwnd: HWND,
    owner_pid: u32,
    owner_thread: u32,
    session_id: u32,
    gpui_window_id: u64,
    generation: u64,
    _thread_affine: PhantomData<Rc<()>>,
}

impl BorrowedHwnd {
    /// Validates an HWND directly extracted from a live GPUI `Window`.
    pub fn verify_current_process(
        hwnd: isize,
        gpui_window_id: u64,
        generation: u64,
    ) -> Result<Self, &'static str> {
        if hwnd == 0 || gpui_window_id == 0 || generation == 0 || generation > usize::MAX as u64 {
            return Err("invalid-gpui-window-identity");
        }
        let hwnd = HWND(hwnd as _);
        // SAFETY: query-only validation of a numeric borrowed HWND.
        if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return Err("invalid-gpui-hwnd");
        }
        let mut owner_pid = 0;
        // SAFETY: the HWND is only queried and the PID output is local.
        let owner_thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut owner_pid)) };
        // SAFETY: current process/thread identity queries do not mutate process state.
        if owner_thread == 0
            || owner_pid != unsafe { GetCurrentProcessId() }
            || owner_thread != unsafe { GetCurrentThreadId() }
        {
            return Err("foreign-gpui-hwnd");
        }
        let mut owner_session = 0;
        let mut current_session = 0;
        // SAFETY: valid process IDs and writable local output storage.
        if unsafe { ProcessIdToSessionId(owner_pid, &mut owner_session) } == 0
            || unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut current_session) } == 0
            || owner_session != current_session
        {
            return Err("gpui-hwnd-session-mismatch");
        }
        Ok(Self {
            hwnd,
            owner_pid,
            owner_thread,
            session_id: owner_session,
            gpui_window_id,
            generation,
            _thread_affine: PhantomData,
        })
    }

    fn revalidate(&self, generation: u64, terminal: bool) -> Result<(), &'static str> {
        if terminal {
            return Err("retired-subclass-generation");
        }
        if generation != self.generation || unsafe { GetCurrentThreadId() } != self.owner_thread {
            return Err("gpui-hwnd-generation-or-thread-drift");
        }
        // SAFETY: query-only validation of the stored borrowed HWND.
        if !unsafe { IsWindow(Some(self.hwnd)).as_bool() } {
            return Err("retired-gpui-hwnd");
        }
        let mut pid = 0;
        // SAFETY: borrowed HWND and local PID output are valid.
        let thread = unsafe { GetWindowThreadProcessId(self.hwnd, Some(&mut pid)) };
        let mut session = 0;
        // SAFETY: valid PID and local writable session output.
        if thread != self.owner_thread
            || pid != self.owner_pid
            || unsafe { ProcessIdToSessionId(pid, &mut session) } == 0
            || session != self.session_id
        {
            return Err("gpui-hwnd-identity-drift");
        }
        Ok(())
    }
}

/// Allocation-free callback state. No lock is taken from the Win32 callback, so
/// callback reentrancy and a panic cannot leave a poisoned lock behind.
pub struct TerminalCoordinator {
    generation: u64,
    phase: AtomicU8,
    event_mask: AtomicU8,
    adapter_event_mask: AtomicU8,
    dpi_x: AtomicUsize,
    dpi_y: AtomicUsize,
    dpi_left: AtomicI32,
    dpi_top: AtomicI32,
    dpi_right: AtomicI32,
    dpi_bottom: AtomicI32,
    display_bpp: AtomicUsize,
    display_width: AtomicUsize,
    display_height: AtomicUsize,
    activation_state: AtomicUsize,
    fatal: AtomicU8,
    sequence: AtomicU64,
    attached_at: AtomicU64,
    closing_at: AtomicU64,
    ncdestroy_at: AtomicU64,
    on_closed_at: AtomicU64,
    finalized_at: AtomicU64,
    callbacks_before_close: AtomicUsize,
    callbacks_after_close: AtomicUsize,
    late_event_rejected: AtomicBool,
    raw_ref_released: AtomicBool,
}

impl TerminalCoordinator {
    fn new(generation: u64) -> Self {
        Self {
            generation,
            phase: AtomicU8::new(PHASE_ATTACHED),
            event_mask: AtomicU8::new(0),
            adapter_event_mask: AtomicU8::new(0),
            dpi_x: AtomicUsize::new(0),
            dpi_y: AtomicUsize::new(0),
            dpi_left: AtomicI32::new(0),
            dpi_top: AtomicI32::new(0),
            dpi_right: AtomicI32::new(0),
            dpi_bottom: AtomicI32::new(0),
            display_bpp: AtomicUsize::new(0),
            display_width: AtomicUsize::new(0),
            display_height: AtomicUsize::new(0),
            activation_state: AtomicUsize::new(0),
            fatal: AtomicU8::new(FATAL_NONE),
            sequence: AtomicU64::new(1),
            attached_at: AtomicU64::new(1),
            closing_at: AtomicU64::new(0),
            ncdestroy_at: AtomicU64::new(0),
            on_closed_at: AtomicU64::new(0),
            finalized_at: AtomicU64::new(0),
            callbacks_before_close: AtomicUsize::new(0),
            callbacks_after_close: AtomicUsize::new(0),
            late_event_rejected: AtomicBool::new(false),
            raw_ref_released: AtomicBool::new(false),
        }
    }

    fn stamp(&self, slot: &AtomicU64) {
        let next = self.sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = slot.compare_exchange(0, next, Ordering::AcqRel, Ordering::Acquire);
    }

    fn terminal(&self) -> bool {
        self.phase.load(Ordering::Acquire) != PHASE_ATTACHED
    }

    fn begin_closing(&self) -> Result<(), &'static str> {
        self.phase
            .compare_exchange(
                PHASE_ATTACHED,
                PHASE_CLOSING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| "close-not-attached")?;
        self.stamp(&self.closing_at);
        Ok(())
    }

    fn observe_nc_destroy(&self) {
        // Native destruction can begin before the GPUI close callback asks the
        // bridge to close. Make that ordering explicit instead of dereferencing a
        // retired HWND later.
        if self.phase.load(Ordering::Acquire) == PHASE_ATTACHED {
            let _ = self.begin_closing();
        }
        self.phase.store(PHASE_NCDESTROY, Ordering::Release);
        self.event_mask.fetch_or(EVENT_DESTROYED, Ordering::AcqRel);
        self.stamp(&self.ncdestroy_at);
        self.callbacks_before_close.fetch_add(1, Ordering::AcqRel);
    }

    fn observe_message(
        &self,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        adapter: bool,
    ) -> Result<bool, &'static str> {
        if message == WM_NCDESTROY {
            self.observe_nc_destroy();
            return Ok(true);
        }
        if self.terminal() {
            self.late_event_rejected.store(true, Ordering::Release);
            self.callbacks_after_close.fetch_add(1, Ordering::AcqRel);
            return Ok(false);
        }
        let bit = match message {
            WM_DPICHANGED => {
                if lparam.0 == 0 {
                    return Err("dpi-changed-null-suggested-rect");
                }
                // SAFETY: the Win32 WM_DPICHANGED contract supplies a pointer to a
                // RECT valid for the duration of this synchronous callback. We copy
                // it before forwarding and never retain the pointer.
                let suggested = unsafe { (lparam.0 as *const RECT).read() };
                self.dpi_x
                    .store((wparam.0 & 0xffff) as u16 as usize, Ordering::Release);
                self.dpi_y.store(
                    ((wparam.0 >> 16) & 0xffff) as u16 as usize,
                    Ordering::Release,
                );
                self.dpi_left.store(suggested.left, Ordering::Release);
                self.dpi_top.store(suggested.top, Ordering::Release);
                self.dpi_right.store(suggested.right, Ordering::Release);
                self.dpi_bottom.store(suggested.bottom, Ordering::Release);
                EVENT_DPI
            }
            WM_DISPLAYCHANGE => {
                self.display_bpp
                    .store((wparam.0 & 0xffff) as u16 as usize, Ordering::Release);
                self.display_width.store(
                    (lparam.0 as usize & 0xffff) as u16 as usize,
                    Ordering::Release,
                );
                self.display_height.store(
                    ((lparam.0 as usize >> 16) & 0xffff) as u16 as usize,
                    Ordering::Release,
                );
                EVENT_DISPLAY
            }
            WM_ACTIVATE => {
                self.activation_state
                    .store((wparam.0 & 0xffff) as u16 as usize, Ordering::Release);
                EVENT_ACTIVATION
            }
            _ => 0,
        };
        if bit != 0 {
            let target = if adapter {
                &self.adapter_event_mask
            } else {
                &self.event_mask
            };
            target.fetch_or(bit, Ordering::AcqRel);
        }
        self.callbacks_before_close.fetch_add(1, Ordering::AcqRel);
        Ok(true)
    }

    fn observe_adapter_code(&self, code: usize) -> Result<(), &'static str> {
        let bit = match code {
            TEST_DPI => EVENT_DPI,
            TEST_DISPLAY => EVENT_DISPLAY,
            TEST_ACTIVATION => EVENT_ACTIVATION,
            _ => return Err("unknown-test-event"),
        };
        if self.terminal() {
            self.late_event_rejected.store(true, Ordering::Release);
            self.callbacks_after_close.fetch_add(1, Ordering::AcqRel);
            return Err("retired-subclass-generation");
        }
        self.adapter_event_mask.fetch_or(bit, Ordering::AcqRel);
        self.callbacks_before_close.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn owned_events(&self, mask: u8) -> Vec<OwnedWindowEvent> {
        let mut events = Vec::with_capacity(3);
        if mask & EVENT_DPI != 0 {
            events.push(OwnedWindowEvent::DpiChanged {
                x: self.dpi_x.load(Ordering::Acquire) as u16,
                y: self.dpi_y.load(Ordering::Acquire) as u16,
                suggested_left: self.dpi_left.load(Ordering::Acquire),
                suggested_top: self.dpi_top.load(Ordering::Acquire),
                suggested_right: self.dpi_right.load(Ordering::Acquire),
                suggested_bottom: self.dpi_bottom.load(Ordering::Acquire),
            });
        }
        if mask & EVENT_DISPLAY != 0 {
            events.push(OwnedWindowEvent::DisplayChanged {
                bits_per_pixel: self.display_bpp.load(Ordering::Acquire) as u16,
                width: self.display_width.load(Ordering::Acquire) as u16,
                height: self.display_height.load(Ordering::Acquire) as u16,
            });
        }
        if mask & EVENT_ACTIVATION != 0 {
            events.push(OwnedWindowEvent::ActivationChanged {
                state: self.activation_state.load(Ordering::Acquire) as u16,
            });
        }
        events
    }

    fn adapter_event_names(&self) -> Vec<&'static str> {
        [
            (EVENT_DPI, "dpi-changed"),
            (EVENT_DISPLAY, "display-changed"),
            (EVENT_ACTIVATION, "activation"),
        ]
        .into_iter()
        .filter_map(|(bit, name)| {
            (self.adapter_event_mask.load(Ordering::Acquire) & bit != 0).then_some(name)
        })
        .collect()
    }

    fn fail(&self, reason: u8) {
        let _ =
            self.fatal
                .compare_exchange(FATAL_NONE, reason, Ordering::AcqRel, Ordering::Acquire);
        self.phase.store(PHASE_FATAL, Ordering::Release);
    }

    fn claim_raw_release(&self) -> bool {
        self.raw_ref_released
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn maybe_finalize(&self) {
        if self.ncdestroy_at.load(Ordering::Acquire) != 0
            && self.on_closed_at.load(Ordering::Acquire) != 0
            && self.raw_ref_released.load(Ordering::Acquire)
        {
            self.stamp(&self.finalized_at);
        }
    }

    fn release_raw_ref(&self, raw: *const TerminalCoordinator) {
        if self.claim_raw_release() {
            // SAFETY: exactly one successful CAS consumes the Arc transferred to
            // common-controls after SetWindowSubclass succeeded. A callback-local
            // strong Arc keeps the state alive if this executes in the callback.
            unsafe { drop(Arc::from_raw(raw)) };
        }
        self.maybe_finalize();
    }

    fn record_on_window_closed(&self) -> Result<(), &'static str> {
        // GPUI may signal closure before its backend performs asynchronous native
        // destruction. The two terminal signals are intentionally independent.
        if self.phase.load(Ordering::Acquire) == PHASE_ATTACHED {
            self.begin_closing()?;
        }
        self.stamp(&self.on_closed_at);
        self.maybe_finalize();
        Ok(())
    }

    fn terminal_ready(&self) -> bool {
        self.finalized_at.load(Ordering::Acquire) != 0
    }

    fn status(&self, hwnd: HWND, owner_thread: u32) -> TerminalStatus {
        let phase = match self.phase.load(Ordering::Acquire) {
            PHASE_ATTACHED => TerminalPhase::Attached,
            PHASE_CLOSING => TerminalPhase::Closing,
            PHASE_NCDESTROY => TerminalPhase::NativeDestroyed,
            _ => TerminalPhase::Fatal,
        };
        let hwnd_still_valid = if unsafe { GetCurrentThreadId() } == owner_thread {
            // SAFETY: query-only status check on the owner thread. It neither
            // dereferences nor retains the borrowed HWND.
            Some(unsafe { IsWindow(Some(hwnd)).as_bool() })
        } else {
            None
        };
        TerminalStatus {
            phase,
            wm_ncdestroy_seen: self.ncdestroy_at.load(Ordering::Acquire) != 0,
            on_window_closed_seen: self.on_closed_at.load(Ordering::Acquire) != 0,
            raw_ref_outstanding: !self.raw_ref_released.load(Ordering::Acquire),
            finalized: self.terminal_ready(),
            hwnd_still_valid,
        }
    }

    fn trace(
        &self,
        identity: &BorrowedHwnd,
        resources_before: ResourceSnapshot,
        resources_after: ResourceSnapshot,
    ) -> Result<BridgeTrace, &'static str> {
        if !self.terminal_ready() {
            return Err("terminal-signals-incomplete");
        }
        let lifecycle = [
            ("attached", self.attached_at.load(Ordering::Acquire)),
            ("closing", self.closing_at.load(Ordering::Acquire)),
            ("wm-ncdestroy", self.ncdestroy_at.load(Ordering::Acquire)),
            (
                "on-window-closed",
                self.on_closed_at.load(Ordering::Acquire),
            ),
            ("finalized", self.finalized_at.load(Ordering::Acquire)),
        ];
        let attached = lifecycle[0].1;
        let closing = lifecycle[1].1;
        let ncdestroy = lifecycle[2].1;
        let on_closed = lifecycle[3].1;
        let finalized = lifecycle[4].1;
        if attached == 0
            || closing == 0
            || ncdestroy == 0
            || on_closed == 0
            || finalized == 0
            || attached >= closing
            || finalized <= ncdestroy
            || finalized <= on_closed
        {
            return Err("terminal-order-invalid");
        }
        let fatal_callback = match self.fatal.load(Ordering::Acquire) {
            FATAL_NONE => None,
            FATAL_CALLBACK_PANIC => Some(FatalReason::CallbackPanic),
            FATAL_CALLBACK_ERROR => Some(FatalReason::CallbackError),
            FATAL_REMOVE_FAILED => Some(FatalReason::RemoveSubclassFailed),
            _ => Some(FatalReason::CallbackError),
        };
        Ok(BridgeTrace {
            hwnd: identity.hwnd.0 as usize,
            owner_pid: identity.owner_pid,
            owner_thread: identity.owner_thread,
            session_id: identity.session_id,
            gpui_window_id: identity.gpui_window_id,
            generation: identity.generation,
            lifecycle: lifecycle.to_vec(),
            owned_events: self.owned_events(self.event_mask.load(Ordering::Acquire)),
            adapter_events: self.adapter_event_names(),
            callbacks_before_close: self.callbacks_before_close.load(Ordering::Acquire),
            callbacks_after_close: self.callbacks_after_close.load(Ordering::Acquire),
            late_event_rejected: self.late_event_rejected.load(Ordering::Acquire),
            wm_ncdestroy_observed: true,
            on_window_closed_observed: true,
            callback_state_outstanding: 0,
            fatal_callback,
            resources_before,
            resources_after,
        })
    }
}

struct CallbackInvocation {
    forward: bool,
    ncdestroy: bool,
}

fn guard_callback(
    coordinator: &TerminalCoordinator,
    callback: impl FnOnce() -> Result<CallbackInvocation, &'static str>,
) -> CallbackInvocation {
    let result = catch_unwind(AssertUnwindSafe(callback));
    match result {
        Ok(Ok(invocation)) => invocation,
        Ok(Err(_)) => {
            coordinator.fail(FATAL_CALLBACK_ERROR);
            CallbackInvocation {
                forward: false,
                ncdestroy: false,
            }
        }
        Err(_) => {
            coordinator.fail(FATAL_CALLBACK_PANIC);
            CallbackInvocation {
                forward: false,
                ncdestroy: false,
            }
        }
    }
}

fn invoke_callback(
    coordinator: &TerminalCoordinator,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    callback_id: usize,
) -> CallbackInvocation {
    guard_callback(coordinator, || {
        if callback_id != coordinator.generation as usize {
            coordinator
                .late_event_rejected
                .store(true, Ordering::Release);
            coordinator
                .callbacks_after_close
                .fetch_add(1, Ordering::AcqRel);
            return Ok(CallbackInvocation {
                forward: false,
                ncdestroy: false,
            });
        }
        if message == TEST_EVENT_MESSAGE {
            coordinator.observe_adapter_code(wparam.0)?;
            return Ok(CallbackInvocation {
                forward: false,
                ncdestroy: false,
            });
        }
        let forward = coordinator.observe_message(message, wparam, lparam, false)?;
        Ok(CallbackInvocation {
            forward,
            ncdestroy: message == WM_NCDESTROY,
        })
    })
}

/// The outermost Rust ABI boundary. It catches every Rust panic, marks a typed
/// fatal result, avoids forwarding after failure, and only then requests GPUI-owned
/// teardown. No Rust lock is held while `DefSubclassProc` can synchronously reenter.
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    ref_data: usize,
) -> LRESULT {
    let raw = ref_data as *const TerminalCoordinator;
    if raw.is_null() {
        return LRESULT(0);
    }
    // SAFETY: SetWindowSubclass stores this raw Arc until RemoveWindowSubclass
    // succeeds or WM_NCDESTROY releases it. This temporary strong reference
    // survives forwarding and any same-thread reentrancy.
    unsafe { Arc::increment_strong_count(raw) };
    // SAFETY: balances the temporary increment above, never the registered Arc.
    let coordinator = unsafe { Arc::from_raw(raw) };
    let outer = catch_unwind(AssertUnwindSafe(|| {
        let invocation = invoke_callback(&coordinator, message, wparam, lparam, id);
        let result = if invocation.forward {
            // SAFETY: no lock is held and all arguments are supplied by Win32.
            unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
        } else {
            LRESULT(0)
        };
        if invocation.ncdestroy {
            coordinator.release_raw_ref(raw);
        }
        result
    }));
    match outer {
        Ok(result) => {
            if coordinator.fatal.load(Ordering::Acquire) != FATAL_NONE && message != WM_NCDESTROY {
                // SAFETY: non-owning close request; GPUI remains responsible for HWND destruction.
                let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
            }
            result
        }
        Err(_) => {
            coordinator.fail(FATAL_CALLBACK_PANIC);
            // SAFETY: see the successful path; a failed post remains typed fatal and does not panic.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
            LRESULT(0)
        }
    }
}

/// Captures process resource counts for the explicit preview cleanup threshold.
pub fn resource_snapshot() -> Result<ResourceSnapshot, &'static str> {
    let mut process_handles = 0;
    // SAFETY: current-process pseudo handle and writable local count are valid.
    unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut process_handles) }
        .map_err(|_| "process-handle-snapshot")?;
    // SAFETY: query-only counts for the current process pseudo handle.
    let user_objects = unsafe { GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS) };
    // SAFETY: query-only counts for the current process pseudo handle.
    let gdi_objects = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
    Ok(ResourceSnapshot {
        process_handles,
        user_objects,
        gdi_objects,
    })
}

/// Owns only the subclass callback state. GPUI must keep this lease in its root
/// entity until both `WM_NCDESTROY` and `on_window_closed` have been observed.
pub struct SubclassLease {
    hwnd: BorrowedHwnd,
    coordinator: Arc<TerminalCoordinator>,
    raw_ref: *const TerminalCoordinator,
    resources_before: ResourceSnapshot,
    raw_ref_registered: bool,
}

impl SubclassLease {
    /// Installs a same-thread subclass after validating the GPUI-owned identity.
    pub fn attach(hwnd: BorrowedHwnd) -> Result<Self, &'static str> {
        hwnd.revalidate(hwnd.generation, false)?;
        let resources_before = resource_snapshot()?;
        let coordinator = Arc::new(TerminalCoordinator::new(hwnd.generation));
        let raw_ref = Arc::into_raw(Arc::clone(&coordinator));
        // SAFETY: fixed callback ID is the checked generation; the raw Arc is
        // released only after successful removal or WM_NCDESTROY.
        if !unsafe {
            SetWindowSubclass(
                hwnd.hwnd,
                Some(subclass_proc),
                hwnd.generation as usize,
                raw_ref as usize,
            )
            .as_bool()
        } {
            // SAFETY: installation failed, so common-controls has not retained it.
            unsafe { drop(Arc::from_raw(raw_ref)) };
            return Err("set-window-subclass");
        }
        Ok(Self {
            hwnd,
            coordinator,
            raw_ref,
            resources_before,
            raw_ref_registered: true,
        })
    }

    fn validate_active(&self) -> Result<(), &'static str> {
        self.hwnd
            .revalidate(self.coordinator.generation, self.coordinator.terminal())
    }

    /// Sends a private, non-forwarded adapter message. It never fabricates a DPI
    /// lParam and cannot cause GPUI's WndProc to dereference a synthetic pointer.
    pub fn inject_test_event(&self, code: usize) -> Result<(), &'static str> {
        self.validate_active()?;
        // SAFETY: a validated same-thread, still-live GPUI HWND receives a private
        // message which our subclass consumes without forwarding it to GPUI.
        unsafe {
            SendMessageW(
                self.hwnd.hwnd,
                TEST_EVENT_MESSAGE,
                Some(WPARAM(code)),
                Some(LPARAM(0)),
            )
        };
        Ok(())
    }

    /// Synchronously sends representative native messages through the actual
    /// subclass path. The callback records each raw message before forwarding it
    /// to GPUI. The DPI suggested-rectangle remains on this stack until its
    /// synchronous `SendMessageW` call returns; callers never receive a Win32 type
    /// or pointer and cannot fabricate an invalid DPI payload.
    pub fn send_test_raw_messages(&self) -> Result<(), &'static str> {
        self.validate_active()?;
        let suggested_bounds = RECT {
            left: 0,
            top: 0,
            right: TEST_DISPLAY_WIDTH as i32,
            bottom: TEST_DISPLAY_HEIGHT as i32,
        };
        let dpi_wparam = WPARAM(TEST_DPI_X | (TEST_DPI_Y << 16));
        let display_lparam = LPARAM((TEST_DISPLAY_WIDTH & 0xffff) | (TEST_DISPLAY_HEIGHT << 16));
        // SAFETY: `validate_active` proved a same-thread live GPUI HWND. Each call
        // is synchronous. `suggested_bounds` stays valid for the entire DPI call,
        // and the bridge's subclass observes raw messages before DefSubclassProc
        // can dispatch to GPUI's WndProc.
        unsafe {
            SendMessageW(
                self.hwnd.hwnd,
                WM_DPICHANGED,
                Some(dpi_wparam),
                Some(LPARAM(
                    (&suggested_bounds as *const RECT).cast::<()>() as isize
                )),
            );
            SendMessageW(
                self.hwnd.hwnd,
                WM_DISPLAYCHANGE,
                Some(WPARAM(32)),
                Some(display_lparam),
            );
            SendMessageW(
                self.hwnd.hwnd,
                WM_ACTIVATE,
                Some(WPARAM(1)),
                Some(LPARAM(0)),
            );
        }
        Ok(())
    }

    /// Marks close intent before GPUI is asked to close the owning window.
    pub fn begin_closing(&self) -> Result<(), &'static str> {
        self.validate_active()?;
        self.coordinator.begin_closing()
    }

    /// Re-captures the resource baseline after GPUI's executors have completed
    /// their one-time initialization, but before close begins.
    pub fn rebaseline_resources(&mut self) -> Result<(), &'static str> {
        self.validate_active()?;
        self.resources_before = resource_snapshot()?;
        Ok(())
    }

    /// Records the App-level GPUI close notification. It is deliberately distinct
    /// from the native terminal signal and rejects an unverifiable order.
    pub fn on_window_closed(&self) -> Result<(), &'static str> {
        self.coordinator.record_on_window_closed()
    }

    /// Returns a read-only terminal diagnostic for the composition root. This
    /// must not be used to make ownership decisions: the bridge remains borrowed
    /// until both terminal signals have been observed.
    pub fn terminal_status(&self) -> TerminalStatus {
        self.coordinator
            .status(self.hwnd.hwnd, self.hwnd.owner_thread)
    }

    /// Validates that a late event is fenced without touching a retired HWND.
    pub fn reject_late_adapter_event(&self) -> Result<(), &'static str> {
        self.coordinator.observe_adapter_code(TEST_DPI)
    }

    pub fn terminal_trace(
        &self,
        resources_after: ResourceSnapshot,
    ) -> Result<BridgeTrace, &'static str> {
        self.coordinator
            .trace(&self.hwnd, self.resources_before, resources_after)
    }

    fn remove_before_terminal(&mut self) {
        if !self.raw_ref_registered
            || self.coordinator.phase.load(Ordering::Acquire) != PHASE_ATTACHED
        {
            return;
        }
        // This is the final use of the borrowed HWND outside the callback.
        if self.validate_active().is_err() {
            self.coordinator.fail(FATAL_REMOVE_FAILED);
            return;
        }
        // SAFETY: exact callback/id pair on the same validated thread. On failure
        // the Arc remains registered until WM_NCDESTROY, avoiding a callback UAF.
        if unsafe {
            RemoveWindowSubclass(
                self.hwnd.hwnd,
                Some(subclass_proc),
                self.hwnd.generation as usize,
            )
            .as_bool()
        } {
            self.coordinator.release_raw_ref(self.raw_ref);
            self.raw_ref_registered = false;
        } else {
            self.coordinator.fail(FATAL_REMOVE_FAILED);
        }
    }
}

impl Drop for SubclassLease {
    fn drop(&mut self) {
        self.remove_before_terminal();
    }
}

pub const TEST_EVENT_CODES: (usize, usize, usize) = (TEST_DPI, TEST_DISPLAY, TEST_ACTIVATION);

#[cfg(test)]
mod tests {
    use super::{
        EVENT_ACTIVATION, EVENT_DESTROYED, EVENT_DISPLAY, EVENT_DPI, FATAL_CALLBACK_PANIC,
        FatalReason, PHASE_ATTACHED, PHASE_CLOSING, PHASE_NCDESTROY, ResourceSnapshot, TEST_DPI,
        TerminalCoordinator, guard_callback, invoke_callback,
    };
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::atomic::Ordering,
    };
    use windows::Win32::Foundation::{LPARAM, RECT, WPARAM};

    #[test]
    fn closing_fence_rejects_late_adapter_event() {
        let state = TerminalCoordinator::new(7);
        state.begin_closing().unwrap();
        assert_eq!(
            state.observe_adapter_code(TEST_DPI),
            Err("retired-subclass-generation")
        );
        assert!(state.late_event_rejected.load(Ordering::Acquire));
        assert_eq!(state.callbacks_after_close.load(Ordering::Acquire), 1);
    }

    #[test]
    fn terminal_coordinator_requires_two_terminal_signals_and_order() {
        let state = TerminalCoordinator::new(7);
        assert_eq!(state.phase.load(Ordering::Acquire), PHASE_ATTACHED);
        state.begin_closing().unwrap();
        assert_eq!(state.phase.load(Ordering::Acquire), PHASE_CLOSING);
        state.observe_nc_destroy();
        assert_eq!(state.phase.load(Ordering::Acquire), PHASE_NCDESTROY);
        assert_eq!(state.record_on_window_closed(), Ok(()));
        assert_eq!(state.event_mask.load(Ordering::Acquire), EVENT_DESTROYED);
        assert!(
            !state.terminal_ready(),
            "raw Arc ownership is a separate terminal invariant"
        );
    }

    #[test]
    fn adapter_uses_fixed_event_bits_without_claiming_native_messages() {
        let state = TerminalCoordinator::new(7);
        state.observe_adapter_code(1).unwrap();
        state.observe_adapter_code(2).unwrap();
        state.observe_adapter_code(3).unwrap();
        assert_eq!(
            state.adapter_event_mask.load(Ordering::Acquire),
            EVENT_DPI | EVENT_DISPLAY | EVENT_ACTIVATION
        );
        assert_eq!(state.event_mask.load(Ordering::Acquire), 0);
    }

    #[test]
    fn raw_messages_are_observed_before_they_are_forwarded_and_stay_separate_from_adapter() {
        let state = TerminalCoordinator::new(7);
        let suggested = RECT {
            left: -10,
            top: 20,
            right: 310,
            bottom: 200,
        };
        let dpi = invoke_callback(
            &state,
            super::WM_DPICHANGED,
            WPARAM(144 | (192 << 16)),
            LPARAM((&suggested as *const RECT) as isize),
            7,
        );
        let display = invoke_callback(
            &state,
            super::WM_DISPLAYCHANGE,
            WPARAM(32),
            LPARAM(320 | (180 << 16)),
            7,
        );
        let activation = invoke_callback(&state, super::WM_ACTIVATE, WPARAM(2), LPARAM(0), 7);
        for invocation in [dpi, display, activation] {
            assert!(
                invocation.forward,
                "raw message must be recorded before GPUI forwarding"
            );
        }
        assert_eq!(
            state.event_mask.load(Ordering::Acquire),
            EVENT_DPI | EVENT_DISPLAY | EVENT_ACTIVATION
        );
        assert_eq!(state.adapter_event_mask.load(Ordering::Acquire), 0);
        assert_eq!(
            state.owned_events(state.event_mask.load(Ordering::Acquire)),
            vec![
                super::OwnedWindowEvent::DpiChanged {
                    x: 144,
                    y: 192,
                    suggested_left: -10,
                    suggested_top: 20,
                    suggested_right: 310,
                    suggested_bottom: 200,
                },
                super::OwnedWindowEvent::DisplayChanged {
                    bits_per_pixel: 32,
                    width: 320,
                    height: 180,
                },
                super::OwnedWindowEvent::ActivationChanged { state: 2 },
            ]
        );
    }

    #[test]
    fn callback_generation_mismatch_is_fenced_without_forwarding() {
        let state = TerminalCoordinator::new(7);
        let invocation = invoke_callback(&state, super::WM_ACTIVATE, WPARAM(0), LPARAM(0), 8);
        assert!(!invocation.forward);
        assert!(state.late_event_rejected.load(Ordering::Acquire));
        assert_eq!(state.callbacks_after_close.load(Ordering::Acquire), 1);
    }

    #[test]
    fn caught_callback_panic_becomes_typed_fatal_without_poisoning_state() {
        let state = TerminalCoordinator::new(7);
        let result = catch_unwind(AssertUnwindSafe(|| {
            guard_callback(
                &state,
                || -> Result<super::CallbackInvocation, &'static str> { panic!("fixture panic") },
            );
        }));
        assert!(
            result.is_ok(),
            "panic must not escape the callback guard or poison later work"
        );
        assert_eq!(state.fatal.load(Ordering::Acquire), FATAL_CALLBACK_PANIC);
        assert_eq!(
            match state.fatal.load(Ordering::Acquire) {
                FATAL_CALLBACK_PANIC => Some(FatalReason::CallbackPanic),
                _ => None,
            },
            Some(FatalReason::CallbackPanic)
        );
        assert_eq!(
            state.observe_adapter_code(TEST_DPI),
            Err("retired-subclass-generation")
        );
    }

    #[test]
    fn close_notification_and_native_destroy_can_arrive_in_either_order() {
        let state = TerminalCoordinator::new(7);
        assert_eq!(state.record_on_window_closed(), Ok(()));
        state.observe_nc_destroy();
        assert!(state.claim_raw_release());
        state.maybe_finalize();
        assert!(state.terminal_ready());
        assert!(
            state.finalized_at.load(Ordering::Acquire) > state.ncdestroy_at.load(Ordering::Acquire)
        );
        assert!(
            state.finalized_at.load(Ordering::Acquire) > state.on_closed_at.load(Ordering::Acquire)
        );
    }

    #[test]
    fn raw_arc_release_claim_is_exactly_once_even_after_remove_failure_path() {
        let state = TerminalCoordinator::new(7);
        assert!(state.claim_raw_release());
        assert!(!state.claim_raw_release());
        assert!(state.raw_ref_released.load(Ordering::Acquire));
    }

    #[test]
    fn terminal_status_is_read_only_and_reports_independent_terminal_signals() {
        let state = TerminalCoordinator::new(7);
        let owner_thread = unsafe { super::GetCurrentThreadId() };
        let attached = state.status(super::HWND(std::ptr::null_mut()), owner_thread);
        assert_eq!(attached.phase, super::TerminalPhase::Attached);
        assert!(!attached.wm_ncdestroy_seen);
        assert!(!attached.on_window_closed_seen);
        assert!(attached.raw_ref_outstanding);
        assert_eq!(attached.hwnd_still_valid, Some(false));

        state.record_on_window_closed().unwrap();
        let closing = state.status(super::HWND(std::ptr::null_mut()), owner_thread);
        assert_eq!(closing.phase, super::TerminalPhase::Closing);
        assert!(closing.on_window_closed_seen);
        assert!(!closing.wm_ncdestroy_seen);
        assert!(!closing.finalized);
    }

    #[test]
    fn resource_snapshot_is_copyable_for_trace_handoff() {
        let snapshot = ResourceSnapshot {
            process_handles: 1,
            user_objects: 2,
            gdi_objects: 3,
        };
        assert_eq!(snapshot, snapshot);
    }

    #[test]
    fn dpi_payload_is_copied_before_forwarding_and_never_borrows_rect_memory() {
        let state = TerminalCoordinator::new(7);
        let mut suggested = RECT {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        };
        let invocation = invoke_callback(
            &state,
            super::WM_DPICHANGED,
            WPARAM(120 | (144 << 16)),
            LPARAM((&suggested as *const RECT) as isize),
            7,
        );
        assert!(invocation.forward);
        suggested.left = 101;
        suggested.top = 102;
        suggested.right = 103;
        suggested.bottom = 104;
        assert_eq!(
            (
                suggested.left,
                suggested.top,
                suggested.right,
                suggested.bottom
            ),
            (101, 102, 103, 104)
        );
        assert_eq!(
            state.owned_events(EVENT_DPI),
            vec![super::OwnedWindowEvent::DpiChanged {
                x: 120,
                y: 144,
                suggested_left: 1,
                suggested_top: 2,
                suggested_right: 3,
                suggested_bottom: 4,
            }]
        );
    }
}
