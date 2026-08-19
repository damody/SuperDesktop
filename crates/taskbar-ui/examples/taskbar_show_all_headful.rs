#![windows_subsystem = "windows"]

use std::{cell::RefCell, process::ExitCode, rc::Rc, time::Duration};

use gpui::{
    App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    point, px, size,
};
use settings_store::{TaskbarAlignment, TaskbarSearchMode};
use shell_provider_protocol::{
    IconData, IconKey, NotificationIcon, NotificationSnapshot, RegisteredIcon,
};
use taskbar_ui::{
    ClockLocale, CoreStatus, NotificationAreaModel, NotificationOverflowView, ProviderState,
    StatusRegion, TaskbarCallbacks, TaskbarLayout, TaskbarView, TestClock,
};

fn options(width: f32, height: f32, left: f32, top: f32, focus: bool) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(left), px(top)),
            size: size(px(width), px(height)),
        })),
        titlebar: None,
        focus,
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
            day: 19,
            hour: 16,
            minute: 20,
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

fn registered_icon(id: u32) -> RegisteredIcon {
    RegisteredIcon {
        key: IconKey {
            client_id: "show-all-fixture".into(),
            icon_id: id,
        },
        generation: 1,
        icon: NotificationIcon {
            owner_id: "show-all-fixture".into(),
            icon_id: id,
            tooltip: format!("Fixture tray icon {id}"),
            visible: true,
            icon: Some(IconData {
                width: 1,
                height: 1,
                rgba: vec![40 * id as u8, 80, 220, 255],
            }),
        },
        always_visible: id == 1,
    }
}

fn main() -> ExitCode {
    let mode = std::env::args()
        .skip_while(|argument| argument != "--mode")
        .nth(1)
        .unwrap_or_else(|| "empty".into());
    let hold_ms = std::env::args()
        .skip_while(|argument| argument != "--hold-ms")
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30_000);
    let terminal = Rc::new(RefCell::new(false));
    let terminal_for_app = Rc::clone(&terminal);
    let platform = match gpui_windows::WindowsPlatform::new(false) {
        Ok(platform) => platform,
        Err(_) => return ExitCode::FAILURE,
    };
    gpui::Application::with_platform(Rc::new(platform))
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(move |cx: &mut App| {
            let popup_slot = Rc::new(RefCell::new(
                None::<gpui::WindowHandle<NotificationOverflowView>>,
            ));
            let popup_for_callback = Rc::clone(&popup_slot);
            let callbacks = TaskbarCallbacks {
                start: Rc::new(|_| {}),
                show_desktop: Rc::new(|_| {}),
                task_view: Rc::new(|_| {}),
                fixed: Rc::new(|| {}),
                task: Rc::new(|_, _| {}),
                task_hover: Rc::new(|_, _, _| {}),
                task_context: Rc::new(|_, _| {}),
                taskbar_context: Rc::new(|_, _| {}),
                resize_rows: Rc::new(|_, _, _| false),
                notification: Rc::new(|_, _| {}),
                notification_overflow: Rc::new(move |nodes, app| {
                    if let Some(open) = popup_for_callback.borrow_mut().take() {
                        let _ = open.update(app, |_, window, _| window.remove_window());
                        return;
                    }
                    let dismiss_slot = Rc::clone(&popup_for_callback);
                    let opened = app.open_window(
                        options(344.0, 72.0, 520.0, 380.0, true),
                        move |window, cx| {
                            window.activate_window();
                            let dismiss_slot = Rc::clone(&dismiss_slot);
                            cx.new(move |cx| {
                                NotificationOverflowView::new(
                                    nodes,
                                    Rc::new(|_, _| {}),
                                    Rc::new(move |window, _| {
                                        window.remove_window();
                                        *dismiss_slot.borrow_mut() = None;
                                    }),
                                    window,
                                    cx,
                                )
                            })
                        },
                    );
                    if let Ok(handle) = opened {
                        *popup_for_callback.borrow_mut() = Some(handle);
                    }
                }),
                system_status: Rc::new(|_, _| {}),
                system_flyout: Rc::new(|_, _| {}),
                rendered: Rc::new(|| {}),
            };
            let mut notification_area = NotificationAreaModel::default();
            let icons = if mode == "populated" {
                vec![registered_icon(1), registered_icon(2), registered_icon(3)]
            } else {
                Vec::new()
            };
            let _ = notification_area.apply_snapshot(
                NotificationSnapshot {
                    generation: 1,
                    icons,
                    notifications: Vec::new(),
                },
                1,
            );
            let opened = cx.open_window(options(900.0, 80.0, 200.0, 700.0, true), move |_, cx| {
                cx.new(move |_| TaskbarView {
                    accessible_root_name: "Show all tray icons fixture".into(),
                    layout: TaskbarLayout::calculate(1, 96, 900.0, &[], &["superexplorer".into()]),
                    tasks: Vec::new(),
                    fixed_name: "SuperExplorer".into(),
                    fixed_icon: None,
                    status: status(),
                    system_snapshot: None,
                    system_flyout: None,
                    notification_area,
                    overlays: Default::default(),
                    show_labels: false,
                    search_mode: TaskbarSearchMode::Hidden,
                    show_task_view: false,
                    alignment: TaskbarAlignment::Left,
                    locked: true,
                    callbacks: Some(callbacks),
                    keyboard_focus: None,
                    resize_subscription: None,
                })
            });
            let Ok(handle) = opened else {
                cx.quit();
                return;
            };
            let background = cx.background_executor().clone();
            let foreground = cx.foreground_executor().clone();
            let app = cx.to_async();
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
        });
    if *terminal.borrow() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
