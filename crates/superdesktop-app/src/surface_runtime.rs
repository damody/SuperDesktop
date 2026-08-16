use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use desktop_ui::{
    AccessibleAction, AccessibleNode, DeletePolicy, DesktopItem, DesktopOperation,
    DesktopOperationController, DesktopOperationTerminal, DesktopView, TransferIntent,
    execute_desktop_operation,
};
use gpui::{
    App, AppContext, Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    point, px, size,
};
use platform_win::common::{
    appbar_shell_hook::{ControlledShellCapability, ScreenRect},
    desktop::{configure_and_show_desktop_window, current_wallpaper_path},
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
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let _ = writeln!(file, "{millis} {action}");
    }
}

fn launch_superexplorer() {
    launch_superexplorer_at(None);
}

fn launch_superexplorer_at(initial_path: Option<&Path>) {
    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("SuperExplorer.exe")))
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\missing\SuperExplorer.exe"));
    let resolver = explorer_bridge::ExecutableResolver {
        setting: std::env::var_os("SUPEREXPLORER_PATH").map(std::path::PathBuf::from),
        developer_release: adjacent.clone(),
        adjacent,
    };
    match resolver.resolve() {
        Ok((resolved, _)) => {
            let spec = match initial_path {
                Some(path) => match explorer_bridge::build_folder_launch(&resolved, path) {
                    Ok(spec) => spec,
                    Err(_) => {
                        trace_action("superexplorer:folder-validation-failed");
                        return;
                    }
                },
                None => explorer_bridge::build_default_launch(&resolved),
            };
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

#[derive(Default)]
struct DesktopNamespaceRuntime {
    items: BTreeMap<String, DesktopItem>,
    allowed_roots: Vec<PathBuf>,
    user_root: Option<PathBuf>,
}

fn desktop_item_key(item: &DesktopItem) -> String {
    format!("desktop-item:{}", item.identity.as_str())
}

fn refresh_desktop_namespace(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    monitor: &str,
) -> Vec<AccessibleNode> {
    let mut nodes = vec![fixed_node(monitor)];
    let Ok((user_root, public_root)) = platform_win::common::desktop::known_desktop_roots() else {
        trace_action("desktop:root-resolution-failed");
        return nodes;
    };
    let Ok(entries) = platform_win::common::desktop::enumerate_known_desktops() else {
        trace_action("desktop:enumeration-failed");
        return nodes;
    };
    let mut items = BTreeMap::new();
    let mut allowed_roots = vec![user_root.clone(), public_root];
    for entry in entries {
        let Some(parent) = entry.canonical_path.parent().map(Path::to_path_buf) else {
            continue;
        };
        if !allowed_roots.contains(&parent) {
            allowed_roots.push(parent);
        }
        let Ok(item) = DesktopItem::try_from(entry) else {
            continue;
        };
        if item.capabilities.hidden || item.capabilities.system {
            continue;
        }
        let key = desktop_item_key(&item);
        nodes.push(AccessibleNode {
            stable_id: key.clone(),
            name: item.display_name.clone(),
            role: "button",
            selected: false,
            focused: false,
            actions: vec![
                AccessibleAction::Focus,
                AccessibleAction::Select,
                AccessibleAction::Invoke,
            ],
            message_key: None,
        });
        items.insert(key, item);
    }
    nodes[1..].sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.stable_id.cmp(&right.stable_id))
    });
    *runtime.borrow_mut() = DesktopNamespaceRuntime {
        items,
        allowed_roots,
        user_root: Some(user_root),
    };
    trace_action("desktop:refreshed");
    nodes
}

fn activate_desktop_item(runtime: &Rc<RefCell<DesktopNamespaceRuntime>>, stable_id: &str) {
    let item = runtime.borrow().items.get(stable_id).cloned();
    let Some(item) = item else {
        trace_action("desktop:activation-stale");
        return;
    };
    let path = PathBuf::from(&item.activation_token);
    if item.capabilities.folder {
        launch_superexplorer_at(Some(&path));
    } else {
        match platform_win::common::desktop::launch_association(&path) {
            Ok(platform_win::common::desktop::AssociationAdmission::Launched) => {
                trace_action("desktop:association-launched")
            }
            Ok(platform_win::common::desktop::AssociationAdmission::ValidationFailed) => {
                trace_action("desktop:association-validation-failed")
            }
            Ok(platform_win::common::desktop::AssociationAdmission::LaunchFailed) | Err(_) => {
                trace_action("desktop:association-launch-failed")
            }
        }
    }
}

fn recycle_desktop_item(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    operations: &Rc<RefCell<DesktopOperationController>>,
    stable_id: &str,
) -> bool {
    delete_desktop_item(runtime, operations, stable_id, false)
}

