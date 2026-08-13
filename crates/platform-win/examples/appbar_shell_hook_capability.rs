//! Controlled, reversible AppBar/Shell Hook capability runner.
//!
//! It creates two short-lived process-owned tool windows: one is the only AppBar
//! and Shell Hook target, the other induces a controlled activation event. It never
//! obtains, hides, moves, closes, or otherwise mutates an Explorer-owned window.

use std::{
    mem::zeroed,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::atomic::{AtomicU32, Ordering},
    thread,
    time::{Duration, Instant},
};

use platform_win::common::appbar_shell_hook::{ControlledShellCapability, ScreenRect};
use platform_win::common::native_window::{ResourceSnapshot, resource_snapshot};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG, PM_REMOVE,
            PeekMessageW, RegisterClassW, SPI_GETWORKAREA, SW_SHOWNOACTIVATE, SetForegroundWindow,
            ShowWindow, SystemParametersInfoW, TranslateMessage, WNDCLASSW, WS_EX_TOOLWINDOW,
            WS_OVERLAPPEDWINDOW, WS_POPUP,
        },
    },
    core::w,
};

static SHELL_HOOK_MESSAGE: AtomicU32 = AtomicU32::new(0);
static SHELL_HOOK_EVENTS: AtomicU32 = AtomicU32::new(0);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> windows::Win32::Foundation::HINSTANCE;
}

unsafe extern "system" fn controlled_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if message != 0 && message == SHELL_HOOK_MESSAGE.load(Ordering::Acquire) {
            SHELL_HOOK_EVENTS.fetch_add(1, Ordering::AcqRel);
        }
        // SAFETY: forwards only the OS-supplied message to the default procedure.
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }));
    result.unwrap_or(LRESULT(0))
}

