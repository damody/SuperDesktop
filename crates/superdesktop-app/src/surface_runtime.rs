use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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
    appbar_shell_hook::{
        ControlledShellCapability, OwnedShellHookEvent, ScreenRect, system_attention_cadence_ms,
    },
    desktop::{configure_and_show_desktop_window, current_wallpaper_path},
    monitor_dpi_start::{MonitorRecord, enable_per_monitor_v2, snapshot_real_monitors},
    taskbar::{configure_and_show_taskbar_window, snapshot_task_windows},
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use settings_store::{DesktopSortDirection, DesktopSortKey};
use shell_provider_protocol::{
    CURRENT_PROTOCOL, CommandDescriptor, CommandId, CommandRisk, Envelope, IconData, IconKey,
    JumpListRequest, MenuContext, MenuEnumeration, MenuInvocation, NotificationEvent,
    NotificationEventKind, NotificationHostResponse, NotificationMutation, ProviderRequest,
    ResponseBody, SearchBatch, SearchQuery, StatusAvailability, SystemStatusCommand,
    SystemStatusCommandRequest, SystemStatusHostRequest, SystemStatusHostResponse,
    SystemStatusSnapshot, TaskbarProgressKind, TaskbarWindowState, TerminalKind,
    reduce_group_progress,
};
use taskbar_ui::{
    AccessibleTask, ClockLocale, CoreStatus, FlyoutAction, JumpListModel, JumpListView,
    NotificationAreaModel, NotificationOverflowView, PreviewCard, ProgressState, ProviderState,
    StartActions, StartPowerAction, StartSnapshot, StartView, StatusRegion, SystemFlyoutKind,
    SystemFlyoutView, SystemStatusAction, TaskAction, TaskFlyoutView, TaskViewEffect,
    TaskViewModel, TaskViewSurface, TaskbarCallbacks, TaskbarContextCommand, TaskbarContextView,
    TaskbarLayout, TaskbarSettingId, TaskbarSettingsEffect, TaskbarSettingsView, TaskbarView,
    TestClock,
};

use crate::{
    notification_client::NotificationClient,
    provider_client::ProviderClient,
    status_client::{StatusReconciler, SystemStatusClient},
    taskbar_state_client::{TaskbarStateClient, TaskbarStateReconciler},
};

static NEXT_PROVIDER_REQUEST: AtomicU64 = AtomicU64::new(1);
const ICON_CACHE_LIMIT: usize = 2_048;
const HSHELL_WINDOWDESTROYED: u32 = 2;
const HSHELL_WINDOWACTIVATED: u32 = 4;
const HSHELL_RUDEAPPACTIVATED: u32 = 0x8004;
const HSHELL_FLASH: u32 = 0x8006;
const DEFAULT_FLASH_EDGES: u8 = 14;

#[derive(Clone, Debug)]
struct WindowAttention {
    process_id: u32,
    phase_on: bool,
    steady: bool,
    remaining_edges: u8,
    next_edge: Instant,
}

#[derive(Default)]
struct AttentionRuntime {
    windows: BTreeMap<isize, WindowAttention>,
}

impl AttentionRuntime {
    fn apply(&mut self, event: OwnedShellHookEvent, now: Instant) {
        match event.code {
            HSHELL_FLASH if event.hwnd_identity != 0 && event.process_id != 0 => {
                let cadence = Duration::from_millis(u64::from(system_attention_cadence_ms()));
                self.windows.insert(
                    event.hwnd_identity,
                    WindowAttention {
                        process_id: event.process_id,
                        phase_on: true,
                        steady: false,
                        remaining_edges: DEFAULT_FLASH_EDGES,
                        next_edge: now + cadence,
                    },
                );
            }
            HSHELL_WINDOWACTIVATED | HSHELL_RUDEAPPACTIVATED | HSHELL_WINDOWDESTROYED => {
                self.windows.remove(&event.hwnd_identity);
            }
            _ => {}
        }
    }

    fn tick(&mut self, now: Instant) {
        for state in self.windows.values_mut() {
            while !state.steady && now >= state.next_edge {
                state.phase_on = !state.phase_on;
                state.remaining_edges = state.remaining_edges.saturating_sub(1);
                state.next_edge += Duration::from_millis(u64::from(system_attention_cadence_ms()));
                if state.remaining_edges == 0 {
                    state.phase_on = true;
                    state.steady = true;
                }
            }
        }
    }

    fn active_windows(&self) -> BTreeSet<(isize, u32)> {
        self.windows
            .iter()
            .map(|(hwnd, state)| (*hwnd, state.process_id))
            .collect()
    }

    fn visual_for(&self, stable_id: &str) -> (bool, bool) {
        let windows = task_hwnd(stable_id)
            .into_iter()
            .chain(group_window_ids(stable_id))
            .collect::<Vec<_>>();
        let states = windows
            .iter()
            .filter_map(|hwnd| self.windows.get(hwnd))
            .collect::<Vec<_>>();
        (
            states.iter().any(|state| state.phase_on),
            states.iter().any(|state| state.steady),
        )
    }
}

