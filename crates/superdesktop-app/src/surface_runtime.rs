use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use desktop_ui::{
    AccessibleAction, AccessibleNode, DeletePolicy, DesktopItem, DesktopOperation,
    DesktopOperationController, DesktopOperationRequest, DesktopOperationTerminal,
    DesktopTransferStatus, DesktopView, MenuModel, TransferIntent, execute_desktop_operation,
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
use settings_store::{DesktopSortDirection, DesktopSortKey};
use shell_provider_protocol::{
    CURRENT_PROTOCOL, CommandDescriptor, CommandId, CommandRisk, Envelope, IconKey,
    JumpListRequest, MenuContext, MenuEnumeration, MenuInvocation, NotificationEvent,
    NotificationEventKind, NotificationHostResponse, NotificationMutation, ProviderRequest,
    ResponseBody, SearchBatch, SearchQuery, TerminalKind,
};
use taskbar_ui::{
    AccessibleTask, ClockLocale, CoreStatus, FlyoutAction, JumpListModel, JumpListView,
    NotificationAreaModel, PreviewCard, ProviderState, StartActions, StartPowerAction,
    StartSnapshot, StartView, StatusRegion, TaskAction, TaskFlyoutView, TaskViewEffect,
    TaskViewModel, TaskViewSurface, TaskbarCallbacks, TaskbarLayout, TaskbarView, TestClock,
};

use crate::{notification_client::NotificationClient, provider_client::ProviderClient};

static NEXT_PROVIDER_REQUEST: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Default)]
struct ProductionTransferSnapshot {
    status: Option<DesktopTransferStatus>,
    active_correlations: Vec<shell_core::CorrelationId>,
    terminals: Vec<(shell_core::CorrelationId, DesktopOperationTerminal)>,
    refresh_pending: bool,
}

#[derive(Clone, Default)]
struct ProductionTransferRuntime {
    active: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    snapshot: Arc<Mutex<ProductionTransferSnapshot>>,
}

impl ProductionTransferRuntime {
    fn start(&self, requests: Vec<(DesktopOperationRequest, Vec<PathBuf>, String)>) -> bool {
        if requests.is_empty()
            || self
                .active
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return false;
        }
        self.cancelled.store(false, Ordering::Release);
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.status = Some(DesktopTransferStatus {
                label: requests[0].2.clone(),
                completed_bytes: 0,
                total_bytes: 0,
                cancellable: true,
            });
            snapshot.active_correlations = requests
                .iter()
                .map(|(request, _, _)| request.correlation_id)
                .collect();
            snapshot.refresh_pending = false;
        }
        let active = Arc::clone(&self.active);
        let cancelled = Arc::clone(&self.cancelled);
        let snapshot = Arc::clone(&self.snapshot);
        std::thread::spawn(move || {
            for (request, roots, label) in requests {
                let progress = Arc::clone(&snapshot);
                let cancel = Arc::clone(&cancelled);
                let terminal =
                    execute_desktop_operation(&request, &roots, move |completed, total| {
                        if let Ok(mut current) = progress.lock() {
                            current.status = Some(DesktopTransferStatus {
                                label: label.clone(),
                                completed_bytes: completed,
                                total_bytes: total,
                                cancellable: true,
                            });
                        }
                        !cancel.load(Ordering::Acquire)
                    });
                if let Ok(mut current) = snapshot.lock() {
                    current.terminals.push((request.correlation_id, terminal));
                }
                if terminal != DesktopOperationTerminal::Succeeded {
                    break;
                }
            }
            if let Ok(mut current) = snapshot.lock() {
                current.status = None;
                current.active_correlations.clear();
                current.refresh_pending = true;
            }
            active.store(false, Ordering::Release);
        });
        true
    }

    fn cancel(&self) -> Vec<shell_core::CorrelationId> {
        self.cancelled.store(true, Ordering::Release);
        self.snapshot
            .lock()
            .map(|snapshot| snapshot.active_correlations.clone())
            .unwrap_or_default()
    }
}

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
    sort_key: DesktopSortKey,
    sort_direction: DesktopSortDirection,
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
        let properties = |stable_id: &str| {
            let path = items
                .get(stable_id)
                .map(|item| PathBuf::from(&item.activation_token));
            let metadata = path.as_ref().and_then(|path| std::fs::metadata(path).ok());
            let kind = path.as_ref().map_or_else(String::new, |path| {
                if metadata.as_ref().is_some_and(std::fs::Metadata::is_dir) {
                    "folder".into()
                } else {
                    path.extension()
                        .map(|value| value.to_string_lossy().to_lowercase())
                        .unwrap_or_default()
                }
            });
            let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
            let modified = metadata
                .and_then(|value| value.modified().ok())
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |value| value.as_millis());
            (kind, size, modified)
        };
        let (left_kind, left_size, left_modified) = properties(&left.stable_id);
        let (right_kind, right_size, right_modified) = properties(&right.stable_id);
        let primary = match sort_key {
            DesktopSortKey::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
            DesktopSortKey::Kind => left_kind.cmp(&right_kind),
            DesktopSortKey::Size => left_size.cmp(&right_size),
            DesktopSortKey::Modified => left_modified.cmp(&right_modified),
        };
        let primary = if sort_direction == DesktopSortDirection::Descending {
            primary.reverse()
        } else {
            primary
        };
        primary.then_with(|| left.stable_id.cmp(&right.stable_id))
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
    transfers: &ProductionTransferRuntime,
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
    let label = format!("Transferring {}", source_item.display_name);
    let accepted = transfers.start(vec![(request, roots, label)]);
    trace_action(if accepted {
        "desktop:transfer-started"
    } else {
        "desktop:transfer-busy-or-failed"
    });
    accepted
}