fn work_area() -> Result<ScreenRect, String> {
    let mut rect = RECT::default();
    // SAFETY: output storage is valid and the call is query-only.
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut rect as *mut RECT).cast()),
            Default::default(),
        )
    }
    .map_err(|error| format!("spi-getworkarea:{error}"))?;
    Ok(ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

fn create_controlled_window(
    class_name: windows::core::PCWSTR,
    shell_hook_stimulus: bool,
) -> Result<HWND, String> {
    // SAFETY: retrieves this module's handle without mutable process state.
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    if module.is_invalid() {
        return Err("module-handle".to_string());
    }
    let class = WNDCLASSW {
        lpfnWndProc: Some(controlled_wndproc),
        hInstance: module,
        lpszClassName: class_name,
        ..Default::default()
    };
    // SAFETY: class fields are valid static/module values. Existing registration is benign.
    let _ = unsafe { RegisterClassW(&class) };
    // SAFETY: creates a short-lived process-owned tool window; no Explorer HWND is referenced.
    let hwnd = unsafe {
        CreateWindowExW(
            if shell_hook_stimulus {
                Default::default()
            } else {
                WS_EX_TOOLWINDOW
            },
            class_name,
            w!("SuperDesktop capability probe"),
            if shell_hook_stimulus {
                WS_OVERLAPPEDWINDOW
            } else {
                WS_POPUP
            },
            -32_000,
            -32_000,
            1,
            1,
            None,
            None,
            Some(module),
            None,
        )
    }
    .map_err(|error| format!("controlled-window-create:{error}"))?;
    if shell_hook_stimulus {
        // SAFETY: shows only this 1x1, off-screen process-owned test window. It
        // exists solely to request a real shell hook lifecycle notification.
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    }
    Ok(hwnd)
}

fn pump_messages(deadline: Duration) {
    let until = Instant::now() + deadline;
    while Instant::now() < until {
        // SAFETY: MSG output is initialized before each nonzero return and dispatches only this thread's queue.
        unsafe {
            let mut message = zeroed::<MSG>();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn json_rect(value: ScreenRect) -> String {
    format!(
        "{{\"left\":{},\"top\":{},\"right\":{},\"bottom\":{}}}",
        value.left, value.top, value.right, value.bottom
    )
}

fn json_resources(value: ResourceSnapshot) -> String {
    format!(
        "{{\"process_handles\":{},\"user_objects\":{},\"gdi_objects\":{}}}",
        value.process_handles, value.user_objects, value.gdi_objects
    )
}

struct RunData {
    reserved: ScreenRect,
    shell_hook_message: u32,
    shell_hook_events: u32,
    after_work_area: ScreenRect,
    failure_injection_rejected: bool,
    unregister_events_before_helper: u32,
    unregister_events_after_helper: u32,
}

struct MidFailureData {
    after_work_area: ScreenRect,
    typed_failure: &'static str,
    appbar_removed: bool,
}

fn run_lifecycle(before_work_area: ScreenRect) -> Result<RunData, String> {
    SHELL_HOOK_EVENTS.store(0, Ordering::Release);
    let primary =
        create_controlled_window(w!("SuperDesktop.AppBarShellHookCapability.Primary"), false)?;
    let outcome = (|| -> Result<RunData, String> {
        let failure_injection_rejected =
            ControlledShellCapability::attach_controlled_window(1).is_err();
        if !failure_injection_rejected {
            return Err("failure-injection-was-not-rejected".to_string());
        }
        let mut capability =
            ControlledShellCapability::attach_controlled_window(primary.0 as isize)
                .map_err(str::to_owned)?;
        capability.register_appbar().map_err(str::to_owned)?;
        let reserved = capability
            .reserve_bottom(before_work_area, 1)
            .map_err(str::to_owned)?;
        let shell_hook_message = capability.register_shell_hook().map_err(str::to_owned)?;
        SHELL_HOOK_MESSAGE.store(shell_hook_message, Ordering::Release);
        let stimulus =
            create_controlled_window(w!("SuperDesktop.AppBarShellHookCapability.Stimulus"), true)?;
        // SAFETY: activates only a process-owned test HWND to induce a hook event.
        let _ = unsafe { SetForegroundWindow(stimulus) };
        pump_messages(Duration::from_millis(500));
        let hook_events = SHELL_HOOK_EVENTS.load(Ordering::Acquire);
        // SAFETY: destroys only the just-created process-owned test HWND.
        unsafe {
            DestroyWindow(stimulus).map_err(|error| format!("stimulus-destroy:{error}"))?;
        }
        // Drain the controlled helper's destroy notification while the hook is
        // still registered. The post-unregister fence must not misclassify an
        // already-queued pre-unregister delivery as a later callback.
        pump_messages(Duration::from_millis(250));
        let first_teardown = capability.teardown();
        let second_teardown = capability.teardown();
        let unregister_events_before_helper = SHELL_HOOK_EVENTS.load(Ordering::Acquire);
        let post_unregister_helper =
            create_controlled_window(w!("SuperDesktop.AppBarShellHookCapability.Stimulus"), true)?;
        pump_messages(Duration::from_millis(250));
        let unregister_events_after_helper = SHELL_HOOK_EVENTS.load(Ordering::Acquire);
        // SAFETY: destroys only the post-unregister process-owned helper.
        unsafe {
            DestroyWindow(post_unregister_helper)
                .map_err(|error| format!("post-unregister-helper-destroy:{error}"))?;
        }
        let after_work_area = work_area()?;
        if !first_teardown.appbar_removed || !first_teardown.shell_hook_unregistered {
            return Err("controlled-teardown-incomplete".to_string());
        }
        if second_teardown.appbar_removed || second_teardown.shell_hook_unregistered {
            return Err("controlled-teardown-not-idempotent".to_string());
        }
        if before_work_area != after_work_area {
            return Err("work-area-not-restored".to_string());
        }
        if hook_events == 0 {
            return Err("shell-hook-event-not-observed".to_string());
        }
        if unregister_events_before_helper != unregister_events_after_helper {
            return Err("shell-hook-event-after-unregister".to_string());
        }
        Ok(RunData {
            reserved,
            shell_hook_message,
            shell_hook_events: hook_events,
            after_work_area,
            failure_injection_rejected,
            unregister_events_before_helper,
            unregister_events_after_helper,
        })
    })();
    // SAFETY: capability teardown is attempted by both the explicit call and Drop;
    // this destroys only the caller-created primary window afterwards.
    unsafe {
        DestroyWindow(primary).map_err(|error| format!("primary-destroy:{error}"))?;
    }
    pump_messages(Duration::from_millis(100));
    outcome
}

/// Deliberately fails only after AppBar registration and reservation have
/// succeeded, but before Shell Hook registration. It exercises the identical
/// lease teardown path as a normal lifecycle and never treats an invalid HWND as
/// a substitute for this mutation-stage failure.
fn run_mid_failure_lifecycle(before_work_area: ScreenRect) -> Result<MidFailureData, String> {
    let primary =
        create_controlled_window(w!("SuperDesktop.AppBarShellHookCapability.Primary"), false)?;
    let outcome = (|| -> Result<MidFailureData, String> {
        let mut capability =
            ControlledShellCapability::attach_controlled_window(primary.0 as isize)
                .map_err(str::to_owned)?;
        capability.register_appbar().map_err(str::to_owned)?;
        let _ = capability
            .reserve_bottom(before_work_area, 1)
            .map_err(str::to_owned)?;
        let typed_failure = "injected-after-reserve-before-shell-hook";
        let teardown = capability.teardown();
        let after_work_area = work_area()?;
        if !teardown.appbar_removed || teardown.shell_hook_unregistered {
            return Err("mid-failure-teardown-incomplete".to_string());
        }
        if after_work_area != before_work_area {
            return Err("mid-failure-work-area-not-restored".to_string());
        }
        Ok(MidFailureData {
            after_work_area,
            typed_failure,
            appbar_removed: teardown.appbar_removed,
        })
    })();
    // SAFETY: destroys only the caller-created controlled failure fixture HWND.
    unsafe {
        DestroyWindow(primary).map_err(|error| format!("mid-failure-primary-destroy:{error}"))?;
    }
    pump_messages(Duration::from_millis(100));
    outcome
}

fn run() -> Result<String, String> {
    // Warm the two private window classes before the resource baseline. USER/GDI
    // lazily loads per-process support objects on the first window creation; this
    // makes the measured delta about AppBar/Hook teardown rather than that loader.
    for (class, stimulus) in [
        (w!("SuperDesktop.AppBarShellHookCapability.Primary"), false),
        (w!("SuperDesktop.AppBarShellHookCapability.Stimulus"), true),
    ] {
        let warmup = create_controlled_window(class, stimulus)?;
        // SAFETY: destroys only the just-created process-owned warmup HWND.
        unsafe {
            DestroyWindow(warmup).map_err(|error| format!("warmup-destroy:{error}"))?;
        }
    }
    pump_messages(Duration::from_millis(100));
    let warmup_work_area = work_area()?;
    let _warmup = run_lifecycle(warmup_work_area)?;
    pump_messages(Duration::from_millis(100));
    let before_work_area = work_area()?;
    if before_work_area != warmup_work_area {
        return Err("warmup-work-area-not-restored".to_string());
    }
    let mid_failure_before = resource_snapshot().map_err(str::to_owned)?;
    let mid_failure = run_mid_failure_lifecycle(before_work_area)?;
    let mid_failure_after = resource_snapshot().map_err(str::to_owned)?;
    if mid_failure_before != mid_failure_after {
        return Err("resource-baseline-not-restored-mid-failure".to_string());
    }
    let before_first = resource_snapshot().map_err(str::to_owned)?;
    let _first = run_lifecycle(before_work_area)?;
    let after_first = resource_snapshot().map_err(str::to_owned)?;
    if before_first != after_first {
        return Err("resource-baseline-not-restored-first-lifecycle".to_string());
    }
    let before_second = resource_snapshot().map_err(str::to_owned)?;
    let data = run_lifecycle(work_area()?)?;
    let after_second = resource_snapshot().map_err(str::to_owned)?;
    if before_second != after_second {
        return Err("resource-baseline-not-restored-second-lifecycle".to_string());
    }
    Ok(format!(
        "{{\"schema\":\"appbar-shell-hook-trace/v2\",\"controlled_only\":true,\"warmup_unaccepted\":true,\"appbar_registered\":true,\"reserved_rect\":{},\"shell_hook_message\":{},\"shell_hook_events\":{},\"failure_injection_rejected\":{},\"mid_failure\":{{\"typed_failure\":\"{}\",\"appbar_removed\":{},\"work_area_after\":{},\"resources_before\":{},\"resources_after\":{}}},\"first_teardown\":{{\"appbar_removed\":true,\"shell_hook_unregistered\":true}},\"second_teardown\":{{\"appbar_removed\":false,\"shell_hook_unregistered\":false}},\"unregister_event_fence\":{{\"before_helper\":{},\"after_helper\":{}}},\"work_area_before\":{},\"work_area_after\":{},\"resources_before_first\":{},\"resources_after_first\":{},\"resources_before_second\":{},\"resources_after_second\":{}}}",
        json_rect(data.reserved),
        data.shell_hook_message,
        data.shell_hook_events,
        data.failure_injection_rejected,
        mid_failure.typed_failure,
        mid_failure.appbar_removed,
        json_rect(mid_failure.after_work_area),
        json_resources(mid_failure_before),
        json_resources(mid_failure_after),
        data.unregister_events_before_helper,
        data.unregister_events_after_helper,
        json_rect(before_work_area),
        json_rect(data.after_work_area),
        json_resources(before_first),
        json_resources(after_first),
        json_resources(before_second),
        json_resources(after_second)
    ))
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(trace) => {
            println!("{trace}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{{\"admitted\":false,\"error\":\"{}\"}}",
                error.replace('"', "\\\"")
            );
            std::process::ExitCode::from(2)
        }
    }
}