fn permanently_delete_desktop_item(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    operations: &Rc<RefCell<DesktopOperationController>>,
    stable_id: &str,
) -> bool {
    delete_desktop_item(runtime, operations, stable_id, true)
}

fn delete_desktop_item(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    operations: &Rc<RefCell<DesktopOperationController>>,
    stable_id: &str,
    permanent: bool,
) -> bool {
    let (source, display_name, roots) = {
        let runtime = runtime.borrow();
        let Some(item) = runtime.items.get(stable_id) else {
            trace_action("desktop:delete-stale");
            return false;
        };
        (
            PathBuf::from(&item.activation_token),
            item.display_name.clone(),
            runtime.allowed_roots.clone(),
        )
    };
    if permanent
        && !platform_win::common::desktop_operations::confirm_permanent_delete(&display_name)
    {
        trace_action("desktop:permanent-delete-cancelled");
        return false;
    }
    let request = match operations.borrow_mut().plan(DesktopOperation::Delete {
        source,
        policy: if permanent {
            DeletePolicy::PermanentExplicit
        } else {
            DeletePolicy::Recycle
        },
    }) {
        Ok(request) => request,
        Err(_) => {
            trace_action("desktop:delete-plan-rejected");
            return false;
        }
    };
    let terminal = execute_desktop_operation(&request, &roots, |_, _| true);
    let accepted = operations
        .borrow_mut()
        .terminal(request.correlation_id, terminal)
        .is_ok();
    let succeeded = accepted && terminal == DesktopOperationTerminal::Succeeded;
    trace_action(if succeeded && permanent {
        "desktop:permanent-delete-succeeded"
    } else if succeeded {
        "desktop:recycle-succeeded"
    } else {
        "desktop:delete-failed"
    });
    succeeded
}

fn rename_desktop_item(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    operations: &Rc<RefCell<DesktopOperationController>>,
    stable_id: &str,
    new_name: &str,
) -> bool {
    let (source, roots) = {
        let runtime = runtime.borrow();
        let Some(item) = runtime.items.get(stable_id) else {
            trace_action("desktop:rename-stale");
            return false;
        };
        (
            PathBuf::from(&item.activation_token),
            runtime.allowed_roots.clone(),
        )
    };
    let request = match operations.borrow_mut().plan(DesktopOperation::Rename {
        source,
        new_name: new_name.to_owned(),
    }) {
        Ok(request) => request,
        Err(_) => {
            trace_action("desktop:rename-plan-rejected");
            return false;
        }
    };
    let terminal = execute_desktop_operation(&request, &roots, |_, _| true);
    let accepted = operations
        .borrow_mut()
        .terminal(request.correlation_id, terminal)
        .is_ok();
    let succeeded = accepted && terminal == DesktopOperationTerminal::Succeeded;
    trace_action(if succeeded {
        "desktop:rename-succeeded"
    } else {
        "desktop:rename-failed"
    });
    succeeded
}

fn transfer_desktop_item(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    operations: &Rc<RefCell<DesktopOperationController>>,
    source_id: &str,
    destination_id: &str,
) -> bool {
    let (source_item, destination_item, roots) = {
        let runtime = runtime.borrow();
        let Some(source) = runtime.items.get(source_id).cloned() else {
            trace_action("desktop:transfer-source-stale");
            return false;
        };
        let Some(destination) = runtime.items.get(destination_id).cloned() else {
            trace_action("desktop:transfer-destination-stale");
            return false;
        };
        (source, destination, runtime.allowed_roots.clone())
    };
    if !destination_item.capabilities.folder {
        trace_action("desktop:transfer-destination-not-folder");
        return false;
    }
    let source = PathBuf::from(&source_item.activation_token);
    let destination_directory = PathBuf::from(&destination_item.activation_token);
    let Some(file_name) = source.file_name().map(std::ffi::OsStr::to_os_string) else {
        return false;
    };
    let intent = if source_item.origin == destination_item.origin {
        TransferIntent::Move
    } else {
        TransferIntent::Copy
    };
    let request = match operations.borrow_mut().plan(DesktopOperation::Transfer {
        source,
        destination: destination_directory.join(file_name),
        intent,
        collision: platform_win::common::desktop_operations::CollisionPolicy::Rename,
    }) {
        Ok(request) => request,
        Err(_) => {
            trace_action("desktop:transfer-plan-rejected");
            return false;
        }
    };
    let terminal = execute_desktop_operation(&request, &roots, |_, _| true);
    let accepted = operations
        .borrow_mut()
        .terminal(request.correlation_id, terminal)
        .is_ok();
    let succeeded = accepted && terminal == DesktopOperationTerminal::Succeeded;
    trace_action(if succeeded {
        "desktop:transfer-succeeded"
    } else {
        "desktop:transfer-failed"
    });
    succeeded
}

