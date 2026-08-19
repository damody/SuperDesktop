#![windows_subsystem = "windows"]

use std::{cell::RefCell, process::ExitCode, rc::Rc, time::Duration};

use gpui::{
    App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    point, px, size,
};
use platform_win::common::taskbar::promote_owned_popup_topmost;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use settings_store::TaskbarSettings;
use shell_core::WindowId;
use taskbar_ui::{
    PreviewCard, TaskFlyoutView, TaskbarContextView, TaskbarSettingsEffect, TaskbarSettingsView,
};

fn options(surface: &str, _cx: &App) -> WindowOptions {
    let width = match surface {
        "context" => 220.0,
        "preview" => 380.0,
        _ => 1100.0,
    };
    let height = match surface {
        "context" => 210.0,
        "preview" => 260.0,
        _ => 860.0,
    };
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(width), px(height)),
        })),
        titlebar: None,
        focus: surface != "preview",
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
}

fn hwnd(window: &gpui::Window) -> Result<isize, &'static str> {
    let handle = HasWindowHandle::window_handle(window).map_err(|_| "gpui-window-handle")?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("gpui-non-win32-hwnd");
    };
    Ok(handle.hwnd.get())
}

fn main() -> ExitCode {
    let surface = std::env::args()
        .skip_while(|arg| arg != "--surface")
        .nth(1)
        .unwrap_or_else(|| "settings".into());
    let hold_ms = std::env::args()
        .skip_while(|arg| arg != "--hold-ms")
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(4_000);
    let terminal = Rc::new(RefCell::new(false));
    let terminal_for_app = Rc::clone(&terminal);
    let platform = match gpui_windows::WindowsPlatform::new(false) {
        Ok(platform) => platform,
        Err(_) => return ExitCode::FAILURE,
    };
    gpui::Application::with_platform(Rc::new(platform))
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(move |cx: &mut App| {
            let background = cx.background_executor().clone();
            let foreground = cx.foreground_executor().clone();
            let app = cx.to_async();
            if surface == "preview" {
                let Ok(handle) = cx.open_window(options(&surface, cx), move |window, cx| {
                    let destination_hwnd = hwnd(window).expect("preview hwnd");
                    promote_owned_popup_topmost(destination_hwnd).expect("topmost preview");
                    cx.new(move |cx| {
                        TaskFlyoutView::new(
                            vec![PreviewCard {
                                window_id: WindowId::new("headful-preview").unwrap(),
                                title: "SuperDesktop hover preview".into(),
                                minimized: false,
                                preview_available: false,
                                preview_source: None,
                            }],
                            Rc::new(|_| {}),
                            Rc::new(|window, _| window.remove_window()),
                            Rc::new(|_, _| {}),
                            destination_hwnd,
                            false,
                            cx,
                        )
                    })
                }) else {
                    cx.quit();
                    return;
                };
                foreground
                    .spawn(async move {
                        background.timer(Duration::from_millis(hold_ms)).await;
                        app.update(|app| {
                            let _ = handle.update(app, |_, window, _| window.remove_window());
                            *terminal_for_app.borrow_mut() = true;
                            app.quit();
                        });
                    })
                    .detach();
            } else if surface == "context" {
                let Ok(handle) = cx.open_window(options(&surface, cx), move |window, cx| {
                    window.activate_window();
                    cx.new(move |cx| {
                        TaskbarContextView::new(
                            true,
                            settings_store::TaskbarSearchMode::Hidden,
                            true,
                            Rc::new(|_, _| {}),
                            Rc::new(|window, _| window.remove_window()),
                            window,
                            cx,
                        )
                    })
                }) else {
                    cx.quit();
                    return;
                };
                foreground
                    .spawn(async move {
                        background.timer(Duration::from_millis(hold_ms)).await;
                        app.update(|app| {
                            let _ = handle.update(app, |_, window, _| window.remove_window());
                            *terminal_for_app.borrow_mut() = true;
                            app.quit();
                        });
                    })
                    .detach();
            } else {
                let Ok(handle) = cx.open_window(options(&surface, cx), move |window, cx| {
                    window.activate_window();
                    cx.new(move |cx| {
                        TaskbarSettingsView::new(
                            TaskbarSettings::default(),
                            1,
                            Rc::new(|effect| match effect {
                                TaskbarSettingsEffect::Save {
                                    candidate,
                                    base_revision,
                                } => Ok((candidate, base_revision + 1)),
                                _ => Ok((TaskbarSettings::default(), 1)),
                            }),
                            Rc::new(|window, _| window.remove_window()),
                            cx,
                        )
                    })
                }) else {
                    cx.quit();
                    return;
                };
                foreground
                    .spawn(async move {
                        background.timer(Duration::from_millis(hold_ms)).await;
                        app.update(|app| {
                            let _ = handle.update(app, |_, window, _| window.remove_window());
                            *terminal_for_app.borrow_mut() = true;
                            app.quit();
                        });
                    })
                    .detach();
            }
        });
    if *terminal.borrow() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
