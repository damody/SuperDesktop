use std::{cell::RefCell, rc::Rc, time::Duration};

use desktop_ui::{AccessibleNode, DesktopView};
use gpui::{
    App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    point, px, size,
};
use platform_win::common::{
    appbar_shell_hook::{ControlledShellCapability, ScreenRect},
    desktop::configure_and_show_desktop_window,
    monitor_dpi_start::{MonitorRecord, enable_per_monitor_v2, snapshot_real_monitors},
    taskbar::{configure_and_show_taskbar_window, snapshot_task_windows},
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use taskbar_ui::{
    AccessibleTask, ClockLocale, CoreStatus, ProviderState, StatusRegion, TaskAction,
    TaskbarCallbacks, TaskbarLayout, TaskbarView, TestClock,
};

fn trace_action(action: &str) {
    let Some(path) = std::env::var_os("SUPERDESKTOP_ACTION_TRACE") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{action}");
    }
}

fn launch_superexplorer() {
    let developer_release =
        std::path::PathBuf::from(r"D:\SuperExplorer\target\release\SuperExplorer.exe");
    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("SuperExplorer.exe")))
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\missing\SuperExplorer.exe"));
    let resolver = explorer_bridge::ExecutableResolver {
        setting: std::env::var_os("SUPEREXPLORER_PATH").map(std::path::PathBuf::from),
        developer_release,
        adjacent,
    };
    match resolver.resolve() {
        Ok((resolved, _)) => {
            let spec = explorer_bridge::build_default_launch(&resolved);
            match explorer_bridge::ProcessLauncher.launch(&spec) {
                explorer_bridge::LaunchOutcome::Launched { .. } => {
                    trace_action("superexplorer:launched")
                }
                explorer_bridge::LaunchOutcome::ValidationFailed(_) => {
                    trace_action("superexplorer:validation-failed")
                }
                explorer_bridge::LaunchOutcome::SpawnFailed(_) => {
                    trace_action("superexplorer:spawn-failed")
                }
            }
        }
        Err(_) => trace_action("superexplorer:resolver-unavailable"),
    }
}

fn activate_task(stable_id: &str) {
    trace_action("task");
    let Some(hex) = stable_id.rsplit(':').next() else {
        return;
    };
    let Ok(hwnd) = usize::from_str_radix(hex, 16) else {
        return;
    };
    let Ok(windows) = snapshot_task_windows() else {
        return;
    };
    let Some(window) = windows
        .into_iter()
        .find(|window| window.hwnd_identity == hwnd as isize)
    else {
        return;
    };
    let action = if window.foreground {
        platform_win::common::taskbar::WindowAction::Minimize
    } else if window.minimized {
        platform_win::common::taskbar::WindowAction::RestoreAndActivate
    } else {
        platform_win::common::taskbar::WindowAction::Activate
    };
    let _ = platform_win::common::taskbar::apply_window_action(window.hwnd_identity, action);
}

fn hwnd(window: &gpui::Window) -> Result<isize, &'static str> {
    let handle = HasWindowHandle::window_handle(window).map_err(|_| "gpui-window-handle")?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("gpui-non-win32-hwnd");
    };
    Ok(handle.hwnd.get())
}

fn options(monitor: &MonitorRecord, taskbar: bool) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = (monitor.bounds.right - monitor.bounds.left) as f32 / scale;
    let height = if taskbar {
        80.0
    } else {
        (monitor.bounds.bottom - monitor.bounds.top) as f32 / scale
    };
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px(monitor.bounds.left as f32 / scale),
                px(if taskbar {
                    monitor.bounds.bottom as f32 / scale - height
                } else {
                    monitor.bounds.top as f32 / scale
                }),
            ),
            size: size(px(width), px(height)),
        })),
        titlebar: None,
        focus: false,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
}

fn status() -> StatusRegion {
    StatusRegion::new(
        TestClock {
            year: 2026,
            month: 8,
            day: 14,
            hour: 11,
            minute: 22,
        },
        ClockLocale::ZhTw,
        CoreStatus {
            network: ProviderState::Available("online".into()),
            volume: ProviderState::Available(40),
            muted: ProviderState::Available(false),
            input_language: ProviderState::Available("zh-TW".into()),
            battery: ProviderState::Unavailable("desktop"),
            notifications: ProviderState::Available(0),
        },
    )
}

