#![windows_subsystem = "windows"]

use std::time::Duration;

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::Com::{
            CLSCTX_ALL, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance,
            CoInitializeEx,
        },
        UI::Shell::{
            ITaskbarList3, TBPF_ERROR, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL,
            TBPF_PAUSED, TBPFLAG,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, IsWindow, MSG,
            PM_REMOVE, PeekMessageW, RegisterClassW, SW_MINIMIZE, SW_RESTORE, SW_SHOW, ShowWindow,
            TranslateMessage, WINDOW_EX_STYLE, WM_APP, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
    core::{GUID, PCWSTR, Result},
};

const CLSID_TASKBAR_LIST: GUID = GUID::from_u128(0x56fdf344_fd6d_11d0_958a_006097c9a090);
const WM_FIXTURE_MINIMIZE: u32 = WM_APP + 41;
const WM_FIXTURE_RESTORE: u32 = WM_APP + 42;
const FIXTURE_CLASS: &[u16] = &[
    b'S' as u16,
    b'u' as u16,
    b'p' as u16,
    b'e' as u16,
    b'r' as u16,
    b'D' as u16,
    b'e' as u16,
    b's' as u16,
    b'k' as u16,
    b't' as u16,
    b'o' as u16,
    b'p' as u16,
    b'P' as u16,
    b'r' as u16,
    b'o' as u16,
    b'g' as u16,
    b'r' as u16,
    b'e' as u16,
    b's' as u16,
    b's' as u16,
    0,
];
const FIXTURE_TITLE: &[u16] = &[
    b'T' as u16,
    b'a' as u16,
    b's' as u16,
    b'k' as u16,
    b'b' as u16,
    b'a' as u16,
    b'r' as u16,
    b' ' as u16,
    b'P' as u16,
    b'r' as u16,
    b'o' as u16,
    b'g' as u16,
    b'r' as u16,
    b'e' as u16,
    b's' as u16,
    b's' as u16,
    b' ' as u16,
    b'F' as u16,
    b'i' as u16,
    b'x' as u16,
    b't' as u16,
    b'u' as u16,
    b'r' as u16,
    b'e' as u16,
    0,
];

unsafe extern "system" fn fixture_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_FIXTURE_MINIMIZE => {
            let _ = unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
            return LRESULT(0);
        }
        WM_FIXTURE_RESTORE => {
            let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
            return LRESULT(0);
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn create_fixture_window() -> Result<HWND> {
    let instance = HINSTANCE::default();
    let class = WNDCLASSW {
        lpfnWndProc: Some(fixture_window_proc),
        hInstance: instance,
        lpszClassName: PCWSTR(FIXTURE_CLASS.as_ptr()),
        ..Default::default()
    };
    unsafe { RegisterClassW(&class) };
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(FIXTURE_CLASS.as_ptr()),
            PCWSTR(FIXTURE_TITLE.as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            200,
            200,
            480,
            240,
            None,
            None,
            Some(instance),
            None,
        )?
    };
    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
    Ok(hwnd)
}

fn run() -> Result<()> {
    // SAFETY: initializes COM once on the fixture's main thread.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;
    let args = std::env::args().collect::<Vec<_>>();
    let context = if args.iter().any(|arg| arg == "--local-server") {
        CLSCTX_LOCAL_SERVER
    } else {
        CLSCTX_ALL
    };
    let no_progress = args.iter().any(|arg| arg == "--no-progress");
    // SAFETY: this is the documented ordinary application activation route.
    let taskbar = if no_progress {
        None
    } else {
        let taskbar: ITaskbarList3 =
            unsafe { CoCreateInstance(&CLSID_TASKBAR_LIST, None, context)? };
        unsafe { taskbar.HrInit()? };
        Some(taskbar)
    };
    let hwnd = create_fixture_window()?;
    let state = match args.get(1).map(String::as_str) {
        Some("indeterminate") => TBPF_INDETERMINATE,
        Some("paused") => TBPF_PAUSED,
        Some("error") => TBPF_ERROR,
        Some("none") => TBPF_NOPROGRESS,
        _ => TBPF_NORMAL,
    };
    let percent = args
        .get(2)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(42)
        .min(100);
    let hold_ms = args
        .iter()
        .position(|arg| arg == "--hold-ms")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5_000)
        .clamp(1_000, 60_000);
    // SAFETY: ordinary documented calls targeting the fixture-owned HWND.
    if let Some(taskbar) = &taskbar {
        unsafe {
            taskbar.SetProgressState(hwnd, state)?;
            if determinate(state) {
                taskbar.SetProgressValue(hwnd, percent, 100)?;
            }
        }
    }
    println!(
        "state={} percent={percent} hwnd={:X} minimize_message={WM_FIXTURE_MINIMIZE} restore_message={WM_FIXTURE_RESTORE}",
        state.0, hwnd.0 as usize,
    );
    let deadline = std::time::Instant::now() + Duration::from_millis(hold_ms);
    let mut message = MSG::default();
    while std::time::Instant::now() < deadline && unsafe { IsWindow(Some(hwnd)).as_bool() } {
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if unsafe { IsWindow(Some(hwnd)).as_bool() } {
        if let Some(taskbar) = &taskbar {
            unsafe { taskbar.SetProgressState(hwnd, TBPF_NOPROGRESS)? };
        }
        unsafe { DestroyWindow(hwnd)? };
    }
    Ok(())
}

fn determinate(state: TBPFLAG) -> bool {
    state == TBPF_NORMAL || state == TBPF_PAUSED || state == TBPF_ERROR
}

fn main() {
    if let Err(error) = run() {
        eprintln!("taskbar progress fixture failed: {error}");
        std::process::exit(1);
    }
}