fn import_external_desktop_items(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    operations: &Rc<RefCell<DesktopOperationController>>,
    paths: &[PathBuf],
) -> bool {
    if paths.is_empty() {
        return false;
    }
    let (user_root, desktop_roots) = {
        let runtime = runtime.borrow();
        let Some(user_root) = runtime.user_root.clone() else {
            return false;
        };
        (user_root, runtime.allowed_roots.clone())
    };
    let mut all_succeeded = true;
    for source in paths {
        let Some(name) = source.file_name() else {
            all_succeeded = false;
            continue;
        };
        let Some(parent) = source.parent() else {
            all_succeeded = false;
            continue;
        };
        let mut admitted_roots = desktop_roots.clone();
        admitted_roots.push(parent.to_path_buf());
        let request = match operations.borrow_mut().plan(DesktopOperation::Transfer {
            source: source.clone(),
            destination: user_root.join(name),
            intent: TransferIntent::Copy,
            collision: platform_win::common::desktop_operations::CollisionPolicy::Rename,
        }) {
            Ok(request) => request,
            Err(_) => {
                all_succeeded = false;
                continue;
            }
        };
        let terminal = execute_desktop_operation(&request, &admitted_roots, |_, _| true);
        let accepted = operations
            .borrow_mut()
            .terminal(request.correlation_id, terminal)
            .is_ok();
        all_succeeded &= accepted && terminal == DesktopOperationTerminal::Succeeded;
    }
    trace_action(if all_succeeded {
        "desktop:external-drop-succeeded"
    } else {
        "desktop:external-drop-partial-or-failed"
    });
    all_succeeded
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

fn visible_tasks() -> Result<Vec<AccessibleTask>, &'static str> {
    snapshot_task_windows()
        .map_err(|_| "task-window-snapshot")
        .map(|windows| {
            windows
                .into_iter()
                .filter(|window| {
                    window.visible
                        && !window.tool_window
                        && !window.cloaked
                        && !window.owned_transient
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
                    attention: false,
                    group_size: 1,
                    available: true,
                    actions: vec![
                        TaskAction::Focus,
                        TaskAction::Select,
                        TaskAction::Invoke,
                        TaskAction::Minimize,
                        TaskAction::Restore,
                    ],
                })
                .collect()
        })
}

fn verification_state_tasks() -> Vec<AccessibleTask> {
    [
        ("active", true, false, false, 1, true),
        ("minimized", false, true, false, 1, true),
        ("attention", false, false, true, 1, true),
        ("group", false, false, false, 3, true),
        ("unavailable", false, false, false, 1, false),
    ]
    .into_iter()
    .map(
        |(state, active, minimized, attention, group_size, available)| AccessibleTask {
            stable_id: format!("verification-state:{state}"),
            name: format!("State {state}"),
            role: "button",
            active,
            minimized,
            attention,
            group_size,
            available,
            actions: if available {
                vec![TaskAction::Focus, TaskAction::Select, TaskAction::Invoke]
            } else {
                vec![TaskAction::Focus]
            },
        },
    )
    .collect()
}

fn hwnd(window: &gpui::Window) -> Result<isize, &'static str> {
    let handle = HasWindowHandle::window_handle(window).map_err(|_| "gpui-window-handle")?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("gpui-non-win32-hwnd");
    };
    Ok(handle.hwnd.get())
}

fn options(monitor: &MonitorRecord, taskbar: bool, interactive: bool) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = (monitor.bounds.right - monitor.bounds.left) as f32 / scale;
    let height = if taskbar {
        80.0
    } else if interactive {
        (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale
    } else {
        (monitor.bounds.bottom - monitor.bounds.top) as f32 / scale
    };
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px(monitor.bounds.left as f32 / scale),
                px(if taskbar {
                    (if interactive {
                        monitor.work_area.bottom
                    } else {
                        monitor.bounds.bottom
                    }) as f32
                        / scale
                        - height
                } else {
                    (if interactive {
                        monitor.work_area.top
                    } else {
                        monitor.bounds.top
                    }) as f32
                        / scale
                }),
            ),
            size: size(px(width), px(height)),
        })),
        titlebar: None,
        focus: interactive,
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

fn fixed_label() -> &'static str {
    match std::env::var("SUPERDESKTOP_LOCALE").as_deref() {
        Ok("zh-CN") => "超级资源管理器",
        Ok("zh-TW") => "超級檔案總管",
        _ => "SuperExplorer",
    }
}

