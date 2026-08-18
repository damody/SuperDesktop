#![windows_subsystem = "windows"]

use std::{cell::RefCell, process::ExitCode, rc::Rc, time::Duration};

use gpui::{
    App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    point, px, size,
};
use settings_store::TaskbarSettings;
use taskbar_ui::{TaskbarContextView, TaskbarSettingsEffect, TaskbarSettingsView};

fn options(surface: &str, _cx: &App) -> WindowOptions {
    let scale = platform_win::common::monitor_dpi_start::snapshot_real_monitors()
        .ok()
        .and_then(|snapshot| {
            snapshot
                .monitors
                .into_iter()
                .find(|monitor| monitor.primary)
        })
        .map_or(1.0, |monitor| monitor.dpi_x as f32 / 96.0);
    let width = if surface == "context" { 220.0 } else { 900.0 } * scale;
    let height = (if surface == "context" { 80.0 } else { 760.0 } * scale).min(1080.0);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(width), px(height)),
        })),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
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
            if surface == "context" {
                let Ok(handle) = cx.open_window(options(&surface, cx), move |window, cx| {
                    window.activate_window();
                    cx.new(move |cx| {
                        TaskbarContextView::new(
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
