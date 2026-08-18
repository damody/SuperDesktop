#![windows_subsystem = "windows"]

use std::{
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    thread,
    time::{Duration, Instant},
};

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
                NIM_SETFOCUS, NIM_SETVERSION, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, IDI_INFORMATION,
                LoadIconW, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, RegisterWindowMessageW,
                TranslateMessage, UnregisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
                WM_CLOSE, WM_DESTROY, WNDCLASSW,
            },
        },
    },
    core::w,
};

const CALLBACK_MESSAGE: u32 = WM_APP + 37;
static TASKBAR_CREATED_MESSAGE: AtomicU32 = AtomicU32::new(0);
static REREGISTER: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn fixture_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == TASKBAR_CREATED_MESSAGE.load(Ordering::Acquire) {
        REREGISTER.store(true, Ordering::Release);
        println!("taskbar-created");
        return LRESULT(0);
    }
    match message {
        CALLBACK_MESSAGE => {
            println!("callback wparam={} lparam={}", wparam.0, lparam.0);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn set_tip(data: &mut NOTIFYICONDATAW, value: &str) {
    data.szTip.fill(0);
    for (destination, source) in data.szTip.iter_mut().zip(value.encode_utf16()) {
        *destination = source;
    }
}

fn main() {
    let hold_ms = std::env::args()
        .skip_while(|arg| arg != "--hold-ms")
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(12_000)
        .clamp(1_000, 30_000);
    let instance = HINSTANCE::default();
    TASKBAR_CREATED_MESSAGE.store(
        unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
        Ordering::Release,
    );
    let class_name = w!("SuperDesktopNotifyIconFixture");
    let class = WNDCLASSW {
        lpfnWndProc: Some(fixture_proc),
        hInstance: instance,
        lpszClassName: class_name,
        ..WNDCLASSW::default()
    };
    assert_ne!(
        unsafe { RegisterClassW(&class) },
        0,
        "register fixture class"
    );
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("SuperDesktop NotifyIcon fixture"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .expect("create fixture window");
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 7001,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: CALLBACK_MESSAGE,
        hIcon: unsafe { LoadIconW(None, IDI_INFORMATION) }.expect("load fixture icon"),
        ..NOTIFYICONDATAW::default()
    };
    set_tip(&mut data, "SuperDesktop compatibility fixture");
    assert!(
        unsafe { Shell_NotifyIconW(NIM_ADD, &raw const data) }.as_bool(),
        "NIM_ADD"
    );
    data.Anonymous.uVersion = 4;
    assert!(
        unsafe { Shell_NotifyIconW(NIM_SETVERSION, &raw const data) }.as_bool(),
        "NIM_SETVERSION"
    );
    set_tip(&mut data, "SuperDesktop compatibility fixture modified");
    assert!(
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &raw const data) }.as_bool(),
        "NIM_MODIFY"
    );
    println!(
        "fixture-ready pid={} hwnd={} icon_id={}",
        std::process::id(),
        hwnd.0 as isize,
        data.uID
    );
    let deadline = Instant::now() + Duration::from_millis(hold_ms);
    let mut message = MSG::default();
    while Instant::now() < deadline {
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if REREGISTER.swap(false, Ordering::AcqRel) {
            assert!(unsafe { Shell_NotifyIconW(NIM_ADD, &raw const data) }.as_bool());
            data.Anonymous.uVersion = 4;
            assert!(unsafe { Shell_NotifyIconW(NIM_SETVERSION, &raw const data) }.as_bool());
            assert!(unsafe { Shell_NotifyIconW(NIM_MODIFY, &raw const data) }.as_bool());
            println!("fixture-reregistered");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = unsafe { Shell_NotifyIconW(NIM_SETFOCUS, &raw const data) };
    assert!(
        unsafe { Shell_NotifyIconW(NIM_DELETE, &raw const data) }.as_bool(),
        "NIM_DELETE"
    );
    let _ = unsafe { DestroyWindow(hwnd) };
    let _ = unsafe { UnregisterClassW(class_name, Some(instance)) };
    println!("fixture-complete");
}