pub fn run(shell: bool, duration: Option<Duration>) -> Result<(), &'static str> {
    enable_per_monitor_v2()?;
    let snapshot = snapshot_real_monitors()?;
    let visible_tasks = snapshot_task_windows()
        .map_err(|_| "task-window-snapshot")?
        .into_iter()
        .filter(|window| {
            window.visible && !window.tool_window && !window.cloaked && !window.owned_transient
        })
        .take(16)
        .map(|window| AccessibleTask {
            stable_id: window.window_identity,
            name: if window.title.is_empty() {
                window.application_identity
            } else {
                window.title
            },
            role: "button",
            active: window.foreground,
            minimized: window.minimized,
            actions: vec![
                TaskAction::Focus,
                TaskAction::Select,
                TaskAction::Invoke,
                TaskAction::Minimize,
                TaskAction::Restore,
            ],
        })
        .collect::<Vec<_>>();
    let terminal = Rc::new(RefCell::new(None::<Result<(), &'static str>>));
    let terminal_for_app = Rc::clone(&terminal);
    let platform = gpui_windows::WindowsPlatform::new(false).map_err(|_| "gpui-platform")?;
    gpui::Application::with_platform(Rc::new(platform))
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(move |cx: &mut App| {
            let mut desktop_handles = Vec::new();
            let mut taskbar_handles = Vec::new();
            let leases = Rc::new(RefCell::new(Vec::<ControlledShellCapability>::new()));
            let init_error = Rc::new(RefCell::new(None::<&'static str>));
            for monitor in snapshot.monitors.clone() {
                let desktop_monitor = monitor.clone();
                let desktop_error = Rc::clone(&init_error);
                let desktop = cx.open_window(options(&monitor, false), move |window, cx| {
                    let width = desktop_monitor.bounds.right - desktop_monitor.bounds.left;
                    let height = desktop_monitor.bounds.bottom - desktop_monitor.bounds.top;
                    if let Err(error) = hwnd(window).and_then(|value| {
                        configure_and_show_desktop_window(
                            value,
                            desktop_monitor.bounds.left,
                            desktop_monitor.bounds.top,
                            width,
                            height,
                        )
                        .map_err(|_| "desktop-window-configure")
                    }) {
                        *desktop_error.borrow_mut() = Some(error);
                    }
                    cx.new(|_| {
                        DesktopView::new(
                            vec![AccessibleNode::fixed_superexplorer(
                                &desktop_monitor.device_name,
                                false,
                                false,
                            )],
                            false,
                        )
                        .with_fixed_action(Rc::new(launch_superexplorer))
                    })
                });
                let Ok(desktop) = desktop else {
                    *terminal_for_app.borrow_mut() = Some(Err("desktop-window-open"));
                    cx.quit();
                    return;
                };
                desktop_handles.push(desktop);

                let taskbar_monitor = monitor.clone();
                let taskbar_tasks = visible_tasks.clone();
                let taskbar_error = Rc::clone(&init_error);
                let taskbar_leases = Rc::clone(&leases);
                let taskbar = cx.open_window(options(&monitor, true), move |window, cx| {
                    let scale = taskbar_monitor.dpi_x as f32 / 96.0;
                    let width = taskbar_monitor.bounds.right - taskbar_monitor.bounds.left;
                    let height = (80.0 * scale).round() as i32;
                    let bottom = if shell {
                        taskbar_monitor.bounds.bottom
                    } else {
                        taskbar_monitor.work_area.bottom
                    };
                    let configured = hwnd(window).and_then(|value| {
                        configure_and_show_taskbar_window(
                            value,
                            taskbar_monitor.bounds.left,
                            bottom - height,
                            width,
                            height,
                        )
                        .map_err(|_| "taskbar-window-configure")?;
                        if shell {
                            let mut lease =
                                ControlledShellCapability::attach_controlled_window(value)
                                    .map_err(|_| "taskbar-capability-attach")?;
                            lease
                                .register_appbar()
                                .map_err(|_| "taskbar-appbar-register")?;
                            lease
                                .register_shell_hook()
                                .map_err(|_| "taskbar-hook-register")?;
                            lease
                                .reserve_bottom(
                                    ScreenRect {
                                        left: taskbar_monitor.bounds.left,
                                        top: taskbar_monitor.bounds.top,
                                        right: taskbar_monitor.bounds.right,
                                        bottom: taskbar_monitor.bounds.bottom,
                                    },
                                    height,
                                )
                                .map_err(|_| "taskbar-work-area-reserve")?;
                            taskbar_leases.borrow_mut().push(lease);
                        }
                        Ok(())
                    });
                    if let Err(error) = configured {
                        *taskbar_error.borrow_mut() = Some(error);
                    }
                    cx.new(|_| TaskbarView {
                        accessible_root_name: "SuperTaskbar".into(),
                        layout: TaskbarLayout::calculate(
                            2,
                            taskbar_monitor.dpi_x,
                            width as f32,
                            &[],
                            &["superexplorer".into()],
                        ),
                        tasks: taskbar_tasks,
                        fixed_name: "SuperExplorer".into(),
                        status: status(),
                        callbacks: Some(TaskbarCallbacks {
                            start: Rc::new(|| {
                                trace_action("start");
                                let _ = platform_win::common::monitor_dpi_start::invoke_start_host_controlled();
                            }),
                            fixed: Rc::new(launch_superexplorer),
                            task: Rc::new(activate_task),
                        }),
                    })
                });
                let Ok(taskbar) = taskbar else {
                    *terminal_for_app.borrow_mut() = Some(Err("taskbar-window-open"));
                    cx.quit();
                    return;
                };
                taskbar_handles.push(taskbar);
            }
            if let Some(error) = init_error.borrow_mut().take() {
                *terminal_for_app.borrow_mut() = Some(Err(error));
                cx.quit();
                return;
            }

            if let Some(duration) = duration {
                let background = cx.background_executor().clone();
                let foreground = cx.foreground_executor().clone();
                let async_app = cx.to_async();
                let terminal_for_timer = Rc::clone(&terminal_for_app);
                let leases_for_timer = Rc::clone(&leases);
                foreground
                    .spawn(async move {
                        background.timer(duration).await;
                        async_app.update(|app| {
                            for lease in leases_for_timer.borrow_mut().iter_mut() {
                                lease.teardown();
                            }
                            for handle in desktop_handles {
                                let _ = handle.update(app, |_, window, _| window.remove_window());
                            }
                            for handle in taskbar_handles {
                                let _ = handle.update(app, |_, window, _| window.remove_window());
                            }
                            *terminal_for_timer.borrow_mut() = Some(Ok(()));
                            app.quit();
                        });
                    })
                    .detach();
            }
        });
    terminal.borrow_mut().take().unwrap_or(Ok(()))
}