fn prune_icon_cache(
    cache: &mut BTreeMap<String, Option<IconData>>,
    live: &std::collections::BTreeSet<String>,
) {
    cache.retain(|key, _| live.contains(key));
    while cache.len() > ICON_CACHE_LIMIT {
        cache.pop_first();
    }
}

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
        if requests.is_empty() {
            return false;
        }
        if self
            .active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            if let Ok(mut snapshot) = self.snapshot.lock() {
                snapshot
                    .terminals
                    .extend(requests.into_iter().map(|(request, _, _)| {
                        (request.correlation_id, DesktopOperationTerminal::Failed)
                    }));
            }
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
            let mut requests = requests.into_iter();
            while let Some((request, roots, label)) = requests.next() {
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
                    let remaining_terminal = if terminal == DesktopOperationTerminal::Cancelled {
                        DesktopOperationTerminal::Cancelled
                    } else {
                        DesktopOperationTerminal::Failed
                    };
                    if let Ok(mut current) = snapshot.lock() {
                        current.terminals.extend(
                            requests.map(|(request, _, _)| {
                                (request.correlation_id, remaining_terminal)
                            }),
                        );
                    }
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

fn trace_rendered_frame() {
    trace_action("frame-visible");
    let (icons, _) = gpui::compressed_gpu_cache_stats();
    if icons.uploads > 0 {
        trace_action("icon:bc7-gpu-uploaded");
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
    icon_cache: BTreeMap<String, Option<IconData>>,
}

const DESKTOP_GRID_ORIGIN: f32 = 8.0;
const DESKTOP_GRID_CELL_WIDTH: f32 = 104.0;
const DESKTOP_GRID_CELL_HEIGHT: f32 = 112.0;
const DESKTOP_GRID_ROWS: usize = 8;

fn desktop_items_overlap(left: (f32, f32), right: (f32, f32)) -> bool {
    left.0 < right.0 + DESKTOP_GRID_CELL_WIDTH
        && left.0 + DESKTOP_GRID_CELL_WIDTH > right.0
        && left.1 < right.1 + DESKTOP_GRID_CELL_HEIGHT
        && left.1 + DESKTOP_GRID_CELL_HEIGHT > right.1
}

fn reconcile_desktop_item_positions(
    nodes: &[AccessibleNode],
    persisted: &BTreeMap<String, (f32, f32)>,
) -> BTreeMap<String, (f32, f32)> {
    let mut occupied = Vec::with_capacity(nodes.len());
    let mut reconciled = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        let default = (
            DESKTOP_GRID_ORIGIN + (index / DESKTOP_GRID_ROWS) as f32 * DESKTOP_GRID_CELL_WIDTH,
            DESKTOP_GRID_ORIGIN + (index % DESKTOP_GRID_ROWS) as f32 * DESKTOP_GRID_CELL_HEIGHT,
        );
        let candidate = persisted.get(&node.stable_id).copied().unwrap_or(default);
        let position = if occupied
            .iter()
            .copied()
            .any(|other| desktop_items_overlap(candidate, other))
        {
            (0..)
                .map(|slot| {
                    (
                        DESKTOP_GRID_ORIGIN
                            + (slot / DESKTOP_GRID_ROWS) as f32 * DESKTOP_GRID_CELL_WIDTH,
                        DESKTOP_GRID_ORIGIN
                            + (slot % DESKTOP_GRID_ROWS) as f32 * DESKTOP_GRID_CELL_HEIGHT,
                    )
                })
                .find(|candidate| {
                    !occupied
                        .iter()
                        .copied()
                        .any(|other| desktop_items_overlap(*candidate, other))
                })
                .expect("desktop grid has an unbounded number of columns")
        } else {
            candidate
        };
        occupied.push(position);
        reconciled.insert(node.stable_id.clone(), position);
    }
    reconciled
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
    let mut icon_cache = std::mem::take(&mut runtime.borrow_mut().icon_cache);
    let fixed_path = superexplorer_executable();
    let fixed_key = fixed_path.as_ref().map_or_else(
        || "fixed:superexplorer".to_owned(),
        |path| format!("fixed:{}", path.to_string_lossy().to_lowercase()),
    );
    let fixed_icon = icon_cache
        .entry(fixed_key.clone())
        .or_insert_with(|| {
            fixed_path
                .as_deref()
                .and_then(|path| platform_win::common::icon::shell_icon_for_path(path, 48))
        })
        .clone();
    trace_action(if fixed_icon.is_some() {
        "superexplorer:icon-resolved"
    } else {
        "superexplorer:icon-unavailable"
    });
    let mut nodes = vec![fixed_node(monitor, fixed_icon)];
    let Ok((user_root, public_root)) = platform_win::common::desktop::known_desktop_roots() else {
        prune_icon_cache(&mut icon_cache, &std::iter::once(fixed_key).collect());
        runtime.borrow_mut().icon_cache = icon_cache;
        trace_action("desktop:root-resolution-failed");
        return nodes;
    };
    let Ok(entries) = platform_win::common::desktop::enumerate_known_desktops() else {
        prune_icon_cache(&mut icon_cache, &std::iter::once(fixed_key).collect());
        runtime.borrow_mut().icon_cache = icon_cache;
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
        let icon_key = item.icon.source_key.to_lowercase();
        let icon = icon_cache
            .entry(icon_key.clone())
            .or_insert_with(|| {
                platform_win::common::icon::shell_icon_for_path(
                    Path::new(&item.activation_token),
                    48,
                )
            })
            .clone();
        nodes.push(AccessibleNode {
            stable_id: key.clone(),
            name: item.display_name.clone(),
            icon,
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
    let live_icon_keys = items
        .values()
        .map(|item| item.icon.source_key.to_lowercase())
        .chain(std::iter::once(fixed_key))
        .collect::<std::collections::BTreeSet<_>>();
    prune_icon_cache(&mut icon_cache, &live_icon_keys);
    *runtime.borrow_mut() = DesktopNamespaceRuntime {
        items,
        allowed_roots,
        user_root: Some(user_root),
        icon_cache,
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

fn progress_for_task(
    stable_id: &str,
    states: &BTreeMap<(isize, u32), TaskbarWindowState>,
) -> ProgressState {
    let window_ids = task_hwnd(stable_id)
        .into_iter()
        .chain(group_window_ids(stable_id))
        .collect::<BTreeSet<_>>();
    let progress = reduce_group_progress(
        states
            .iter()
            .filter(|((hwnd, _), _)| window_ids.contains(hwnd))
            .map(|(_, state)| state.progress),
    );
    let value = progress.permille().unwrap_or(0);
    match progress.kind {
        TaskbarProgressKind::None => ProgressState::None,
        TaskbarProgressKind::Indeterminate => ProgressState::Indeterminate,
        TaskbarProgressKind::Normal => ProgressState::Normal(value),
        TaskbarProgressKind::Paused => ProgressState::Paused(value),
        TaskbarProgressKind::Error => ProgressState::Error(value),
    }
}

fn visible_tasks(
    pin_order: &[String],
    combine_groups: bool,
    icon_cache: &mut BTreeMap<String, Option<IconData>>,
    attention_windows: &BTreeSet<(isize, u32)>,
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
            let tasks = groups
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
                    let icon = icon_cache
                        .entry(stable_id.clone())
                        .or_insert_with(|| {
                            platform_win::common::icon::window_icon(
                                windows[0].hwnd_identity,
                                Path::new(&application)
                                    .is_file()
                                    .then(|| Path::new(&application)),
                                32,
                            )
                        })
                        .clone();
                    AccessibleTask {
                        stable_id,
                        name,
                        icon,
                        role: "button",
                        active: windows.iter().any(|window| window.foreground),
                        minimized: windows.iter().all(|window| window.minimized),
                        attention: windows.iter().any(|window| {
                            attention_windows.contains(&(window.hwnd_identity, window.process_id))
                                && !window.foreground
                        }),
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
                .collect::<Vec<_>>();
            let live = tasks
                .iter()
                .map(|task| task.stable_id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            let live = live.into_iter().map(str::to_owned).collect();
            prune_icon_cache(icon_cache, &live);
            tasks
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
            icon: None,
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

fn verification_task_overlay(task: &AccessibleTask) -> taskbar_ui::TaskOverlay {
    let progress = if task.stable_id.ends_with(":active") {
        ProgressState::Normal(420)
    } else if task.stable_id.ends_with(":minimized") {
        ProgressState::Paused(650)
    } else if task.stable_id.ends_with(":attention") {
        ProgressState::Error(300)
    } else if task.stable_id.ends_with(":group") {
        ProgressState::Indeterminate
    } else {
        ProgressState::None
    };
    taskbar_ui::TaskOverlay {
        progress,
        attention: task.attention,
        attention_phase_on: task.attention,
        animation_phase: 350,
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct StartWindowGeometry {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn start_window_geometry(monitor: &MonitorRecord) -> StartWindowGeometry {
    let scale = monitor.dpi_x as f32 / 96.0;
    let monitor_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let monitor_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    let width = (monitor_width - 24.0).clamp(1.0, 640.0);
    let height = (monitor_height - 12.0).clamp(1.0, 720.0);
    let work_left = monitor.work_area.left as f32 / scale;
    let work_top = monitor.work_area.top as f32 / scale;
    StartWindowGeometry {
        left: work_left + (monitor_width - width).max(0.0) / 2.0,
        top: (monitor.work_area.bottom as f32 / scale - height - 12.0).max(work_top),
        width,
        height,
    }
}

fn start_options(monitor: &MonitorRecord) -> WindowOptions {
    let geometry = start_window_geometry(monitor);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(geometry.left), px(geometry.top)),
            size: size(px(geometry.width), px(geometry.height)),
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

fn system_flyout_options(
    monitor: &MonitorRecord,
    kind: SystemFlyoutKind,
    input_profile_count: usize,
) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = 380.0;
    let available_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    let height = match kind {
        SystemFlyoutKind::Input => 92.0 + input_profile_count.clamp(1, 6) as f32 * 52.0,
        SystemFlyoutKind::Volume => 190.0,
        SystemFlyoutKind::NetworkPower => 250.0,
        SystemFlyoutKind::Calendar => 420.0,
    }
    .min(available_height);
    let right = monitor.work_area.right as f32 / scale;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px((right - width).max(monitor.work_area.left as f32 / scale)),
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

fn notification_overflow_options(monitor: &MonitorRecord, icon_count: usize) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = 336.0;
    let rows = icon_count.max(1).div_ceil(6).min(6) as f32;
    let height = 24.0 + rows * 48.0;
    let right = monitor.work_area.right as f32 / scale;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px((right - width).max(monitor.work_area.left as f32 / scale)),
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

fn taskbar_context_options(
    monitor: &MonitorRecord,
    anchor: gpui::Point<gpui::Pixels>,
) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let (left, top, _, _) = taskbar_context_placement(monitor, anchor);
    let width = 220.0;
    let height = 80.0;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(left as f32), px(top as f32)),
            size: size(px(width * scale), px(height * scale)),
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

fn taskbar_context_placement(
    monitor: &MonitorRecord,
    anchor: gpui::Point<gpui::Pixels>,
) -> (i32, i32, i32, i32) {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = 220.0;
    let height = 80.0;
    let work_left = monitor.work_area.left as f32 / scale;
    let work_right = monitor.work_area.right as f32 / scale;
    let work_top = monitor.work_area.top as f32 / scale;
    let work_bottom = monitor.work_area.bottom as f32 / scale;
    let left = (work_left + anchor.x.as_f32() - width / 2.0)
        .clamp(work_left, (work_right - width).max(work_left));
    let top = (work_bottom - height - 8.0).max(work_top);
    (
        (left * scale).round() as i32,
        (top * scale).round() as i32,
        (width * scale).round() as i32,
        (height * scale).round() as i32,
    )
}

fn taskbar_settings_options(monitor: &MonitorRecord) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let available_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let available_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    let width = available_width.clamp(640.0, 980.0);
    let height = available_height.clamp(480.0, 860.0);
    let (left, top, _, _) = taskbar_settings_placement(monitor);
    let physical_width =
        (width * scale).min((monitor.work_area.right - monitor.work_area.left) as f32);
    let physical_height =
        (height * scale).min((monitor.work_area.bottom - monitor.work_area.top) as f32);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(left as f32), px(top as f32)),
            size: size(px(physical_width), px(physical_height)),
        })),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: true,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
}

fn taskbar_settings_placement(monitor: &MonitorRecord) -> (i32, i32, i32, i32) {
    let scale = monitor.dpi_x as f32 / 96.0;
    let available_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let available_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    let width = available_width.clamp(640.0, 980.0);
    let height = available_height.clamp(480.0, 860.0);
    let left = monitor.work_area.left as f32 / scale + (available_width - width) / 2.0;
    let top = monitor.work_area.top as f32 / scale + (available_height - height) / 2.0;
    (
        (left * scale).round() as i32,
        (top * scale).round() as i32,
        (width * scale).round() as i32,
        (height * scale).round() as i32,
    )
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

fn unavailable_status<T>() -> ProviderState<T> {
    ProviderState::Unavailable("system-status-host-not-ready")
}

fn status(snapshot: Option<&SystemStatusSnapshot>) -> StatusRegion {
    let local = platform_win::common::taskbar_status::local_date_time();
    let network = snapshot.map_or_else(unavailable_status, |snapshot| match &snapshot.network {
        StatusAvailability::Available(network) => ProviderState::Available(if network.internet {
            format!("{} (Internet)", network.display_name)
        } else {
            network.display_name.clone()
        }),
        StatusAvailability::NotPresent => ProviderState::Unavailable("network-not-present"),
        StatusAvailability::Unavailable { .. } => unavailable_status(),
    });
    let (volume, muted) = snapshot.map_or_else(
        || (unavailable_status(), unavailable_status()),
        |snapshot| match &snapshot.audio {
            StatusAvailability::Available(audio) => (
                ProviderState::Available(audio.volume_percent),
                ProviderState::Available(audio.muted),
            ),
            StatusAvailability::NotPresent => (
                ProviderState::Unavailable("audio-not-present"),
                ProviderState::Unavailable("audio-not-present"),
            ),
            StatusAvailability::Unavailable { .. } => (unavailable_status(), unavailable_status()),
        },
    );
    let input_language =
        snapshot.map_or_else(unavailable_status, |snapshot| match &snapshot.input {
            StatusAvailability::Available(input) => input
                .profiles
                .iter()
                .find(|profile| profile.id == input.active_profile_id)
                .map_or_else(unavailable_status, |profile| {
                    ProviderState::Available(profile.language_tag.clone())
                }),
            StatusAvailability::NotPresent => ProviderState::Unavailable("input-not-present"),
            StatusAvailability::Unavailable { .. } => unavailable_status(),
        });
    let battery = snapshot.map_or_else(unavailable_status, |snapshot| match &snapshot.power {
        StatusAvailability::Available(power) => power.battery_percent.map_or(
            ProviderState::Unavailable("battery-not-present"),
            ProviderState::Available,
        ),
        StatusAvailability::NotPresent => ProviderState::Unavailable("battery-not-present"),
        StatusAvailability::Unavailable { .. } => unavailable_status(),
    });
    StatusRegion::new(
        TestClock {
            year: local.year,
            month: local.month,
            day: local.day,
            hour: local.hour,
            minute: local.minute,
        },
        ClockLocale::ZhTw,
        CoreStatus {
            network,
            volume,
            muted,
            input_language,
            battery,
            notifications: ProviderState::Unavailable("notification-provider-not-ready"),
        },
    )
}

fn apply_system_status_action(
    action: SystemStatusAction,
    app: &mut App,
    client: &Rc<RefCell<SystemStatusClient>>,
    reconciler: &Rc<RefCell<StatusReconciler>>,
    start_window: &Rc<RefCell<Option<gpui::WindowHandle<StartView>>>>,
) {
    let restore_start_focus = matches!(&action, SystemStatusAction::ActivateInputProfile(_));
    let Some(expected_host_generation) = reconciler
        .borrow()
        .snapshot()
        .map(|snapshot| snapshot.host_generation)
    else {
        trace_action("status:command-provider-unavailable");
        return;
    };
    let command = match action {
        SystemStatusAction::ActivateInputProfile(profile_id) => {
            SystemStatusCommand::ActivateInputProfile { profile_id }
        }
        SystemStatusAction::SetVolume(volume_percent) => {
            SystemStatusCommand::SetVolume { volume_percent }
        }
        SystemStatusAction::SetMute(muted) => SystemStatusCommand::SetMute { muted },
    };
    let correlation_id = format!(
        "system-status-{}",
        NEXT_PROVIDER_REQUEST.fetch_add(1, Ordering::Relaxed)
    );
    let request = SystemStatusHostRequest::Command {
        request: SystemStatusCommandRequest {
            correlation_id,
            expected_host_generation,
            deadline_unix_ms: unix_time_ms().saturating_add(1_000),
            command,
        },
    };
    let response = client
        .borrow_mut()
        .request(&request, Duration::from_millis(1_000));
    match response {
        Ok(response @ SystemStatusHostResponse::Terminal(_)) => {
            reconciler.borrow_mut().apply(response);
            if let Ok(snapshot @ SystemStatusHostResponse::Snapshot(_)) =
                client.borrow_mut().request(
                    &SystemStatusHostRequest::Snapshot,
                    Duration::from_millis(500),
                )
            {
                reconciler.borrow_mut().apply(snapshot);
            }
            trace_action("status:command-terminal");
        }
        Ok(_) => trace_action("status:command-invalid-response"),
        Err(_) => {
            reconciler.borrow_mut().provider_unavailable();
            trace_action("status:command-provider-failed");
        }
    }
    if restore_start_focus && let Some(start) = *start_window.borrow() {
        let _ = start.update(app, |_, window, cx| {
            window.activate_window();
            cx.notify();
        });
        trace_action("start:ime-focus-restored");
    }
}

fn fixed_label() -> &'static str {
    match std::env::var("SUPERDESKTOP_LOCALE").as_deref() {
        Ok("zh-CN") => "超级资源管理器",
        Ok("zh-TW") => "超級檔案總管",
        _ => "SuperExplorer",
    }
}

fn fixed_node(monitor: &str, icon: Option<IconData>) -> AccessibleNode {
    let selected = std::env::var_os("SUPERDESKTOP_VERIFICATION_DESKTOP_SELECTED").is_some();
    let mut node = AccessibleNode::fixed_superexplorer(monitor, selected, selected);
    node.name = fixed_label().into();
    node.icon = icon;
    node
}

fn superexplorer_executable() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("SUPEREXPLORER_PATH") {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(parent) = current.parent() {
            candidates.push(parent.join("SuperExplorer.exe"));
        }
        if let Some(parent_workspace) = current
            .ancestors()
            .find(|ancestor| {
                ancestor
                    .file_name()
                    .is_some_and(|name| name == "SuperDesktop")
            })
            .and_then(Path::parent)
        {
            candidates.push(parent_workspace.join("target/release/SuperExplorer.exe"));
            candidates.push(parent_workspace.join("target/debug/SuperExplorer.exe"));
        }
    }
    candidates.into_iter().find(|path| path.is_file())
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
    let task_icon_cache = Rc::new(RefCell::new(BTreeMap::new()));
    let initial_tasks = if state_matrix {
        verification_state_tasks()
    } else {
        visible_tasks(
            &persisted_settings.taskbar.pins,
            persisted_settings.taskbar.combine_groups,
            &mut task_icon_cache.borrow_mut(),
            &BTreeSet::new(),
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
    let notification_client = Rc::new(RefCell::new(NotificationClient::adjacent(shell)?));
    let status_client = Rc::new(RefCell::new(SystemStatusClient::adjacent()?));
    let mut initial_taskbar_state_client = TaskbarStateClient::adjacent(shell)?;
    initial_taskbar_state_client.ensure_started()?;
    let taskbar_state_client = Rc::new(RefCell::new(initial_taskbar_state_client));
    let taskbar_state_reconciler = Rc::new(RefCell::new(TaskbarStateReconciler::default()));
    let status_reconciler = Rc::new(RefCell::new(StatusReconciler::default()));
    if let Ok(response) = status_client.borrow_mut().request(
        &SystemStatusHostRequest::Snapshot,
        Duration::from_millis(750),
    ) {
        status_reconciler.borrow_mut().apply(response);
    }
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
            let mut system_flyout_windows = Vec::new();
            let leases = Rc::new(RefCell::new(Vec::<ControlledShellCapability>::new()));
            let attention_runtime = Rc::new(RefCell::new(AttentionRuntime::default()));
            let taskbar_context_window =
                Rc::new(RefCell::new(None::<gpui::WindowHandle<TaskbarContextView>>));
            let taskbar_settings_window =
                Rc::new(RefCell::new(None::<gpui::WindowHandle<TaskbarSettingsView>>));
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
                            let persisted_item_positions = desktop_settings_for_view
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
                            let item_positions = reconcile_desktop_item_positions(
                                &nodes,
                                &persisted_item_positions,
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
                            .with_rendered_action(Rc::new(trace_rendered_frame));
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
                let start_window_for_status = Rc::clone(&start_window);
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
                let notification_client_for_overflow = Rc::clone(&notification_client);
                let notification_overflow_window = Rc::new(RefCell::new(None::<
                    gpui::WindowHandle<NotificationOverflowView>,
                >));
                let notification_overflow_window_for_taskbar =
                    Rc::clone(&notification_overflow_window);
                let notification_overflow_monitor = taskbar_monitor.clone();
                let status_for_taskbar = Rc::clone(&status_reconciler);
                let status_client_for_taskbar = Rc::clone(&status_client);
                let status_commands_for_taskbar = Rc::clone(&status_reconciler);
                let system_flyout_window = Rc::new(RefCell::new(None::<(
                    SystemFlyoutKind,
                    gpui::WindowHandle<SystemFlyoutView>,
                )>));
                system_flyout_windows.push(Rc::clone(&system_flyout_window));
                let system_flyout_window_for_taskbar = Rc::clone(&system_flyout_window);
                let system_flyout_monitor = taskbar_monitor.clone();
                let system_flyout_status = Rc::clone(&status_reconciler);
                let system_flyout_client = Rc::clone(&status_client);
                let system_flyout_start = Rc::clone(&start_window);
                let context_window_for_taskbar = Rc::clone(&taskbar_context_window);
                let settings_window_for_context = Rc::clone(&taskbar_settings_window);
                let context_monitor = taskbar_monitor.clone();
                let context_settings_store = Rc::clone(&settings_store);
                let context_settings_target = Rc::clone(&settings_target);
                let context_persisted_settings = Rc::clone(&persisted_settings);
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
                            trace_action("taskbar:appbar-owned");
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
                            .filter_map(|task| {
                                let overlay = if state_matrix {
                                    verification_task_overlay(task)
                                } else {
                                    taskbar_ui::TaskOverlay {
                                        attention: task.attention,
                                        ..Default::default()
                                    }
                                };
                                (overlay != taskbar_ui::TaskOverlay::default())
                                    .then(|| (task.stable_id.clone(), overlay))
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
                        fixed_icon: superexplorer_executable().and_then(|path| {
                            platform_win::common::icon::shell_icon_for_path(&path, 32)
                        }),
                        status: status(status_for_taskbar.borrow().snapshot()),
                        system_snapshot: status_for_taskbar.borrow().snapshot().cloned(),
                        system_flyout: None,
                        notification_area: NotificationAreaModel::default(),
                        overlays: taskbar_overlays,
                        show_labels: production_taskbar_settings.show_labels,
                        search_mode: production_taskbar_settings.search_mode,
                        show_task_view: production_taskbar_settings.show_task_view,
                        alignment: production_taskbar_settings.alignment,
                        callbacks: Some(TaskbarCallbacks {
                            start: Rc::new(move |app| {
                                trace_action("start");
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
                            taskbar_context: Rc::new(move |anchor, app| {
                                if let Some(existing) = *context_window_for_taskbar.borrow() {
                                    let _ = existing.update(app, |_, window, _| window.remove_window());
                                    *context_window_for_taskbar.borrow_mut() = None;
                                }
                                let context_slot = Rc::clone(&context_window_for_taskbar);
                                let settings_slot = Rc::clone(&settings_window_for_context);
                                let settings_monitor = context_monitor.clone();
                                let settings_store = Rc::clone(&context_settings_store);
                                let settings_target = Rc::clone(&context_settings_target);
                                let live_settings = Rc::clone(&context_persisted_settings);
                                let opened = app.open_window(
                                    taskbar_context_options(&context_monitor, anchor),
                                    move |window, cx| {
                                        window.activate_window();
                                        let dismiss_slot = Rc::clone(&context_slot);
                                        let settings_slot_for_action = Rc::clone(&settings_slot);
                                        let monitor_for_action = settings_monitor.clone();
                                        let store_for_action = Rc::clone(&settings_store);
                                        let target_for_action = Rc::clone(&settings_target);
                                        let live_for_action = Rc::clone(&live_settings);
                                        cx.new(move |cx| {
                                            TaskbarContextView::new(
                                                Rc::new(move |command, app| match command {
                                                    TaskbarContextCommand::OpenTaskManager => {
                                                        trace_action(if platform_win::common::taskbar::launch_task_manager().is_ok() {
                                                            "taskbar:task-manager-launched"
                                                        } else {
                                                            "taskbar:task-manager-rejected"
                                                        });
                                                    }
                                                    TaskbarContextCommand::OpenTaskbarSettings => {
                                                        if let Some(existing) = *settings_slot_for_action.borrow() {
                                                            if existing.update(app, |_, window, _| window.activate_window()).is_ok() {
                                                                trace_action("taskbar:settings-activated");
                                                                return;
                                                            }
                                                            *settings_slot_for_action.borrow_mut() = None;
                                                        }
                                                        let initial = live_for_action.borrow().clone();
                                                        let save_store = Rc::clone(&store_for_action);
                                                        let save_target = Rc::clone(&target_for_action);
                                                        let save_live = Rc::clone(&live_for_action);
                                                        let settings_dismiss_slot = Rc::clone(&settings_slot_for_action);
                                                        let settings_dismiss_slot_for_view = Rc::clone(&settings_dismiss_slot);
                                                        let settings_opened = app.open_window(
                                                            taskbar_settings_options(&monitor_for_action),
                                                            move |window, cx| {
                                                                window.activate_window();
                                                                cx.new(move |cx| TaskbarSettingsView::new(
                                                                    initial.taskbar.clone(),
                                                                    initial.revision,
                                                                    Rc::new(move |effect| {
                                                                        match effect {
                                                                            TaskbarSettingsEffect::Save { candidate, base_revision } => {
                                                                                let current = save_live.borrow().clone();
                                                                                if current.revision != base_revision || !(1..=3).contains(&candidate.rows) {
                                                                                    return Err("Settings changed in another window; reopen Taskbar settings".into());
                                                                                }
                                                                                let mut updated = current;
                                                                                updated.taskbar = candidate;
                                                                                match save_store.borrow_mut().save(&save_target, &updated) {
                                                                                    Ok(saved) => {
                                                                                        let result = (saved.taskbar.clone(), saved.revision);
                                                                                        *save_live.borrow_mut() = saved;
                                                                                        trace_action("taskbar:settings-saved");
                                                                                        Ok(result)
                                                                                    }
                                                                                    Err(_) => Err("Taskbar settings could not be saved".into()),
                                                                                }
                                                                            }
                                                                            TaskbarSettingsEffect::OpenOtherTrayIcons => {
                                                                                let current = save_live.borrow();
                                                                                trace_action("taskbar:settings-overflow-requested");
                                                                                Ok((current.taskbar.clone(), current.revision))
                                                                            }
                                                                            TaskbarSettingsEffect::OpenRelated(id) => {
                                                                                let current = save_live.borrow();
                                                                                trace_action(match id {
                                                                                    TaskbarSettingId::DateTime => "taskbar:settings-date-time-requested",
                                                                                    _ => "taskbar:settings-notifications-requested",
                                                                                });
                                                                                Ok((current.taskbar.clone(), current.revision))
                                                                            }
                                                                        }
                                                                    }),
                                                                    Rc::new(move |window, _| {
                                                                        window.remove_window();
                                                                        *settings_dismiss_slot_for_view.borrow_mut() = None;
                                                                    }),
                                                                    cx,
                                                                ))
                                                            },
                                                        );
                                                        if let Ok(handle) = settings_opened {
                                                            *settings_dismiss_slot.borrow_mut() = Some(handle);
                                                            trace_action("taskbar:settings-opened");
                                                        }
                                                    }
                                                }),
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
                                    *context_window_for_taskbar.borrow_mut() = Some(handle);
                                    trace_action("taskbar:context-opened");
                                }
                            }),
                            notification: Rc::new(move |key, kind| {
                                send_notification_event(
                                    &notification_client_for_taskbar,
                                    key,
                                    kind,
                                )
                            }),
                            notification_overflow: Rc::new(move |nodes, app| {
                                if let Some(open) = notification_overflow_window_for_taskbar
                                    .borrow_mut()
                                    .take()
                                {
                                    let _ = open.update(app, |_, window, _| window.remove_window());
                                    trace_action("notification:overflow-closed");
                                    return;
                                }
                                if nodes.is_empty() {
                                    return;
                                }
                                let dismiss_slot =
                                    Rc::clone(&notification_overflow_window_for_taskbar);
                                let event_client = Rc::clone(&notification_client_for_overflow);
                                let opened = app.open_window(
                                    notification_overflow_options(
                                        &notification_overflow_monitor,
                                        nodes.len(),
                                    ),
                                    move |window, cx| {
                                        window.activate_window();
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            NotificationOverflowView::new(
                                                nodes,
                                                Rc::new(move |key, kind| {
                                                    send_notification_event(
                                                        &event_client,
                                                        key,
                                                        kind,
                                                    );
                                                }),
                                                Rc::new(move |window, _| {
                                                    window.remove_window();
                                                    *dismiss_slot.borrow_mut() = None;
                                                    trace_action(
                                                        "notification:overflow-dismissed",
                                                    );
                                                }),
                                                window,
                                                cx,
                                            )
                                        })
                                    },
                                );
                                if let Ok(handle) = opened {
                                    *notification_overflow_window_for_taskbar.borrow_mut() =
                                        Some(handle);
                                    trace_action("notification:owned-overflow-opened");
                                }
                            }),
                            system_status: Rc::new(move |action, app| {
                                apply_system_status_action(
                                    action,
                                    app,
                                    &status_client_for_taskbar,
                                    &status_commands_for_taskbar,
                                    &start_window_for_status,
                                );
                            }),
                            system_flyout: Rc::new(move |kind, app| {
                                if let Some((open_kind, handle)) =
                                    system_flyout_window_for_taskbar.borrow_mut().take()
                                {
                                    let _ = handle.update(app, |_, window, _| {
                                        window.remove_window();
                                    });
                                    if open_kind == kind {
                                        trace_action("status:flyout-closed");
                                        return;
                                    }
                                }
                                let snapshot = system_flyout_status.borrow().snapshot().cloned();
                                let flyout_status = status(snapshot.as_ref());
                                let input_profile_count = snapshot
                                    .as_ref()
                                    .and_then(|snapshot| match &snapshot.input {
                                        StatusAvailability::Available(input) => {
                                            Some(input.profiles.len())
                                        }
                                        _ => None,
                                    })
                                    .unwrap_or(1);
                                let action_client = Rc::clone(&system_flyout_client);
                                let action_status = Rc::clone(&system_flyout_status);
                                let action_start = Rc::clone(&system_flyout_start);
                                let dismiss_slot =
                                    Rc::clone(&system_flyout_window_for_taskbar);
                                let opened = app.open_window(
                                    system_flyout_options(
                                        &system_flyout_monitor,
                                        kind,
                                        input_profile_count,
                                    ),
                                    move |window, cx| {
                                        window.activate_window();
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            SystemFlyoutView::new(
                                                kind,
                                                snapshot,
                                                flyout_status,
                                                Rc::new(move |action, app| {
                                                    apply_system_status_action(
                                                        action,
                                                        app,
                                                        &action_client,
                                                        &action_status,
                                                        &action_start,
                                                    );
                                                }),
                                                Rc::new(move |window, _| {
                                                    window.remove_window();
                                                    *dismiss_slot.borrow_mut() = None;
                                                    trace_action("status:flyout-dismissed");
                                                }),
                                                window,
                                                cx,
                                            )
                                        })
                                    },
                                );
                                if let Ok(handle) = opened {
                                    *system_flyout_window_for_taskbar.borrow_mut() =
                                        Some((kind, handle));
                                    trace_action("status:owned-flyout-opened");
                                }
                            }),
                            rendered: Rc::new(trace_rendered_frame),
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
                let refresh_status_client = Rc::clone(&status_client);
                let refresh_status_reconciler = Rc::clone(&status_reconciler);
                let refresh_taskbar_state_client = Rc::clone(&taskbar_state_client);
                let refresh_taskbar_state_reconciler = Rc::clone(&taskbar_state_reconciler);
                let refresh_system_flyouts = system_flyout_windows.clone();
                let refresh_settings = Rc::clone(&persisted_settings);
                let refresh_task_icons = Rc::clone(&task_icon_cache);
                let refresh_attention = Rc::clone(&attention_runtime);
                refresh_foreground
                    .spawn(async move {
                        let mut notification_tick = 0u8;
                        loop {
                            refresh_background.timer(Duration::from_millis(50)).await;
                            let now = Instant::now();
                            {
                                let mut attention = refresh_attention.borrow_mut();
                                for event in ControlledShellCapability::drain_shell_hook_events() {
                                    attention.apply(event, now);
                                }
                                attention.tick(now);
                            }
                            let attention_windows = refresh_attention.borrow().active_windows();
                            let taskbar_settings = refresh_settings.borrow().taskbar.clone();
                            let Ok(tasks) = visible_tasks(
                                &taskbar_settings.pins,
                                taskbar_settings.combine_groups,
                                &mut refresh_task_icons.borrow_mut(),
                                &attention_windows,
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
                            if notification_tick.is_multiple_of(10) {
                                match refresh_status_client.borrow_mut().request(
                                    &SystemStatusHostRequest::Snapshot,
                                    Duration::from_millis(250),
                                ) {
                                    Ok(response @ SystemStatusHostResponse::Snapshot(_)) => {
                                        refresh_status_reconciler.borrow_mut().apply(response);
                                    }
                                    Ok(_) => {}
                                    Err(_) => {
                                        let mut reconciler = refresh_status_reconciler.borrow_mut();
                                        reconciler.provider_unavailable();
                                        if !reconciler.restart_allowed() {
                                            trace_action("status:restart-capacity-exhausted");
                                        }
                                    }
                                }
                            }
                            if notification_tick.is_multiple_of(2) {
                                match refresh_taskbar_state_client
                                    .borrow_mut()
                                    .request_snapshot(Duration::from_millis(100))
                                {
                                    Ok(snapshot) => {
                                        refresh_taskbar_state_reconciler.borrow_mut().apply(snapshot);
                                    }
                                    Err("taskbar-state-disabled") => {}
                                    Err(_) => {
                                        refresh_taskbar_state_reconciler
                                            .borrow_mut()
                                            .provider_unavailable();
                                    }
                                }
                            }
                            let taskbar_states = refresh_taskbar_state_reconciler
                                .borrow()
                                .windows()
                                .clone();
                            let current_status = status(refresh_status_reconciler.borrow().snapshot());
                            let current_system_snapshot =
                                refresh_status_reconciler.borrow().snapshot().cloned();
                            refresh_app.update(|app| {
                                let mut alive = false;
                                for handle in &refresh_handles {
                                    if handle
                                        .update(app, |view, window, cx| {
                                            alive = true;
                                            let mut settings_changed = false;
                                            if view.show_labels != taskbar_settings.show_labels {
                                                view.show_labels = taskbar_settings.show_labels;
                                                settings_changed = true;
                                            }
                                            if view.search_mode != taskbar_settings.search_mode {
                                                view.search_mode = taskbar_settings.search_mode;
                                                settings_changed = true;
                                            }
                                            if view.show_task_view != taskbar_settings.show_task_view {
                                                view.show_task_view = taskbar_settings.show_task_view;
                                                settings_changed = true;
                                            }
                                            if view.alignment != taskbar_settings.alignment {
                                                view.alignment = taskbar_settings.alignment;
                                                settings_changed = true;
                                            }
                                            if view.layout.rows.get() != taskbar_settings.rows {
                                                let scale = window.scale_factor();
                                                let bounds = window.bounds();
                                                let physical_width =
                                                    (bounds.size.width.as_f32() * scale).round() as i32;
                                                let physical_bottom = ((bounds.origin.y
                                                    + bounds.size.height)
                                                    .as_f32()
                                                    * scale)
                                                    .round()
                                                    as i32;
                                                let physical_left =
                                                    (bounds.origin.x.as_f32() * scale).round() as i32;
                                                let physical_height = (40.0
                                                    * f32::from(taskbar_settings.rows)
                                                    * scale)
                                                    .round()
                                                    as i32;
                                                if let Ok(raw) = hwnd(window) {
                                                    let _ = configure_and_show_taskbar_window(
                                                        raw,
                                                        physical_left,
                                                        physical_bottom - physical_height,
                                                        physical_width,
                                                        physical_height,
                                                    );
                                                }
                                                settings_changed = true;
                                            }
                                            if settings_changed || view.tasks != tasks {
                                                let running = tasks
                                                    .iter()
                                                    .map(|task| task.stable_id.clone())
                                                    .collect::<Vec<_>>();
                                                view.layout = TaskbarLayout::calculate(
                                                    taskbar_settings.rows,
                                                    (window.scale_factor() * 96.0).round() as u32,
                                                    window.bounds().size.width.as_f32()
                                                        * window.scale_factor(),
                                                    &running,
                                                    &taskbar_settings.pins,
                                                );
                                            }
                                            if view.status != current_status {
                                                view.status = current_status.clone();
                                                trace_action("clock:updated");
                                                cx.notify();
                                            }
                                            if view.system_snapshot != current_system_snapshot {
                                                view.system_snapshot = current_system_snapshot.clone();
                                                trace_action("status:snapshot-updated");
                                                cx.notify();
                                            }
                                            let mut task_visual_changed = false;
                                            for task in &tasks {
                                                let overlay = view
                                                    .overlays
                                                    .entry(task.stable_id.clone())
                                                    .or_default();
                                                let (phase_on, steady) = refresh_attention
                                                    .borrow()
                                                    .visual_for(&task.stable_id);
                                                let before = (
                                                    overlay.attention,
                                                    overlay.attention_phase_on,
                                                    overlay.attention_steady,
                                                    overlay.progress,
                                                    overlay.animation_phase,
                                                );
                                                overlay.attention = task.attention;
                                                overlay.attention_phase_on = phase_on;
                                                overlay.attention_steady = steady;
                                                overlay.progress =
                                                    progress_for_task(&task.stable_id, &taskbar_states);
                                                if overlay.attention || overlay.progress == ProgressState::Indeterminate {
                                                    overlay.animation_phase =
                                                        (overlay.animation_phase + 80) % 1_001;
                                                } else {
                                                    overlay.animation_phase = 0;
                                                }
                                                task_visual_changed |= before
                                                    != (
                                                        task.attention,
                                                        phase_on,
                                                        steady,
                                                        overlay.progress,
                                                        overlay.animation_phase,
                                                    );
                                            }
                                            if view.tasks != tasks {
                                                view.tasks = tasks.clone();
                                                let live = tasks
                                                    .iter()
                                                    .map(|task| task.stable_id.as_str())
                                                    .collect::<std::collections::BTreeSet<_>>();
                                                view.overlays
                                                    .retain(|stable_id, _| live.contains(stable_id.as_str()));
                                                trace_action("shell-event");
                                                cx.notify();
                                            } else if task_visual_changed || settings_changed {
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
                                for slot in &refresh_system_flyouts {
                                    let handle = slot
                                        .borrow()
                                        .as_ref()
                                        .map(|(_, handle)| *handle);
                                    if let Some(handle) = handle {
                                        let updated = handle.update(app, |view, _, cx| {
                                            if view.snapshot != current_system_snapshot
                                                || view.status != current_status
                                            {
                                                view.snapshot = current_system_snapshot.clone();
                                                view.status = current_status.clone();
                                                cx.notify();
                                            }
                                        });
                                        if updated.is_err() {
                                            slot.borrow_mut().take();
                                        }
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
                let context_for_timer = Rc::clone(&taskbar_context_window);
                let settings_for_timer = Rc::clone(&taskbar_settings_window);
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
                            for slot in system_flyout_windows {
                                if let Some((_, handle)) = slot.borrow_mut().take() {
                                    let _ = handle.update(app, |_, window, _| window.remove_window());
                                }
                            }
                            if let Some(handle) = context_for_timer.borrow_mut().take() {
                                let _ = handle.update(app, |_, window, _| window.remove_window());
                            }
                            if let Some(handle) = settings_for_timer.borrow_mut().take() {
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

#[cfg(test)]
mod live_parity_tests {
    use super::{
        AttentionRuntime, DEFAULT_FLASH_EDGES, HSHELL_FLASH, HSHELL_WINDOWACTIVATED,
        ICON_CACHE_LIMIT, MonitorRecord, prune_icon_cache, reconcile_desktop_item_positions,
        start_window_geometry,
    };
    use desktop_ui::AccessibleNode;
    use gpui::{WindowBounds, point, px};
    use platform_win::common::appbar_shell_hook::OwnedShellHookEvent;
    use platform_win::common::monitor_dpi_start::ScreenRect;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;
    use std::time::Instant;

    #[test]
    fn shell_attention_flashes_then_holds_until_activation() {
        let start = Instant::now();
        let cadence = std::time::Duration::from_millis(u64::from(
            platform_win::common::appbar_shell_hook::system_attention_cadence_ms(),
        ));
        let mut runtime = AttentionRuntime::default();
        runtime.apply(
            OwnedShellHookEvent {
                code: HSHELL_FLASH,
                hwnd_identity: 0x2a,
                process_id: 1,
                session_id: 1,
            },
            start,
        );
        assert_eq!(runtime.visual_for("win:1:2A"), (true, false));
        runtime.tick(start + cadence);
        assert_eq!(runtime.visual_for("win:1:2A"), (false, false));
        runtime.tick(start + cadence * u32::from(DEFAULT_FLASH_EDGES));
        assert_eq!(runtime.visual_for("win:1:2A"), (true, true));
        runtime.apply(
            OwnedShellHookEvent {
                code: HSHELL_WINDOWACTIVATED,
                hwnd_identity: 0x2a,
                process_id: 1,
                session_id: 1,
            },
            start,
        );
        assert_eq!(runtime.visual_for("win:1:2A"), (false, false));
    }

    fn product_start_composition_is_owned(source: &str) -> bool {
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        production.contains("StartView::new")
            && production.contains("start:owned-opened")
            && production.contains("start:closed")
            && !production.contains("invoke_start_host_controlled")
            && !production.contains("SUPERDESKTOP_VERIFICATION_OWNED_START")
    }

    fn product_status_has_no_fixed_provider_values(source: &str) -> bool {
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        !production.contains("ProviderState::Available(\"online\"")
            && !production.contains("ProviderState::Available(40)")
            && !production.contains("ProviderState::Available(false)")
            && !production.contains("ProviderState::Available(\"zh-TW\"")
            && production.contains("system-status-host-not-ready")
    }

    #[test]
    fn product_status_source_rejects_fixed_provider_values() {
        assert!(product_status_has_no_fixed_provider_values(include_str!(
            "surface_runtime.rs"
        )));
        let fixed_fixture = r#"
            CoreStatus {
                network: ProviderState::Available("online".into()),
                volume: ProviderState::Available(40),
                muted: ProviderState::Available(false),
                input_language: ProviderState::Available("zh-TW".into()),
            }
        "#;
        assert!(!product_status_has_no_fixed_provider_values(fixed_fixture));
    }

    #[test]
    fn unavailable_status_providers_remain_independent() {
        let region = super::status(None);
        assert!(matches!(
            region.core.network,
            taskbar_ui::ProviderState::Unavailable(_)
        ));
        assert!(matches!(
            region.core.volume,
            taskbar_ui::ProviderState::Unavailable(_)
        ));
        assert!(matches!(
            region.core.muted,
            taskbar_ui::ProviderState::Unavailable(_)
        ));
        assert!(matches!(
            region.core.input_language,
            taskbar_ui::ProviderState::Unavailable(_)
        ));
        assert!(matches!(
            region.core.battery,
            taskbar_ui::ProviderState::Unavailable(_)
        ));
        assert!(matches!(
            region.core.notifications,
            taskbar_ui::ProviderState::Unavailable(_)
        ));
    }

    #[test]
    fn product_start_source_is_exclusively_owned_in_every_mode() {
        let source = include_str!("surface_runtime.rs");
        let composition = include_str!("lib.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(product_start_composition_is_owned(source));
        assert_eq!(production.matches("StartView::new").count(), 1);
        assert!(!production.contains("if !shell"));
        assert!(!composition.contains("invoke_start_host_controlled"));
        assert!(!composition.contains("start-host-unavailable"));
    }

    #[test]
    fn owned_start_guard_rejects_a_delegated_callback_fixture() {
        let delegated = r#"
            start: Rc::new(move |_| {
                platform_win::common::monitor_dpi_start::invoke_start_host_controlled();
            })
        "#;
        assert!(!product_start_composition_is_owned(delegated));
    }

    #[test]
    fn historical_platform_start_probe_remains_isolated_from_product_composition() {
        let adapter = include_str!("../../platform-win/src/common/monitor_dpi_start.rs");
        assert!(adapter.contains("invoke_start_host_controlled"));
        assert!(product_start_composition_is_owned(include_str!(
            "surface_runtime.rs"
        )));
    }

    #[test]
    fn production_status_uses_platform_clock_and_refreshes_changed_values() {
        let source = include_str!("surface_runtime.rs");
        assert!(source.contains("taskbar_status::local_date_time()"));
        assert!(source.contains("if view.status != current_status"));
        assert!(source.contains("view.status = current_status.clone()"));
        assert!(!source.contains("year: 2026,\n            month: 8,\n            day: 14,"));
    }

    #[test]
    fn input_profile_command_restores_owned_start_focus_without_mutating_composition() {
        let source = include_str!("surface_runtime.rs");
        let start = include_str!("../../taskbar-ui/src/start.rs");
        assert!(source.contains("start:ime-focus-restored"));
        assert!(source.contains("SystemStatusAction::ActivateInputProfile"));
        assert!(source.contains("let _ = start.update(app"));
        assert!(source.contains("window.activate_window();"));
        assert!(start.contains("let composition = self.model.composition.clone();"));
        let status_callback = source
            .split("system_status: Rc::new")
            .nth(1)
            .and_then(|tail| tail.split("rendered: Rc::new").next())
            .expect("system status callback source");
        assert!(!status_callback.contains("composition"));
    }

    #[test]
    fn system_status_supervision_and_owned_flyouts_are_mode_independent() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert_eq!(
            production.matches("SystemStatusClient::adjacent()").count(),
            1
        );
        assert!(production.contains("status:restart-capacity-exhausted"));
        assert!(production.contains("status:owned-flyout-opened"));
        let callback = production
            .split("system_flyout: Rc::new")
            .nth(1)
            .and_then(|tail| tail.split("rendered: Rc::new").next())
            .expect("owned system flyout callback");
        assert!(!callback.contains("if shell"));
    }

    #[test]
    fn icon_cache_reuses_live_entries_prunes_stale_and_stays_bounded() {
        let mut cache = (0..ICON_CACHE_LIMIT + 4)
            .map(|index| (format!("icon:{index:04}"), None))
            .collect::<BTreeMap<_, _>>();
        let live = cache.keys().cloned().collect::<BTreeSet<_>>();
        prune_icon_cache(&mut cache, &live);
        assert_eq!(cache.len(), ICON_CACHE_LIMIT);
        let retained = cache.keys().next().unwrap().clone();
        prune_icon_cache(&mut cache, &std::iter::once(retained.clone()).collect());
        assert_eq!(cache.keys().cloned().collect::<Vec<_>>(), vec![retained]);
    }

    #[test]
    fn development_superexplorer_icon_source_is_discoverable_when_present() {
        let development_binary = std::env::current_exe().ok().and_then(|path| {
            path.ancestors()
                .find(|ancestor| {
                    ancestor
                        .file_name()
                        .is_some_and(|name| name == "SuperDesktop")
                })
                .and_then(Path::parent)
                .map(|root| root.join("target/release/SuperExplorer.exe"))
        });
        if development_binary.is_some_and(|path| path.is_file()) {
            let resolved = super::superexplorer_executable().unwrap();
            assert!(
                platform_win::common::icon::shell_icon_for_path(&resolved, 48).is_some(),
                "missing icon for {}",
                resolved.display()
            );
        }
    }

    #[test]
    fn fixed_desktop_entry_reserves_its_cell_and_rehomes_overlapping_items() {
        let nodes = vec![
            AccessibleNode::fixed_superexplorer("fixture", false, false),
            AccessibleNode {
                stable_id: "desktop-item:overlap".into(),
                name: "Overlap".into(),
                icon: None,
                role: "button",
                selected: false,
                focused: false,
                actions: Vec::new(),
                message_key: None,
            },
            AccessibleNode {
                stable_id: "desktop-item:next".into(),
                name: "Next".into(),
                icon: None,
                role: "button",
                selected: false,
                focused: false,
                actions: Vec::new(),
                message_key: None,
            },
            AccessibleNode {
                stable_id: "desktop-item:manual".into(),
                name: "Manual".into(),
                icon: None,
                role: "button",
                selected: false,
                focused: false,
                actions: Vec::new(),
                message_key: None,
            },
        ];
        let persisted = BTreeMap::from([
            ("desktop-item:overlap".into(), (10.0, 18.0)),
            ("desktop-item:next".into(), (8.0, 120.0)),
            ("desktop-item:manual".into(), (500.0, 64.0)),
        ]);
        let positions = reconcile_desktop_item_positions(&nodes, &persisted);
        assert_eq!(positions["desktop:fixture:superexplorer"], (8.0, 8.0));
        assert_eq!(positions["desktop-item:overlap"], (8.0, 120.0));
        assert_eq!(positions["desktop-item:next"], (8.0, 232.0));
        assert_eq!(positions["desktop-item:manual"], (500.0, 64.0));
    }

    #[test]
    fn start_geometry_centers_clamps_and_preserves_bottom_gap() {
        let monitor = MonitorRecord {
            device_name: "fixture".into(),
            primary: true,
            bounds: ScreenRect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            work_area: ScreenRect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1000,
            },
            dpi_x: 168,
            dpi_y: 168,
        };
        let geometry = start_window_geometry(&monitor);
        let logical_width = 1920.0 / 1.75;
        let logical_bottom = 1000.0 / 1.75;
        assert_eq!(geometry.width, 640.0);
        assert!((geometry.left - (logical_width - 640.0) / 2.0).abs() < 0.01);
        assert!((logical_bottom - geometry.top - geometry.height - 12.0).abs() < 0.01);

        let mut small = monitor;
        small.work_area.right = 500;
        small.work_area.bottom = 400;
        small.dpi_x = 96;
        small.dpi_y = 96;
        let geometry = start_window_geometry(&small);
        assert_eq!((geometry.left, geometry.top), (12.0, 0.0));
        assert_eq!((geometry.width, geometry.height), (476.0, 388.0));
    }

    #[test]
    fn taskbar_context_geometry_clamps_to_monitor_and_settings_fit_work_area() {
        let monitor = MonitorRecord {
            device_name: "fixture".into(),
            primary: true,
            bounds: ScreenRect {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1080,
            },
            work_area: ScreenRect {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1040,
            },
            dpi_x: 144,
            dpi_y: 144,
        };
        let (left, _, width, _) =
            super::taskbar_context_placement(&monitor, point(px(9_999.), px(0.)));
        assert!(left >= -1920);
        assert!(left + width <= 0);
        let settings = super::taskbar_settings_options(&monitor);
        let Some(WindowBounds::Windowed(settings)) = settings.window_bounds else {
            panic!("bounds")
        };
        let work_width = (monitor.work_area.right - monitor.work_area.left) as f32;
        let work_height = (monitor.work_area.bottom - monitor.work_area.top) as f32;
        assert_eq!(settings.size.width.as_f32(), 980.0 * 1.5);
        assert_eq!(settings.size.height.as_f32(), work_height);
        assert!(settings.size.width.as_f32() <= work_width);
        assert!(settings.size.height.as_f32() <= work_height);
    }

    #[test]
    fn owned_taskbar_context_settings_are_atomic_and_never_delegate_to_explorer() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for token in [
            "taskbar_context: Rc::new",
            "TaskbarContextView::new",
            "TaskbarSettingsView::new",
            "current.revision != base_revision",
            "save_store.borrow_mut().save",
            "taskbar:settings-saved",
            "launch_task_manager()",
        ] {
            assert!(production.contains(token), "missing {token}");
        }
        assert!(!production.contains("ms-settings:taskbar"));
        assert!(!production.contains("Shell_TrayWnd"));
        assert!(include_str!("../../taskbar-ui/src/view.rs").contains("cx.stop_propagation();"));
    }
}