fn import_external_desktop_items(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    operations: &Rc<RefCell<DesktopOperationController>>,
    transfers: &ProductionTransferRuntime,
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
    let mut requests = Vec::new();
    for source in paths {
        let Some(name) = source.file_name() else {
            continue;
        };
        let Some(parent) = source.parent() else {
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
                continue;
            }
        };
        requests.push((
            request,
            admitted_roots,
            format!("Copying {}", name.to_string_lossy()),
        ));
    }
    let accepted = transfers.start(requests);
    trace_action(if accepted {
        "desktop:external-drop-started"
    } else {
        "desktop:external-drop-busy-or-rejected"
    });
    accepted
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn provider_request(payload: ProviderRequest, deadline_ms: u64) -> Envelope<ProviderRequest> {
    let sequence = NEXT_PROVIDER_REQUEST.fetch_add(1, Ordering::Relaxed);
    Envelope {
        protocol: CURRENT_PROTOCOL,
        request_id: format!("surface-{sequence}"),
        correlation_id: format!("surface-correlation-{sequence}"),
        deadline_unix_ms: Some(unix_time_ms().saturating_add(deadline_ms)),
        payload,
    }
}

fn search_start(provider: &Rc<RefCell<ProviderClient>>, query: SearchQuery) -> Vec<SearchBatch> {
    let request = provider_request(ProviderRequest::Search(query), 2_000);
    provider
        .borrow_mut()
        .request(&request, Duration::from_secs(2))
        .ok()
        .and_then(|response| {
            (response.terminal == TerminalKind::Success)
                .then_some(response.body)
                .and_then(|body| match body {
                    ResponseBody::Search(batches) => Some(batches),
                    _ => None,
                })
        })
        .unwrap_or_default()
}

fn query_jump_list(
    provider: &Rc<RefCell<ProviderClient>>,
    application_id: &str,
    local: Vec<CommandDescriptor>,
) -> JumpListModel {
    let request = provider_request(
        ProviderRequest::JumpList(JumpListRequest {
            application_id: application_id.into(),
        }),
        2_000,
    );
    let response = provider
        .borrow_mut()
        .request(&request, Duration::from_millis(500))
        .ok()
        .and_then(|response| {
            (response.terminal == TerminalKind::Success)
                .then_some(response.body)
                .and_then(|body| match body {
                    ResponseBody::JumpList(list) => Some(list),
                    _ => None,
                })
        })
        .unwrap_or_default();
    JumpListModel::compose(response.recent, response.frequent, response.tasks, local)
}

fn activate_jump_command(command: &CommandDescriptor) -> bool {
    let id = command.id.0.as_str();
    let path = id
        .strip_prefix("jump:open:")
        .or_else(|| id.strip_prefix("jump:launch:"));
    path.is_some_and(|path| {
        platform_win::common::desktop::launch_association(Path::new(path)).is_ok()
    })
}

fn send_notification_event(
    client: &Rc<RefCell<NotificationClient>>,
    key: &IconKey,
    kind: NotificationEventKind,
) {
    let sequence = NEXT_PROVIDER_REQUEST.fetch_add(1, Ordering::Relaxed);
    let request = NotificationMutation::Event {
        event: NotificationEvent {
            correlation_id: format!("notification-event-{sequence}"),
            key: key.clone(),
            kind,
            admitted_unix_ms: unix_time_ms(),
        },
    };
    let completed = client
        .borrow_mut()
        .request(&request, Duration::from_millis(100))
        .is_ok_and(|response| {
            matches!(
                response,
                NotificationHostResponse::Accepted { changed: false, .. }
            )
        });
    trace_action(if completed {
        "notification:event-completed"
    } else {
        "notification:event-rejected"
    });
}

fn activate_start_command(command: &CommandDescriptor) {
    let id = command.id.0.as_str();
    let outcome = if let Some(uri) = id.strip_prefix("settings:") {
        platform_win::common::desktop::launch_settings_uri(uri).is_ok()
    } else if let Some(path) = id.strip_prefix("open:") {
        platform_win::common::desktop::launch_association(Path::new(path)).is_ok()
    } else {
        false
    };
    trace_action(if outcome {
        "start:activation-succeeded"
    } else {
        "start:activation-rejected"
    });
}

fn local_context_menu(stable_id: &str) -> Option<MenuModel> {
    let command = |id: &str, label: &str, risk| CommandDescriptor {
        id: CommandId(format!("local:{id}")),
        label: label.into(),
        enabled: true,
        risk,
        children: Vec::new(),
    };
    let commands = if stable_id == "desktop-background" {
        vec![
            command("refresh", "Refresh", CommandRisk::Normal),
            command("sort-name", "Sort by name", CommandRisk::Normal),
            command("sort-kind", "Sort by kind", CommandRisk::Normal),
            command("sort-size", "Sort by size", CommandRisk::Normal),
            command("sort-modified", "Sort by modified", CommandRisk::Normal),
            command("sort-ascending", "Ascending", CommandRisk::Normal),
            command("sort-descending", "Descending", CommandRisk::Normal),
            command("new", "New folder", CommandRisk::Normal),
        ]
    } else {
        vec![
            command("open", "Open", CommandRisk::Normal),
            command("rename", "Rename", CommandRisk::Normal),
            command("recycle", "Delete", CommandRisk::Destructive),
            command("properties", "Properties", CommandRisk::Normal),
        ]
    };
    MenuModel::new(MenuEnumeration {
        generation: 0,
        selection_fingerprint: stable_id.into(),
        commands,
        optional_enrichment_complete: false,
    })
    .ok()
}

fn enumerate_desktop_context_menu(
    runtime: &Rc<RefCell<DesktopNamespaceRuntime>>,
    provider: &Rc<RefCell<ProviderClient>>,
    stable_id: &str,
) -> Option<MenuModel> {
    let background = stable_id == "desktop-background";
    if !background && !runtime.borrow().items.contains_key(stable_id) {
        return None;
    }
    if background {
        return local_context_menu(stable_id);
    }
    let request = provider_request(
        ProviderRequest::ContextMenuEnumerate(MenuContext {
            selection_fingerprint: stable_id.into(),
            selection_count: usize::from(!background),
            background,
            can_open: !background,
            can_rename: !background,
            can_delete: !background,
            can_show_properties: !background,
        }),
        2_000,
    );
    let menu = provider
        .borrow_mut()
        .request(&request, Duration::from_millis(200))
        .ok()
        .and_then(|response| {
            (response.terminal == TerminalKind::Success)
                .then_some(response.body)
                .and_then(|body| match body {
                    ResponseBody::Menu(menu) => MenuModel::new(menu).ok(),
                    _ => None,
                })
        });
    if menu.is_some() {
        trace_action("desktop:context-menu-provider");
        menu
    } else {
        trace_action("desktop:context-menu-fallback");
        local_context_menu(stable_id)
    }
}

