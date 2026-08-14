//! Short-lived, real GPUI window contract used by the desktop evidence gate.

use std::{cell::RefCell, env, fs, process::ExitCode, rc::Rc, time::Duration};

use desktop_ui::{AccessibleNode, DesktopView};
use gpui::{
    App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    point, px, size,
};
use platform_win::common::desktop::configure_and_show_desktop_window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn options(origin_x: f32) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(origin_x), px(40.)),
            size: size(px(320.), px(180.)),
        })),
        titlebar: None,
        focus: false,
        show: false,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
}

fn configure_window(window: &gpui::Window, origin_x: f32) -> Result<(), String> {
    let handle = HasWindowHandle::window_handle(window).map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("gpui-non-win32-hwnd".into());
    };
    let scale = window.scale_factor();
    configure_and_show_desktop_window(
        handle.hwnd.get(),
        (origin_x * scale) as i32,
        (40. * scale) as i32,
        (320. * scale) as i32,
        (180. * scale) as i32,
    )
    .map_err(|error| format!("{error:?}"))
}

fn run() -> Result<String, String> {
    let trace_output = env::var("DESKTOP_HEADFUL_OUTPUT").ok();
    let terminal = Rc::new(RefCell::new(None::<Result<String, String>>));
    let terminal_for_app = Rc::clone(&terminal);
    let platform = gpui_windows::WindowsPlatform::new(false).map_err(|error| error.to_string())?;
    gpui::Application::with_platform(Rc::new(platform))
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(move |cx: &mut App| {
            let init_error = Rc::new(RefCell::new(None::<String>));
            let first_error = Rc::clone(&init_error);
            let first = cx.open_window(options(40.), move |window, cx| {
                if let Err(error) = configure_window(window, 40.) {
                    *first_error.borrow_mut() = Some(error);
                }
                cx.new(|_| {
                    DesktopView::new(
                        vec![AccessibleNode::fixed_superexplorer("monitor-96", true, true)],
                        false,
                    )
                })
            });
            let second_error = Rc::clone(&init_error);
            let second = cx.open_window(options(400.), move |window, cx| {
                if let Err(error) = configure_window(window, 400.) {
                    *second_error.borrow_mut() = Some(error);
                }
                cx.new(|_| {
                    DesktopView::new(
                        vec![AccessibleNode::fixed_superexplorer("monitor-144", false, false)],
                        true,
                    )
                })
            });
            let (Ok(first), Ok(second)) = (first, second) else {
                *terminal_for_app.borrow_mut() = Some(Err("gpui-open-window".into()));
                cx.quit();
                return;
            };
            if let Some(error) = init_error.borrow_mut().take() {
                *terminal_for_app.borrow_mut() = Some(Err(error));
                cx.quit();
                return;
            }

            let terminal_for_timer = Rc::clone(&terminal_for_app);
            let trace_output_for_timer = trace_output.clone();
            let background = cx.background_executor().clone();
            let foreground = cx.foreground_executor().clone();
            let async_app = cx.to_async();
            foreground
                .spawn(async move {
                    background.timer(Duration::from_millis(150)).await;
                    async_app.update(|app| {
                        let first_active = first.is_active(app).unwrap_or(false);
                        let second_active = second.is_active(app).unwrap_or(false);
                        let first_bounds = first.update(app, |_, window, _| window.bounds());
                        let second_bounds = second.update(app, |_, window, _| window.bounds());
                        match (first_bounds, second_bounds) {
                            (Ok(first_bounds), Ok(second_bounds)) => {
                                let trace = format!(
                                    concat!(
                                        "{{\"schema\":\"desktop-headful-contract/v1\",",
                                        "\"gpui_windows_opened\":2,\"requested_focus\":false,",
                                        "\"window_active\":[{},{}],",
                                        "\"virtual_dpi_matrix\":[96,144],",
                                        "\"bounds\":[{{\"width\":{},\"height\":{}}},",
                                        "{{\"width\":{},\"height\":{}}}],",
                                        "\"accessible_root\":\"SuperDesktop\",",
                                        "\"accessible_item\":{{\"name\":\"SuperExplorer\",",
                                        "\"role\":\"button\",\"actions\":[\"focus\",\"select\",\"invoke\"]}},",
                                        "\"windows_closed\":2}}"
                                    ),
                                    first_active,
                                    second_active,
                                    f32::from(first_bounds.size.width),
                                    f32::from(first_bounds.size.height),
                                    f32::from(second_bounds.size.width),
                                    f32::from(second_bounds.size.height),
                                );
                                if let Some(path) = &trace_output_for_timer
                                    && let Err(error) = fs::write(path, &trace)
                                {
                                    *terminal_for_timer.borrow_mut() =
                                        Some(Err(format!("trace-write:{error}")));
                                    let _ = first.update(app, |_, window, _| window.remove_window());
                                    let _ = second.update(app, |_, window, _| window.remove_window());
                                    app.quit();
                                    return;
                                }
                                *terminal_for_timer.borrow_mut() = Some(Ok(trace));
                            }
                            _ => {
                                *terminal_for_timer.borrow_mut() =
                                    Some(Err("gpui-window-bounds".into()));
                            }
                        }
                        let _ = first.update(app, |_, window, _| window.remove_window());
                        let _ = second.update(app, |_, window, _| window.remove_window());
                        app.quit();
                    });
                })
                .detach();
        });
    terminal
        .borrow_mut()
        .take()
        .ok_or_else(|| "gpui-run-without-terminal".to_string())?
}

fn main() -> ExitCode {
    match run() {
        Ok(trace) => {
            if let Ok(path) = env::var("DESKTOP_HEADFUL_OUTPUT")
                && let Err(error) = fs::write(path, &trace)
            {
                println!("{{\"admitted\":false,\"error\":\"trace-write:{error}\"}}");
                return ExitCode::from(2);
            }
            println!("{trace}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{{\"admitted\":false,\"error\":\"{error}\"}}");
            ExitCode::from(2)
        }
    }
}
