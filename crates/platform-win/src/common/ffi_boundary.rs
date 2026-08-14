//! Shared no-unwind guard for every SuperDesktop-owned Win32 callback boundary.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoUnwind<T> {
    Returned(T),
    Panicked,
}

/// Runs Rust callback work behind a catch-unwind fence. An `extern "system"`
/// entry point must translate `Panicked` to a typed fatal state before return.
pub fn catch_no_unwind<T>(callback: impl FnOnce() -> T) -> NoUnwind<T> {
    match catch_unwind(AssertUnwindSafe(callback)) {
        Ok(value) => NoUnwind::Returned(value),
        Err(_) => NoUnwind::Panicked,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FfiFatal {
    None = 0,
    CallbackPanic = 1,
    ReentrantCallback = 2,
    ShutdownRace = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackResult<T> {
    Returned(T),
    Rejected(FfiFatal),
}

pub struct CallbackFence {
    in_flight: AtomicBool,
    shutdown: AtomicBool,
    fatal: AtomicU8,
    entered: AtomicUsize,
    completed: AtomicUsize,
}

impl Default for CallbackFence {
    fn default() -> Self {
        Self {
            in_flight: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            fatal: AtomicU8::new(FfiFatal::None as u8),
            entered: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
        }
    }
}

struct InFlight<'a>(&'a AtomicBool);
impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl CallbackFence {
    fn reject(&self, fatal: FfiFatal) -> CallbackResult<()> {
        let _ = self.fatal.compare_exchange(
            FfiFatal::None as u8,
            fatal as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        CallbackResult::Rejected(fatal)
    }

    pub fn invoke<T>(&self, callback: impl FnOnce() -> T) -> CallbackResult<T> {
        if self.shutdown.load(Ordering::Acquire) {
            self.reject(FfiFatal::ShutdownRace);
            return CallbackResult::Rejected(FfiFatal::ShutdownRace);
        }
        if self.in_flight.swap(true, Ordering::AcqRel) {
            self.reject(FfiFatal::ReentrantCallback);
            return CallbackResult::Rejected(FfiFatal::ReentrantCallback);
        }
        let _in_flight = InFlight(&self.in_flight);
        self.entered.fetch_add(1, Ordering::AcqRel);
        match catch_no_unwind(callback) {
            NoUnwind::Returned(value) => {
                self.completed.fetch_add(1, Ordering::AcqRel);
                CallbackResult::Returned(value)
            }
            NoUnwind::Panicked => {
                self.reject(FfiFatal::CallbackPanic);
                CallbackResult::Rejected(FfiFatal::CallbackPanic)
            }
        }
    }

    pub fn begin_shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    pub fn fatal(&self) -> FfiFatal {
        match self.fatal.load(Ordering::Acquire) {
            1 => FfiFatal::CallbackPanic,
            2 => FfiFatal::ReentrantCallback,
            3 => FfiFatal::ShutdownRace,
            _ => FfiFatal::None,
        }
    }

    pub fn counts(&self) -> (usize, usize) {
        (
            self.entered.load(Ordering::Acquire),
            self.completed.load(Ordering::Acquire),
        )
    }
}

pub const SPIKE_CALLBACK_ABI: &str =
    "unsafe extern system fn(*const CallbackFence, usize) -> isize";

/// Concrete ABI probe proving that a SuperDesktop-owned `extern system`
/// function translates panic/reentrancy/shutdown into non-unwinding return codes.
///
/// # Safety
/// `context` must reference a live `CallbackFence` for this synchronous call.
pub unsafe extern "system" fn spike_callback(context: *const CallbackFence, mode: usize) -> isize {
    let Some(fence) = (unsafe { context.as_ref() }) else {
        return -4;
    };
    let result = fence.invoke(|| match mode {
        1 => panic!("deliberate ffi callback panic"),
        2 => {
            let _ = fence.invoke(|| 0);
        }
        _ => {}
    });
    match result {
        CallbackResult::Returned(()) if fence.fatal() == FfiFatal::None => 0,
        CallbackResult::Returned(()) | CallbackResult::Rejected(_) => -(fence.fatal() as isize),
    }
}

#[cfg(test)]
mod tests {
    use super::{CallbackFence, FfiFatal, NoUnwind, catch_no_unwind, spike_callback};

    #[test]
    fn catches_panic_and_allows_later_callback() {
        assert_eq!(catch_no_unwind(|| panic!("fixture")), NoUnwind::Panicked);
        assert_eq!(catch_no_unwind(|| 7), NoUnwind::Returned(7));
    }

    #[test]
    fn extern_system_callback_types_panic_without_unwind() {
        let fence = CallbackFence::default();
        assert_eq!(unsafe { spike_callback(&fence, 1) }, -1);
        assert_eq!(fence.fatal(), FfiFatal::CallbackPanic);
        assert_eq!(fence.counts(), (1, 0));
    }

    #[test]
    fn double_callback_and_shutdown_race_are_fenced() {
        let double = CallbackFence::default();
        assert_eq!(unsafe { spike_callback(&double, 2) }, -2);
        assert_eq!(double.fatal(), FfiFatal::ReentrantCallback);
        assert_eq!(double.counts(), (1, 1));

        let shutdown = CallbackFence::default();
        shutdown.begin_shutdown();
        assert_eq!(unsafe { spike_callback(&shutdown, 0) }, -3);
        assert_eq!(shutdown.fatal(), FfiFatal::ShutdownRace);
        assert_eq!(shutdown.counts(), (0, 0));
    }
}