fn invoke_desktop_context_menu(
    provider: &Rc<RefCell<ProviderClient>>,
    invocation: &MenuInvocation,
) -> Option<String> {
    if let Some(command) = invocation.token.strip_prefix("local:") {
        return matches!(
            command,
            "open"
                | "rename"
                | "recycle"
                | "properties"
                | "refresh"
                | "sort-name"
                | "sort-kind"
                | "sort-size"
                | "sort-modified"
                | "sort-ascending"
                | "sort-descending"
                | "new"
        )
        .then(|| command.to_owned());
    }
    let request = provider_request(
        ProviderRequest::ContextMenuInvoke(invocation.clone()),
        2_000,
    );
    provider
        .borrow_mut()
        .request(&request, Duration::from_secs(2))
        .ok()
        .and_then(|response| {
            (response.terminal == TerminalKind::Success)
                .then_some(response.body)
                .and_then(|body| match body {
                    ResponseBody::MenuInvocation(result) => Some(result.command_id),
                    _ => None,
                })
        })
}

fn create_desktop_folder(runtime: &Rc<RefCell<DesktopNamespaceRuntime>>) -> bool {
    let (Some(user_root), allowed_roots) = ({
        let runtime = runtime.borrow();
        (runtime.user_root.clone(), runtime.allowed_roots.clone())
    }) else {
        trace_action("desktop:new-folder-root-unavailable");
        return false;
    };
    let created = platform_win::common::desktop_operations::create_directory(
        &user_root,
        "New folder",
        &allowed_roots,
    )
    .is_ok();
    trace_action(if created {
        "desktop:new-folder-created"
    } else {
        "desktop:new-folder-failed"
    });
    created
}