fn fixed_node(monitor: &str) -> AccessibleNode {
    let selected = std::env::var_os("SUPERDESKTOP_VERIFICATION_DESKTOP_SELECTED").is_some();
    let mut node = AccessibleNode::fixed_superexplorer(monitor, selected, selected);
    node.name = fixed_label().into();
    node
}

pub fn run(shell: bool, duration: Option<Duration>) -> Result<(), &'static str> {
    enable_per_monitor_v2()?;
    let snapshot = snapshot_real_monitors()?;
    let state_matrix = std::env::var_os("SUPERDESKTOP_VERIFICATION_STATE_MATRIX").is_some();
    let initial_tasks = if state_matrix {
        verification_state_tasks()
    } else {
        visible_tasks()?
    };
    let wallpaper = std::env::var_os("SUPERDESKTOP_WALLPAPER_PATH")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| current_wallpaper_path().ok());
    let verification_surface = std::env::var("SUPERDESKTOP_VERIFICATION_SURFACE").ok();
    let interactive = verification_surface.is_some();
    let desktop_namespace = Rc::new(RefCell::new(DesktopNamespaceRuntime::default()));
    let desktop_operations = Rc::new(RefCell::new(DesktopOperationController::default()));
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
                if verification_surface.as_deref() != Some("taskbar") {
                    let desktop_monitor = monitor.clone();
                    let desktop_wallpaper = wallpaper.clone();
                    let desktop_error = Rc::clone(&init_error);
                    let desktop_namespace_for_view = Rc::clone(&desktop_namespace);
                    let desktop_operations_for_view = Rc::clone(&desktop_operations);
                    let desktop = cx.open_window(options(&monitor, false, interactive), move |window, cx| {
                        if interactive {
                            window.activate_window();
                        }
                        let width = desktop_monitor.bounds.right - desktop_monitor.bounds.left;
                        let height = desktop_monitor.bounds.bottom - desktop_monitor.bounds.top;
                        if !interactive
                            && let Err(error) = hwnd(window).and_then(|value| {
                                configure_and_show_desktop_window(
                                    value,
                                    desktop_monitor.bounds.left,
                                    desktop_monitor.bounds.top,
                                    width,
                                    height,
                                )
                                .map_err(|_| "desktop-window-configure")
                            })
                        {
                            *desktop_error.borrow_mut() = Some(error);
                        }
                        cx.new(move |cx| {
                            let monitor_key = desktop_monitor.device_name.clone();
                            let nodes = refresh_desktop_namespace(
                                &desktop_namespace_for_view,
                                &monitor_key,
                            );
                            let activation_namespace = Rc::clone(&desktop_namespace_for_view);
                            let recycle_namespace = Rc::clone(&desktop_namespace_for_view);
                            let recycle_operations = Rc::clone(&desktop_operations_for_view);
                            let permanent_delete_namespace =
                                Rc::clone(&desktop_namespace_for_view);
                            let permanent_delete_operations =
                                Rc::clone(&desktop_operations_for_view);
                            let rename_namespace = Rc::clone(&desktop_namespace_for_view);
                            let rename_operations = Rc::clone(&desktop_operations_for_view);
                            let transfer_namespace = Rc::clone(&desktop_namespace_for_view);
                            let transfer_operations = Rc::clone(&desktop_operations_for_view);
                            let external_drop_namespace = Rc::clone(&desktop_namespace_for_view);
                            let external_drop_operations = Rc::clone(&desktop_operations_for_view);
                            let refresh_namespace = Rc::clone(&desktop_namespace_for_view);
                            let refresh_monitor = monitor_key.clone();
                            let mut view = DesktopView::new(
                                nodes,
                                false,
                            )
                            .with_fixed_action(Rc::new(launch_superexplorer))
                            .with_item_action(Rc::new(move |stable_id| {
                                activate_desktop_item(&activation_namespace, stable_id)
                            }))
                            .with_item_recycle_action(Rc::new(move |stable_id| {
                                recycle_desktop_item(
                                    &recycle_namespace,
                                    &recycle_operations,
                                    stable_id,
                                )
                            }))
                            .with_item_permanent_delete_action(Rc::new(move |stable_id| {
                                permanently_delete_desktop_item(
                                    &permanent_delete_namespace,
                                    &permanent_delete_operations,
                                    stable_id,
                                )
                            }))
                            .with_item_rename_action(Rc::new(move |stable_id, new_name| {
                                rename_desktop_item(
                                    &rename_namespace,
                                    &rename_operations,
                                    stable_id,
                                    new_name,
                                )
                            }))
                            .with_item_transfer_action(Rc::new(move |source, destination| {
                                transfer_desktop_item(
                                    &transfer_namespace,
                                    &transfer_operations,
                                    source,
                                    destination,
                                )
                            }))
                            .with_external_drop_action(Rc::new(move |paths| {
                                import_external_desktop_items(
                                    &external_drop_namespace,
                                    &external_drop_operations,
                                    paths,
                                )
                            }))
                            .with_refresh_action(Rc::new(move || {
                                refresh_desktop_namespace(&refresh_namespace, &refresh_monitor)
                            }))
                            .with_rendered_action(Rc::new(|| trace_action("frame-visible")));
                            if let Some(path) = desktop_wallpaper.clone() {
                                view = view.with_wallpaper(path);
                            }
                            if interactive {
                                view.enable_keyboard_focus(window, cx)
                            } else {
                                view
                            }
                        })
                    });
                    let Ok(desktop) = desktop else {
                        *terminal_for_app.borrow_mut() = Some(Err("desktop-window-open"));
                        cx.quit();
                        return;
                    };
                    desktop_handles.push(desktop);
                }

                if verification_surface.as_deref() == Some("desktop") {
                    continue;
                }
                let taskbar_monitor = monitor.clone();
                let taskbar_tasks = initial_tasks.clone();
                let taskbar_error = Rc::clone(&init_error);
                let taskbar_leases = Rc::clone(&leases);
                let taskbar = cx.open_window(options(&monitor, true, interactive), move |window, cx| {
                    if interactive {
                        window.activate_window();
                    }
                    let scale = taskbar_monitor.dpi_x as f32 / 96.0;
                    let width = taskbar_monitor.bounds.right - taskbar_monitor.bounds.left;
                    let height = (80.0 * scale).round() as i32;
                    let bottom = if shell {
                        taskbar_monitor.bounds.bottom
                    } else {
                        taskbar_monitor.work_area.bottom
                    };
                    let configured = if interactive { Ok(()) } else { hwnd(window).and_then(|value| {
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
                    }) };
                    if let Err(error) = configured {
                        *taskbar_error.borrow_mut() = Some(error);
                    }
                    cx.new(|cx| {
                        let focus_handle = interactive.then(|| cx.focus_handle());
                        if let Some(handle) = &focus_handle {
                            window.focus(handle, cx);
                        }
                        TaskbarView {
                        accessible_root_name: "SuperTaskbar".into(),
                        layout: TaskbarLayout::calculate(
                            2,
                            taskbar_monitor.dpi_x,
                            width as f32,
                            &[],
                            &["superexplorer".into()],
                        ),
                        tasks: taskbar_tasks,
                        fixed_name: fixed_label().into(),
                        status: status(),
                        callbacks: Some(TaskbarCallbacks {
                            start: Rc::new(|| {
                                trace_action("start");
                                let _ = platform_win::common::monitor_dpi_start::invoke_start_host_controlled();
                            }),
                            fixed: Rc::new(launch_superexplorer),
                            task: Rc::new(activate_task),
                            rendered: Rc::new(|| trace_action("frame-visible")),
                        }),
                        keyboard_focus: focus_handle,
                    }
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
            if let Ok(locale) = std::env::var("SUPERDESKTOP_LOCALE") {
                trace_action(&format!("locale:{locale}"));
            }
            if let Ok(theme) = std::env::var("SUPERDESKTOP_THEME") {
                trace_action(&format!("theme:{theme}"));
            }
            if wallpaper.is_some() {
                trace_action("wallpaper:loaded");
            }

            let refresh_handles = taskbar_handles.clone();
            if !refresh_handles.is_empty() && !state_matrix {
                let refresh_background = cx.background_executor().clone();
                let refresh_foreground = cx.foreground_executor().clone();
                let refresh_app = cx.to_async();
                refresh_foreground
                    .spawn(async move {
                        loop {
                            refresh_background.timer(Duration::from_millis(50)).await;
                            let Ok(tasks) = visible_tasks() else {
                                continue;
                            };
                            refresh_app.update(|app| {
                                let mut alive = false;
                                for handle in &refresh_handles {
                                    if handle
                                        .update(app, |view, _, cx| {
                                            alive = true;
                                            if view.tasks != tasks {
                                                view.tasks = tasks.clone();
                                                trace_action("shell-event");
                                                cx.notify();
                                            }
                                        })
                                        .is_err()
                                    {
                                        continue;
                                    }
                                }
                                if !alive {
                                    app.quit();
                                }
                            });
                        }
                    })
                    .detach();
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
