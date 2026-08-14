//! Headful probe for panic containment in the pinned gpui_windows main WndProc.
//!
//! The child installs a public GPUI window-bounds observer that panics when a
//! real resize reaches the backend's WM_SIZE path. The controller survives an
//! abort and records whether typed-fatal and terminal markers were produced.

use std::{cell::Cell, fs, process::Command, rc::Rc, time::Duration};

use gpui::{
    App, AppContext, Context, IntoElement, Render, Styled, Subscription, Window, WindowBounds,
    WindowOptions, div, px, size,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

struct PanicView {
    _bounds_subscription: Subscription,
}

impl Render for PanicView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

fn argument(name: &str) -> Result<String, String> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == name {
            return arguments.next().ok_or_else(|| format!("missing-{name}"));
        }
    }
    Err(format!("missing-{name}"))
}

fn live_win32_hwnd(window: &Window) -> Result<isize, String> {
    let handle = HasWindowHandle::window_handle(window).map_err(|_| "window-handle")?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        _ => Err("non-win32-window-handle".into()),
    }
}

fn panic_child() -> Result<(), String> {
    let entered = argument("--callback-entered")?;
    let panic_marker = argument("--panic-observed")?;
    let returned = argument("--callback-returned")?;
    let terminal = argument("--terminal")?;
    let fatal = argument("--typed-fatal")?;
    let panic_for_hook = panic_marker.clone();
    std::panic::set_hook(Box::new(move |_info| {
        let _ = fs::write(&panic_for_hook, "panic-hook-observed");
    }));

    let platform = gpui_windows::WindowsPlatform::new(false).map_err(|error| error.to_string())?;
    gpui::Application::with_platform(Rc::new(platform))
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(move |cx: &mut App| {
            let hwnd_slot = Rc::new(Cell::new(0_isize));
            let hwnd_for_window = Rc::clone(&hwnd_slot);
            let entered_for_callback = entered.clone();
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(gpui::Bounds::centered(
                        None,
                        size(px(320.), px(180.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                move |window, cx| {
                    hwnd_for_window.set(live_win32_hwnd(window).expect("live Win32 HWND"));
                    cx.new(|cx| {
                        let subscription = cx.observe_window_bounds(window, move |_, _, _| {
                            fs::write(&entered_for_callback, "public-gpui-bounds-callback-entered")
                                .expect("write callback marker");
                            panic!("deliberate public GPUI callback panic fixture");
                        });
                        PanicView {
                            _bounds_subscription: subscription,
                        }
                    })
                },
            );
            let Ok(handle) = opened else {
                let _ = fs::write(&returned, "open-window-failed");
                cx.quit();
                return;
            };
            let terminal_marker = terminal.clone();
            let window_id = handle.window_id();
            cx.on_window_closed(move |app, closed| {
                if closed == window_id {
                    let _ = fs::write(&terminal_marker, "gpui-window-closed");
                    app.quit();
                }
            })
            .detach();
            let fatal_marker = fatal.clone();
            let fatal_window = handle;
            let fatal_app = cx.to_async();
            let fatal_executor = cx.foreground_executor().clone();
            let mut terminal_bridge = Some((fatal_window, fatal_app));
            gpui_windows::on_callback_fatal(move |event| {
                let _ = fs::write(
                    &fatal_marker,
                    format!(
                        "{{\"type\":\"WindowsCallbackFatal\",\"hwnd\":{},\"message\":{},\"wm_ncdestroy_observed\":{}}}",
                        event.hwnd, event.message, event.wm_ncdestroy_observed
                    ),
                );
                if let Some((fatal_window, mut fatal_app)) = terminal_bridge.take() {
                    fatal_executor
                        .spawn(async move {
                            let _ = fatal_window.update(&mut fatal_app, |_, window, _| {
                                window.remove_window();
                            });
                        })
                        .detach();
                }
            });
            // WindowsPlatform::resize synchronously sends WM_SIZE. The public
            // observer above therefore executes inside the pinned main WndProc.
            let resize = handle.update(cx, |_, window, _| {
                window.resize(size(px(333.), px(197.)));
            });
            let _ = fs::write(&returned, format!("resize-returned:{resize:?}"));
            let timeout_app = cx.to_async();
            let timeout_background = cx.background_executor().clone();
            cx.foreground_executor()
                .spawn(async move {
                    timeout_background.timer(Duration::from_secs(3)).await;
                    timeout_app.update(|app| app.quit());
                })
                .detach();
        });
    Ok(())
}

fn controller() -> Result<(), String> {
    let output = argument("--output")?;
    let root = std::path::Path::new(&output)
        .parent()
        .ok_or_else(|| "output-parent".to_string())?;
    fs::create_dir_all(root).map_err(|_| "output-directory".to_string())?;
    let entered = root.join("gpui-public-callback-entered.txt");
    let panic_marker = root.join("gpui-panic-hook-observed.txt");
    let returned = root.join("gpui-callback-returned.txt");
    let terminal = root.join("gpui-window-terminal.txt");
    let fatal = root.join("gpui-typed-fatal.json");
    for path in [&entered, &panic_marker, &returned, &terminal, &fatal] {
        let _ = fs::remove_file(path);
    }
    let executable = std::env::current_exe().map_err(|_| "current-exe".to_string())?;
    let status = Command::new(executable)
        .arg("--panic-child")
        .arg("--callback-entered")
        .arg(&entered)
        .arg("--panic-observed")
        .arg(&panic_marker)
        .arg("--callback-returned")
        .arg(&returned)
        .arg("--terminal")
        .arg(&terminal)
        .arg("--typed-fatal")
        .arg(&fatal)
        .status()
        .map_err(|_| "panic-child-launch".to_string())?;
    let callback_entered = entered.is_file();
    let panic_observed = panic_marker.is_file();
    let callback_returned = returned.is_file();
    let window_terminal = terminal.is_file();
    let fatal_event = fatal.is_file();
    let backend_terminal = fatal_event
        && fs::read_to_string(&fatal)
            .is_ok_and(|value| value.contains("\"wm_ncdestroy_observed\":true"));
    let capability_passed =
        callback_entered && status.success() && fatal_event && backend_terminal && window_terminal;
    let trace = format!(
        "{{\"schema\":\"gpui-callback-panic/v2\",\"pinned_revision\":\"8945e2981b9fd00ca887e042d8adb9acc241b168\",\"backend_patch\":\"B-W2-3.5-001-no-unwind-terminal\",\"public_callback\":\"Context::observe_window_bounds\",\"native_dispatch\":\"gpui_windows::window_procedure/WM_SIZE\",\"child_exit_code\":{},\"child_success\":{},\"callback_entered\":{},\"panic_observed\":{},\"callback_returned\":{},\"typed_fatal_event\":{},\"backend_hwnd_terminal\":{},\"gpui_window_closed_terminal\":{},\"capability_passed\":{},\"disposition\":\"{}\",\"explorer_mutations\":false,\"shell_takeover\":false}}",
        status
            .code()
            .map_or_else(|| "null".into(), |code| code.to_string()),
        status.success(),
        callback_entered,
        panic_observed,
        callback_returned,
        fatal_event,
        backend_terminal,
        window_terminal,
        capability_passed,
        if capability_passed { "go" } else { "stop" }
    );
    fs::write(&output, &trace).map_err(|_| "trace-write".to_string())?;
    println!("{trace}");
    if !capability_passed || !panic_observed {
        return Err("pinned-backend-no-unwind-terminal-contract-failed".into());
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    let result = if std::env::args().any(|value| value == "--panic-child") {
        panic_child()
    } else if std::env::args().any(|value| value == "--controller") {
        controller()
    } else {
        Err("mode-required".into())
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(3)
        }
    }
}