fn show_desktop_item_properties(runtime: &Rc<RefCell<DesktopNamespaceRuntime>>, stable_id: &str) {
    let path = runtime
        .borrow()
        .items
        .get(stable_id)
        .map(|item| PathBuf::from(&item.activation_token));
    if path
        .as_deref()
        .is_some_and(|path| platform_win::common::desktop::show_properties(path).is_ok())
    {
        trace_action("desktop:properties-opened");
    } else {
        trace_action("desktop:properties-failed");
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

fn task_hwnd(stable_id: &str) -> Option<isize> {
    let hex = stable_id.rsplit(':').next()?;
    usize::from_str_radix(hex, 16)
        .ok()
        .map(|value| value as isize)
}

fn apply_flyout_action(action: FlyoutAction) {
    let (window_id, action) = match action {
        FlyoutAction::Activate(window_id) => (
            window_id,
            platform_win::common::taskbar::WindowAction::RestoreAndActivate,
        ),
        FlyoutAction::Close(window_id) => (
            window_id,
            platform_win::common::taskbar::WindowAction::Close,
        ),
    };
    let outcome = task_hwnd(window_id.as_str())
        .and_then(|hwnd| platform_win::common::taskbar::apply_window_action(hwnd, action).ok())
        .is_some();
    trace_action(if outcome {
        "task-flyout:action-succeeded"
    } else {
        "task-flyout:action-rejected"
    });
}

fn apply_task_view_effect(effect: TaskViewEffect) -> bool {
    let TaskViewEffect::MoveWindow {
        window_id,
        desktop_id,
    } = effect
    else {
        trace_action("task-view:unsupported-effect-rejected");
        return false;
    };
    let completed = task_hwnd(window_id.as_str()).is_some_and(|hwnd| {
        platform_win::common::virtual_desktop::move_window_to_desktop(hwnd, false, desktop_id)
            .is_ok()
            && platform_win::common::virtual_desktop::window_desktop_id(hwnd, false)
                .is_ok_and(|observed| observed == desktop_id)
    });
    trace_action(if completed {
        "task-view:move-succeeded"
    } else {
        "task-view:move-rejected"
    });
    completed
}

fn group_window_ids(stable_id: &str) -> Vec<isize> {
    stable_id
        .strip_prefix("task-group:")
        .into_iter()
        .flat_map(|ids| ids.split(','))
        .filter_map(|value| usize::from_str_radix(value, 16).ok())
        .map(|value| value as isize)
        .collect()
}

fn visible_tasks(
    pin_order: &[String],
    combine_groups: bool,
) -> Result<Vec<AccessibleTask>, &'static str> {
    snapshot_task_windows()
        .map_err(|_| "task-window-snapshot")
        .map(|windows| {
            let mut grouped = BTreeMap::<String, Vec<_>>::new();
            for window in windows.into_iter().filter(|window| {
                window.visible && !window.tool_window && !window.cloaked && !window.owned_transient
            }) {
                grouped
                    .entry(window.application_identity.clone())
                    .or_default()
                    .push(window);
            }
            let mut groups = if combine_groups {
                grouped.into_iter().collect::<Vec<_>>()
            } else {
                grouped
                    .into_iter()
                    .flat_map(|(application, windows)| {
                        windows
                            .into_iter()
                            .map(move |window| (application.clone(), vec![window]))
                    })
                    .collect::<Vec<_>>()
            };
            groups.sort_by(|(left, _), (right, _)| {
                let left_pin = pin_order.iter().position(|pin| pin == left);
                let right_pin = pin_order.iter().position(|pin| pin == right);
                left_pin
                    .unwrap_or(usize::MAX)
                    .cmp(&right_pin.unwrap_or(usize::MAX))
                    .then_with(|| left.cmp(right))
            });
            groups
                .into_iter()
                .take(16)
                .map(|(application, windows)| {
                    let group_size = windows.len();
                    let stable_id = if group_size == 1 {
                        windows[0].window_identity.clone()
                    } else {
                        format!(
                            "task-group:{}",
                            windows
                                .iter()
                                .map(|window| format!("{:X}", window.hwnd_identity as usize))
                                .collect::<Vec<_>>()
                                .join(",")
                        )
                    };
                    let name = if group_size == 1 && !windows[0].title.is_empty() {
                        windows[0].title.clone()
                    } else {
                        Path::new(&application)
                            .file_stem()
                            .and_then(|value| value.to_str())
                            .unwrap_or(&application)
                            .to_owned()
                    };
                    AccessibleTask {
                        stable_id,
                        name,
                        role: "button",
                        active: windows.iter().any(|window| window.foreground),
                        minimized: windows.iter().all(|window| window.minimized),
                        attention: false,
                        group_size,
                        available: true,
                        actions: vec![
                            TaskAction::Focus,
                            TaskAction::Select,
                            TaskAction::Invoke,
                            TaskAction::Minimize,
                            TaskAction::Restore,
                        ],
                    }
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

fn options(
    monitor: &MonitorRecord,
    taskbar: bool,
    interactive: bool,
    taskbar_rows: u8,
) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = (monitor.bounds.right - monitor.bounds.left) as f32 / scale;
    let height = if taskbar {
        40.0 * f32::from(taskbar_rows.clamp(1, 3))
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

fn start_options(monitor: &MonitorRecord) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let monitor_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let monitor_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    let width = monitor_width.min(560.0);
    let height = monitor_height.min(680.0);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px(monitor.work_area.left as f32 / scale),
                px(monitor.work_area.bottom as f32 / scale - height),
            ),
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

fn task_flyout_options(monitor: &MonitorRecord, card_count: usize) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = (card_count.clamp(1, 4) as f32 * 228.0 + 16.0).min(928.0);
    let height = 260.0;
    let monitor_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let left = monitor.work_area.left as f32 / scale + (monitor_width - width).max(0.0) / 2.0;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px(left),
                px(monitor.work_area.bottom as f32 / scale - height),
            ),
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

fn jump_list_options(monitor: &MonitorRecord) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = 360.0;
    let height = 480.0_f32.min((monitor.work_area.bottom - monitor.work_area.top) as f32 / scale);
    let monitor_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let left = monitor.work_area.left as f32 / scale + (monitor_width - width).max(0.0) / 2.0;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px(left),
                px(monitor.work_area.bottom as f32 / scale - height),
            ),
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

fn task_view_options(monitor: &MonitorRecord) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px(monitor.work_area.left as f32 / scale),
                px(monitor.work_area.top as f32 / scale),
            ),
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

fn observed_task_view_model() -> TaskViewModel {
    let capabilities = platform_win::common::virtual_desktop::probe_capabilities();
    let mut model = TaskViewModel::new(capabilities);
    let mut membership = BTreeMap::new();
    if capabilities.query_window
        && let Ok(windows) = snapshot_task_windows()
    {
        for window in windows.into_iter().filter(|window| {
            window.visible && !window.tool_window && !window.cloaked && !window.owned_transient
        }) {
            let Ok(desktop_id) = platform_win::common::virtual_desktop::window_desktop_id(
                window.hwnd_identity,
                false,
            ) else {
                continue;
            };
            let Ok(window_id) = shell_core::WindowId::new(window.window_identity) else {
                continue;
            };
            membership
                .entry(desktop_id)
                .or_insert_with(Vec::new)
                .push(window_id);
        }
    }
    model.observed_membership(membership);
    model
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
    let (mut settings_store, settings_target) =
        platform_win::common::settings_file::production_settings_store()
            .map_err(|_| "settings-store-init")?;
    let persisted_settings = settings_store
        .load(&settings_target)
        .map_err(|_| "settings-store-load")?
        .settings;
    let state_matrix = std::env::var_os("SUPERDESKTOP_VERIFICATION_STATE_MATRIX").is_some();
    let initial_tasks = if state_matrix {
        verification_state_tasks()
    } else {
        visible_tasks(
            &persisted_settings.taskbar.pins,
            persisted_settings.taskbar.combine_groups,
        )?
    };
    let wallpaper = std::env::var_os("SUPERDESKTOP_WALLPAPER_PATH")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| current_wallpaper_path().ok());
    let verification_surface = std::env::var("SUPERDESKTOP_VERIFICATION_SURFACE").ok();
    let interactive = verification_surface.is_some();
    let desktop_namespace = Rc::new(RefCell::new(DesktopNamespaceRuntime::default()));
    let desktop_operations = Rc::new(RefCell::new(DesktopOperationController::default()));
    let desktop_transfers = ProductionTransferRuntime::default();
    let provider_client = Rc::new(RefCell::new(ProviderClient::adjacent()?));
    let notification_client = Rc::new(RefCell::new(NotificationClient::adjacent()?));
    let settings_store = Rc::new(RefCell::new(settings_store));
    let settings_target = Rc::new(settings_target);
    let persisted_settings = Rc::new(RefCell::new(persisted_settings));
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
                    let desktop_transfers_for_view = desktop_transfers.clone();
                    let provider_client_for_view = Rc::clone(&provider_client);
                    let desktop_settings_for_view = Rc::clone(&persisted_settings);
                    let desktop_store_for_view = Rc::clone(&settings_store);
                    let desktop_target_for_view = Rc::clone(&settings_target);
                    let desktop = cx.open_window(options(&monitor, false, interactive, 2), move |window, cx| {
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
                            let desktop_settings = desktop_settings_for_view.borrow().desktop.clone();
                            let nodes = refresh_desktop_namespace(
                                &desktop_namespace_for_view,
                                &monitor_key,
                                desktop_settings.sort_key,
                                desktop_settings.sort_direction,
                            );
                            let item_positions = desktop_settings_for_view
                                .borrow()
                                .desktop_positions
                                .iter()
                                .filter(|position| position.monitor_id == monitor_key)
                                .map(|position| {
                                    (
                                        position.item_id.clone(),
                                        (position.logical_x as f32, position.logical_y as f32),
                                    )
                                })
                                .collect::<BTreeMap<_, _>>();
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
                            let transfer_runtime = desktop_transfers_for_view.clone();
                            let external_drop_namespace = Rc::clone(&desktop_namespace_for_view);
                            let external_drop_operations = Rc::clone(&desktop_operations_for_view);
                            let external_drop_runtime = desktop_transfers_for_view.clone();
                            let context_namespace = Rc::clone(&desktop_namespace_for_view);
                            let context_provider = Rc::clone(&provider_client_for_view);
                            let invocation_provider = Rc::clone(&provider_client_for_view);
                            let properties_namespace = Rc::clone(&desktop_namespace_for_view);
                            let new_folder_namespace = Rc::clone(&desktop_namespace_for_view);
                            let refresh_namespace = Rc::clone(&desktop_namespace_for_view);
                            let refresh_monitor = monitor_key.clone();
                            let refresh_settings = Rc::clone(&desktop_settings_for_view);
                            let sort_namespace = Rc::clone(&desktop_namespace_for_view);
                            let sort_monitor = monitor_key.clone();
                            let sort_settings = Rc::clone(&desktop_settings_for_view);
                            let sort_store = Rc::clone(&desktop_store_for_view);
                            let sort_target = Rc::clone(&desktop_target_for_view);
                            let position_settings = Rc::clone(&desktop_settings_for_view);
                            let position_store = Rc::clone(&desktop_store_for_view);
                            let position_target = Rc::clone(&desktop_target_for_view);
                            let position_monitor = monitor_key.clone();
                            let cancel_transfer_runtime = desktop_transfers_for_view.clone();
                            let cancel_transfer_operations =
                                Rc::clone(&desktop_operations_for_view);
                            let mut view = DesktopView::new(
                                nodes,
                                false,
                            )
                            .with_item_positions(item_positions)
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
                                    &transfer_runtime,
                                    source,
                                    destination,
                                )
                            }))
                            .with_item_reposition_action(Rc::new(move |stable_id, x, y| {
                                let mut settings = position_settings.borrow().clone();
                                let next_revision = settings
                                    .desktop_positions
                                    .iter()
                                    .map(|position| position.layout_revision)
                                    .max()
                                    .unwrap_or_default()
                                    .saturating_add(1);
                                settings.desktop_positions.retain(|position| {
                                    position.monitor_id != position_monitor
                                        || position.item_id != stable_id
                                });
                                settings.desktop_positions.push(settings_store::DesktopPosition {
                                    monitor_id: position_monitor.clone(),
                                    item_id: stable_id.to_owned(),
                                    logical_x: x.round() as i32,
                                    logical_y: y.round() as i32,
                                    layout_revision: next_revision,
                                });
                                match position_store.borrow_mut().save(&position_target, &settings) {
                                    Ok(saved) => {
                                        *position_settings.borrow_mut() = saved;
                                        trace_action("desktop:position-persisted");
                                    }
                                    Err(_) => trace_action("desktop:position-persist-failed"),
                                }
                            }))
                            .with_external_drop_action(Rc::new(move |paths| {
                                import_external_desktop_items(
                                    &external_drop_namespace,
                                    &external_drop_operations,
                                    &external_drop_runtime,
                                    paths,
                                )
                            }))
                            .with_cancel_transfer_action(Rc::new(move || {
                                for correlation_id in cancel_transfer_runtime.cancel() {
                                    let _ = cancel_transfer_operations
                                        .borrow_mut()
                                        .cancel(correlation_id);
                                }
                                trace_action("desktop:transfer-cancel-requested");
                            }))
                            .with_context_menu_action(Rc::new(move |stable_id| {
                                enumerate_desktop_context_menu(
                                    &context_namespace,
                                    &context_provider,
                                    stable_id,
                                )
                            }))
                            .with_context_invoke_action(Rc::new(move |invocation| {
                                invoke_desktop_context_menu(&invocation_provider, invocation)
                            }))
                            .with_item_properties_action(Rc::new(move |stable_id| {
                                show_desktop_item_properties(&properties_namespace, stable_id)
                            }))
                            .with_background_new_action(Rc::new(move || {
                                create_desktop_folder(&new_folder_namespace)
                            }))
                            .with_refresh_action(Rc::new(move || {
                                let desktop = refresh_settings.borrow().desktop.clone();
                                refresh_desktop_namespace(
                                    &refresh_namespace,
                                    &refresh_monitor,
                                    desktop.sort_key,
                                    desktop.sort_direction,
                                )
                            }))
                            .with_sort_action(Rc::new(move |command| {
                                let mut settings = sort_settings.borrow().clone();
                                match command {
                                    "sort-name" => settings.desktop.sort_key = DesktopSortKey::Name,
                                    "sort-kind" => settings.desktop.sort_key = DesktopSortKey::Kind,
                                    "sort-size" => settings.desktop.sort_key = DesktopSortKey::Size,
                                    "sort-modified" => settings.desktop.sort_key = DesktopSortKey::Modified,
                                    "sort-ascending" => settings.desktop.sort_direction = DesktopSortDirection::Ascending,
                                    "sort-descending" => settings.desktop.sort_direction = DesktopSortDirection::Descending,
                                    _ => {}
                                }
                                if let Ok(saved) = sort_store.borrow_mut().save(&sort_target, &settings) {
                                    *sort_settings.borrow_mut() = saved;
                                }
                                let desktop = sort_settings.borrow().desktop.clone();
                                refresh_desktop_namespace(
                                    &sort_namespace,
                                    &sort_monitor,
                                    desktop.sort_key,
                                    desktop.sort_direction,
                                )
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
                if !persisted_settings.borrow().taskbar.all_monitors && !monitor.primary {
                    continue;
                }
                let taskbar_monitor = monitor.clone();
                let taskbar_tasks = initial_tasks.clone();
                let taskbar_error = Rc::clone(&init_error);
                let taskbar_leases = Rc::clone(&leases);
                let start_window = Rc::new(RefCell::new(None::<gpui::WindowHandle<StartView>>));
                let start_window_for_taskbar = Rc::clone(&start_window);
                let start_provider_for_taskbar = Rc::clone(&provider_client);
                let start_settings_store = Rc::clone(&settings_store);
                let start_settings_target = Rc::clone(&settings_target);
                let start_persisted_settings = Rc::clone(&persisted_settings);
                let start_monitor = taskbar_monitor.clone();
                let flyout_window =
                    Rc::new(RefCell::new(None::<gpui::WindowHandle<TaskFlyoutView>>));
                let flyout_window_for_taskbar = Rc::clone(&flyout_window);
                let flyout_monitor = taskbar_monitor.clone();
                let jump_list_window =
                    Rc::new(RefCell::new(None::<gpui::WindowHandle<JumpListView>>));
                let jump_list_window_for_taskbar = Rc::clone(&jump_list_window);
                let jump_list_monitor = taskbar_monitor.clone();
                let jump_list_provider = Rc::clone(&provider_client);
                let jump_settings_store = Rc::clone(&settings_store);
                let jump_settings_target = Rc::clone(&settings_target);
                let jump_persisted_settings = Rc::clone(&persisted_settings);
                let task_view_window =
                    Rc::new(RefCell::new(None::<gpui::WindowHandle<TaskViewSurface>>));
                let task_view_window_for_taskbar = Rc::clone(&task_view_window);
                let task_view_monitor = taskbar_monitor.clone();
                let notification_client_for_taskbar = Rc::clone(&notification_client);
                let production_taskbar_settings = persisted_settings.borrow().taskbar.clone();
                let taskbar = cx.open_window(
                    options(
                        &monitor,
                        true,
                        interactive,
                        production_taskbar_settings.rows,
                    ),
                    move |window, cx| {
                    if interactive {
                        window.activate_window();
                    }
                    let scale = taskbar_monitor.dpi_x as f32 / 96.0;
                    let width = taskbar_monitor.bounds.right - taskbar_monitor.bounds.left;
                    let height = (40.0
                        * f32::from(production_taskbar_settings.rows.clamp(1, 3))
                        * scale)
                        .round() as i32;
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
                    cx.new(move |cx| {
                        let focus_handle = interactive.then(|| cx.focus_handle());
                        if let Some(handle) = &focus_handle {
                            window.focus(handle, cx);
                        }
                        let taskbar_overlays = taskbar_tasks
                            .iter()
                            .filter(|task| task.attention)
                            .map(|task| {
                                (
                                    task.stable_id.clone(),
                                    taskbar_ui::TaskOverlay {
                                        attention: true,
                                        ..taskbar_ui::TaskOverlay::default()
                                    },
                                )
                            })
                            .collect();
                        TaskbarView {
                        accessible_root_name: "SuperTaskbar".into(),
                        layout: TaskbarLayout::calculate(
                            production_taskbar_settings.rows,
                            taskbar_monitor.dpi_x,
                            width as f32,
                            &[],
                            &["superexplorer".into()],
                        ),
                        tasks: taskbar_tasks,
                        fixed_name: fixed_label().into(),
                        status: status(),
                        notification_area: NotificationAreaModel::default(),
                        overlays: taskbar_overlays,
                        show_labels: production_taskbar_settings.show_labels,
                        callbacks: Some(TaskbarCallbacks {
                            start: Rc::new(move |app| {
                                trace_action("start");
                                if !shell {
                                    let _ = platform_win::common::monitor_dpi_start::invoke_start_host_controlled();
                                    return;
                                }
                                if let Some(existing) = *start_window_for_taskbar.borrow() {
                                    if existing
                                        .update(app, |_, window, _| window.remove_window())
                                        .is_ok()
                                    {
                                        *start_window_for_taskbar.borrow_mut() = None;
                                        trace_action("start:closed");
                                        return;
                                    }
                                    *start_window_for_taskbar.borrow_mut() = None;
                                }
                                let mut catalog = platform_win::common::start_search::settings_catalog();
                                catalog.extend(platform_win::common::start_search::discover_applications(
                                    &platform_win::common::start_search::default_application_roots(),
                                    80,
                                ));
                                let search_provider = Rc::clone(&start_provider_for_taskbar);
                                let dismiss_slot = Rc::clone(&start_window_for_taskbar);
                                let snapshot = {
                                    let settings = start_persisted_settings.borrow();
                                    StartSnapshot {
                                        initialized: settings.start.initialized,
                                        pinned_ids: settings.start.pinned_ids.clone(),
                                        recent_ids: settings.start.recent_ids.clone(),
                                    }
                                };
                                for id in snapshot
                                    .pinned_ids
                                    .iter()
                                    .chain(&snapshot.recent_ids)
                                {
                                    if !catalog.iter().any(|item| item.id == *id)
                                        && let Some(item) = platform_win::common::start_search::restore_persisted_result(id)
                                    {
                                        catalog.push(item);
                                    }
                                }
                                let persist_store = Rc::clone(&start_settings_store);
                                let persist_target = Rc::clone(&start_settings_target);
                                let persist_settings = Rc::clone(&start_persisted_settings);
                                let opened = app.open_window(start_options(&start_monitor), move |window, cx| {
                                    window.activate_window();
                                    let provider = Rc::clone(&search_provider);
                                    let dismiss_slot = Rc::clone(&dismiss_slot);
                                    cx.new(move |cx| {
                                        StartView::new(
                                            catalog,
                                            snapshot,
                                            StartActions {
                                                search: Rc::new(move |query| search_start(&provider, query)),
                                                activate: Rc::new(activate_start_command),
                                                dismiss: Rc::new(move |window, _| {
                                                    window.remove_window();
                                                    *dismiss_slot.borrow_mut() = None;
                                                    trace_action("start:closed");
                                                }),
                                                persist: Rc::new(move |snapshot| {
                                                    let mut settings = persist_settings.borrow().clone();
                                                    settings.start.initialized = snapshot.initialized;
                                                    settings.start.pinned_ids = snapshot.pinned_ids.clone();
                                                    settings.start.recent_ids = snapshot.recent_ids.clone();
                                                    match persist_store.borrow_mut().save(&persist_target, &settings) {
                                                        Ok(saved) => {
                                                            *persist_settings.borrow_mut() = saved;
                                                            trace_action("start:snapshot-persisted");
                                                        }
                                                        Err(_) => trace_action("start:snapshot-persist-failed"),
                                                    }
                                                }),
                                                power: Rc::new(move |action| {
                                                    let action = match action {
                                                        StartPowerAction::SignOut => platform_win::common::power::SessionPowerAction::SignOut,
                                                        StartPowerAction::Restart => platform_win::common::power::SessionPowerAction::Restart,
                                                        StartPowerAction::ShutDown => platform_win::common::power::SessionPowerAction::ShutDown,
                                                    };
                                                    match platform_win::common::power::confirm_and_execute(action) {
                                                        Ok(true) => trace_action("start:power-accepted"),
                                                        Ok(false) => trace_action("start:power-cancelled"),
                                                        Err(_) => trace_action("start:power-failed"),
                                                    }
                                                }),
                                            },
                                            cx,
                                        )
                                    })
                                });
                                match opened {
                                    Ok(handle) => {
                                        *start_window_for_taskbar.borrow_mut() = Some(handle);
                                        trace_action("start:owned-opened");
                                    }
                                    Err(_) => trace_action("start:owned-open-failed"),
                                }
                            }),
                            task_view: Rc::new(move |app| {
                                if let Some(existing) = *task_view_window_for_taskbar.borrow() {
                                    if existing
                                        .update(app, |_, window, _| window.remove_window())
                                        .is_ok()
                                    {
                                        *task_view_window_for_taskbar.borrow_mut() = None;
                                        trace_action("task-view:closed");
                                        return;
                                    }
                                    *task_view_window_for_taskbar.borrow_mut() = None;
                                }
                                let model = observed_task_view_model();
                                let dismiss_slot = Rc::clone(&task_view_window_for_taskbar);
                                let opened = app.open_window(
                                    task_view_options(&task_view_monitor),
                                    move |window, cx| {
                                        window.activate_window();
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            TaskViewSurface::new(
                                                model,
                                                Rc::new(apply_task_view_effect),
                                                Rc::new(move |window, _| {
                                                    window.remove_window();
                                                    *dismiss_slot.borrow_mut() = None;
                                                    trace_action("task-view:closed");
                                                }),
                                                cx,
                                            )
                                        })
                                    },
                                );
                                if let Ok(handle) = opened {
                                    *task_view_window_for_taskbar.borrow_mut() = Some(handle);
                                    trace_action("task-view:opened");
                                }
                            }),
                            fixed: Rc::new(launch_superexplorer),
                            task: Rc::new(move |stable_id, app| {
                                let group_ids = group_window_ids(stable_id);
                                if group_ids.is_empty() {
                                    activate_task(stable_id);
                                    return;
                                }
                                if let Some(existing) = *flyout_window_for_taskbar.borrow() {
                                    if existing
                                        .update(app, |_, window, _| window.remove_window())
                                        .is_ok()
                                    {
                                        *flyout_window_for_taskbar.borrow_mut() = None;
                                        return;
                                    }
                                    *flyout_window_for_taskbar.borrow_mut() = None;
                                }
                                let Ok(windows) = snapshot_task_windows() else {
                                    return;
                                };
                                let now = unix_time_ms();
                                let cards = windows
                                    .into_iter()
                                    .filter(|window| group_ids.contains(&window.hwnd_identity))
                                    .filter_map(|window| {
                                        let window_id = shell_core::WindowId::new(window.window_identity).ok()?;
                                        let preview_available = production_taskbar_settings.previews_enabled && matches!(
                                            platform_win::common::taskbar_preview::admit_live_preview(
                                                window.hwnd_identity,
                                                false,
                                                now,
                                                unix_time_ms(),
                                            ),
                                            platform_win::common::taskbar_preview::PreviewAdmission::Available { .. }
                                        );
                                        Some(PreviewCard {
                                            window_id,
                                            title: if window.title.is_empty() {
                                                window.application_identity
                                            } else {
                                                window.title
                                            },
                                            minimized: window.minimized,
                                            preview_available,
                                            preview_source: preview_available.then_some(window.hwnd_identity),
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                if cards.is_empty() {
                                    return;
                                }
                                let dismiss_slot = Rc::clone(&flyout_window_for_taskbar);
                                let opened = app.open_window(
                                    task_flyout_options(&flyout_monitor, cards.len()),
                                    move |window, cx| {
                                        window.activate_window();
                                        let destination_hwnd = hwnd(window).unwrap_or_default();
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            TaskFlyoutView::new(
                                                cards,
                                                Rc::new(apply_flyout_action),
                                                Rc::new(move |window, _| {
                                                    window.remove_window();
                                                    *dismiss_slot.borrow_mut() = None;
                                                }),
                                                destination_hwnd,
                                                cx,
                                            )
                                        })
                                    },
                                );
                                if let Ok(handle) = opened {
                                    *flyout_window_for_taskbar.borrow_mut() = Some(handle);
                                }
                            }),
                            task_context: Rc::new(move |stable_id, app| {
                                if let Some(existing) = *jump_list_window_for_taskbar.borrow() {
                                    if existing
                                        .update(app, |_, window, _| window.remove_window())
                                        .is_ok()
                                    {
                                        *jump_list_window_for_taskbar.borrow_mut() = None;
                                        return;
                                    }
                                    *jump_list_window_for_taskbar.borrow_mut() = None;
                                }
                                let Ok(windows) = snapshot_task_windows() else {
                                    return;
                                };
                                let selected_ids = {
                                    let group = group_window_ids(stable_id);
                                    if group.is_empty() {
                                        task_hwnd(stable_id).into_iter().collect::<Vec<_>>()
                                    } else {
                                        group
                                    }
                                };
                                let Some(application_id) = windows
                                    .iter()
                                    .find(|window| selected_ids.contains(&window.hwnd_identity))
                                    .map(|window| window.application_identity.clone())
                                else {
                                    return;
                                };
                                let application_windows = windows
                                    .iter()
                                    .filter(|window| window.application_identity == application_id)
                                    .map(|window| window.hwnd_identity)
                                    .collect::<Vec<_>>();
                                let pinned = jump_persisted_settings
                                    .borrow()
                                    .taskbar
                                    .pins
                                    .contains(&application_id);
                                let local = vec![
                                    CommandDescriptor {
                                        id: CommandId("local:taskbar-pin".into()),
                                        label: if pinned { "Unpin from taskbar" } else { "Pin to taskbar" }.into(),
                                        enabled: true,
                                        risk: CommandRisk::Normal,
                                        children: Vec::new(),
                                    },
                                    CommandDescriptor {
                                        id: CommandId("local:taskbar-close-all".into()),
                                        label: "Close all windows".into(),
                                        enabled: !application_windows.is_empty(),
                                        risk: CommandRisk::Destructive,
                                        children: Vec::new(),
                                    },
                                ];
                                let model = query_jump_list(&jump_list_provider, &application_id, local);
                                let dismiss_slot = Rc::clone(&jump_list_window_for_taskbar);
                                let invoke_store = Rc::clone(&jump_settings_store);
                                let invoke_target = Rc::clone(&jump_settings_target);
                                let invoke_settings = Rc::clone(&jump_persisted_settings);
                                let invoke_application = application_id.clone();
                                let opened = app.open_window(
                                    jump_list_options(&jump_list_monitor),
                                    move |window, cx| {
                                        window.activate_window();
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            JumpListView::new(
                                                model,
                                                Rc::new(move |command| {
                                                    let completed = match command.id.0.as_str() {
                                                        "local:taskbar-pin" => {
                                                            let mut settings = invoke_settings.borrow().clone();
                                                            if pinned {
                                                                settings.taskbar.pins.retain(|pin| pin != &invoke_application);
                                                            } else if !settings.taskbar.pins.contains(&invoke_application) {
                                                                settings.taskbar.pins.push(invoke_application.clone());
                                                            }
                                                            match invoke_store.borrow_mut().save(&invoke_target, &settings) {
                                                                Ok(saved) => {
                                                                    *invoke_settings.borrow_mut() = saved;
                                                                    true
                                                                }
                                                                Err(_) => false,
                                                            }
                                                        }
                                                        "local:taskbar-close-all" => application_windows.iter().all(|hwnd| {
                                                            platform_win::common::taskbar::apply_window_action(
                                                                *hwnd,
                                                                platform_win::common::taskbar::WindowAction::Close,
                                                            )
                                                            .is_ok()
                                                        }),
                                                        _ => activate_jump_command(command),
                                                    };
                                                    trace_action(if completed {
                                                        "taskbar:jump-list-action-succeeded"
                                                    } else {
                                                        "taskbar:jump-list-action-rejected"
                                                    });
                                                }),
                                                Rc::new(move |window, _| {
                                                    window.remove_window();
                                                    *dismiss_slot.borrow_mut() = None;
                                                }),
                                                cx,
                                            )
                                        })
                                    },
                                );
                                if let Ok(handle) = opened {
                                    *jump_list_window_for_taskbar.borrow_mut() = Some(handle);
                                    trace_action("taskbar:jump-list-opened");
                                }
                            }),
                            notification: Rc::new(move |key, kind| {
                                send_notification_event(
                                    &notification_client_for_taskbar,
                                    key,
                                    kind,
                                )
                            }),
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

            let transfer_handles = desktop_handles.clone();
            if !transfer_handles.is_empty() && !state_matrix {
                let transfer_background = cx.background_executor().clone();
                let transfer_foreground = cx.foreground_executor().clone();
                let transfer_app = cx.to_async();
                let transfer_runtime = desktop_transfers.clone();
                let transfer_operations = Rc::clone(&desktop_operations);
                transfer_foreground
                    .spawn(async move {
                        loop {
                            transfer_background.timer(Duration::from_millis(50)).await;
                            let (status, terminals, refresh) = {
                                let Ok(mut snapshot) = transfer_runtime.snapshot.lock() else {
                                    continue;
                                };
                                let terminals = std::mem::take(&mut snapshot.terminals);
                                let refresh = std::mem::take(&mut snapshot.refresh_pending);
                                (snapshot.status.clone(), terminals, refresh)
                            };
                            for (correlation_id, terminal) in terminals {
                                let _ = transfer_operations
                                    .borrow_mut()
                                    .terminal(correlation_id, terminal);
                            }
                            transfer_app.update(|app| {
                                for handle in &transfer_handles {
                                    let _ = handle.update(app, |view, _, cx| {
                                        view.set_transfer_status(status.clone());
                                        if refresh {
                                            view.refresh_authoritative();
                                            trace_action("desktop:transfer-reconciled");
                                        }
                                        cx.notify();
                                    });
                                }
                            });
                        }
                    })
                    .detach();
            }

            let refresh_handles = taskbar_handles.clone();
            if !refresh_handles.is_empty() && !state_matrix {
                let refresh_background = cx.background_executor().clone();
                let refresh_foreground = cx.foreground_executor().clone();
                let refresh_app = cx.to_async();
                let refresh_notification_client = Rc::clone(&notification_client);
                let refresh_settings = Rc::clone(&persisted_settings);
                refresh_foreground
                    .spawn(async move {
                        let mut notification_tick = 0u8;
                        loop {
                            refresh_background.timer(Duration::from_millis(50)).await;
                            let taskbar_settings = refresh_settings.borrow().taskbar.clone();
                            let Ok(tasks) = visible_tasks(
                                &taskbar_settings.pins,
                                taskbar_settings.combine_groups,
                            ) else {
                                continue;
                            };
                            notification_tick = notification_tick.wrapping_add(1);
                            let notification_update = notification_tick.is_multiple_of(10).then(|| {
                                refresh_notification_client
                                    .borrow_mut()
                                    .request(
                                        &NotificationMutation::Snapshot,
                                        Duration::from_millis(100),
                                    )
                                    .ok()
                                    .and_then(|response| match response {
                                        NotificationHostResponse::Snapshot(snapshot) => {
                                            Some(snapshot)
                                        }
                                        _ => None,
                                    })
                            });
                            refresh_app.update(|app| {
                                let mut alive = false;
                                for handle in &refresh_handles {
                                    if handle
                                        .update(app, |view, _, cx| {
                                            alive = true;
                                            if view.tasks != tasks {
                                                view.tasks = tasks.clone();
                                                let live = tasks
                                                    .iter()
                                                    .map(|task| task.stable_id.as_str())
                                                    .collect::<std::collections::BTreeSet<_>>();
                                                view.overlays
                                                    .retain(|stable_id, _| live.contains(stable_id.as_str()));
                                                for task in &tasks {
                                                    let overlay = view
                                                        .overlays
                                                        .entry(task.stable_id.clone())
                                                        .or_default();
                                                    overlay.attention = task.attention;
                                                }
                                                trace_action("shell-event");
                                                cx.notify();
                                            }
                                            if let Some(snapshot) = notification_update.clone() {
                                                let changed = if let Some(snapshot) = snapshot {
                                                    view.notification_area
                                                        .apply_snapshot(snapshot, 5)
                                                } else if view
                                                    .notification_area
                                                    .provider_available()
                                                {
                                                    view.notification_area.provider_unavailable();
                                                    true
                                                } else {
                                                    false
                                                };
                                                if changed {
                                                    trace_action(
                                                        "notification:snapshot-reconciled",
                                                    );
                                                    cx.notify();
                                                }
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
