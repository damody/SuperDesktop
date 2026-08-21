use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use desktop_ui::{
    AccessibleAction, AccessibleNode, DeletePolicy, DesktopItem, DesktopOperation,
    DesktopOperationController, DesktopOperationRequest, DesktopOperationTerminal,
    DesktopTransferStatus, DesktopView, MenuModel, TransferIntent, execute_desktop_operation,
};
use gpui::{
    App, AppContext, Bounds, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, point, px, size,
};
use platform_win::common::{
    appbar_shell_hook::{
        ControlledShellCapability, OwnedShellHookEvent, ScreenRect, system_attention_cadence_ms,
    },
    desktop::{configure_and_show_desktop_window, current_wallpaper_path},
    explorer_recovery::trusted_explorer_shell_present,
    monitor_dpi_start::{MonitorRecord, enable_per_monitor_v2, snapshot_real_monitors},
    shell_hotkey::ShellHotkeyAction,
    taskbar::{
        MinimizedWindowShelf, OwnedTaskWindow, configure_and_show_taskbar_window,
        move_owned_taskbar_client, owned_taskbar_resize_active, physical_cursor_position,
        post_owned_taskbar_reveal, promote_owned_popup_topmost, set_owned_taskbar_auto_hide_clip,
        snapshot_task_windows,
    },
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use settings_store::{
    DesktopSortDirection, DesktopSortKey, TaskbarAlignment, TaskbarSearchMode, TaskbarSettings,
};
use shell_provider_protocol::{
    CURRENT_PROTOCOL, CommandDescriptor, CommandId, CommandRisk, Envelope, IconData, IconKey,
    JumpListRequest, MenuContext, MenuEnumeration, MenuInvocation, NotificationEvent,
    NotificationEventKind, NotificationHostResponse, NotificationMutation, ProviderRequest,
    ResponseBody, SearchBatch, SearchQuery, StatusAvailability, SystemStatusCommand,
    SystemStatusCommandRequest, SystemStatusHostRequest, SystemStatusHostResponse,
    SystemStatusSnapshot, SystemStatusTerminalKind, TaskbarProgressKind, TaskbarStateSnapshot,
    TaskbarWindowState, TerminalKind, reduce_group_progress,
};
use taskbar_ui::{
    AccessibleTask, AltTabView, AutoHideEffect, AutoHideInput, AutoHideState, ClockLocale,
    CoreStatus, FlyoutAction, HOVER_PREVIEW_CLOSE_GRACE_MS, HOVER_PREVIEW_DELAY_MS,
    HoverPreviewController, JumpListModel, JumpListView, NotificationAreaModel,
    NotificationCenterAction, NotificationOverflowView, PreviewCard, ProgressState, ProviderState,
    ShowDesktopObservation, ShowDesktopPlan, ShowDesktopSession, ShowDesktopTarget, StartActions,
    StartPowerAction, StartSnapshot, StartView, StatusRegion, SystemControlContextCommand,
    SystemControlContextKind, SystemControlContextView, SystemFlyoutKind, SystemFlyoutPresentation,
    SystemFlyoutTheme, SystemFlyoutView, SystemStatusAction, TaskAction, TaskFlyoutView,
    TaskViewEffect, TaskViewModel, TaskViewSurface, TaskbarCallbacks, TaskbarContextCommand,
    TaskbarContextView, TaskbarLayout, TaskbarSettingId, TaskbarSettingsEffect,
    TaskbarSettingsView, TaskbarView, TestClock, WindowsGuiMetrics, auto_hide_endpoints,
    reduce_auto_hide,
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

#[derive(Clone)]
struct VolumeCommandCoordinator {
    desired: Arc<std::sync::atomic::AtomicU8>,
    pending: Arc<AtomicBool>,
    wake: SyncSender<()>,
}

impl VolumeCommandCoordinator {
    fn start() -> Result<Self, &'static str> {
        let desired = Arc::new(std::sync::atomic::AtomicU8::new(0));
        let pending = Arc::new(AtomicBool::new(false));
        let (wake, receiver) = mpsc::sync_channel(1);
        let worker_desired = Arc::clone(&desired);
        let worker_pending = Arc::clone(&pending);
        std::thread::Builder::new()
            .name("superdesktop-volume-command".into())
            .spawn(move || {
                let mut client = match SystemStatusClient::adjacent() {
                    Ok(client) => client,
                    Err(error) => {
                        report_error("status:volume-worker", error);
                        return;
                    }
                };
                let mut reconciler = StatusReconciler::default();
                while receiver.recv().is_ok() {
                    loop {
                        let volume_percent = worker_desired.load(Ordering::Acquire).min(100);
                        if reconciler.snapshot().is_none() {
                            match client.request(
                                &SystemStatusHostRequest::Snapshot,
                                Duration::from_millis(750),
                            ) {
                                Ok(snapshot) => {
                                    reconciler.apply(snapshot);
                                }
                                Err(error) => {
                                    report_error("status:volume-snapshot", error);
                                }
                            }
                        }
                        let result = execute_system_status_command(
                            SystemStatusCommand::SetVolume { volume_percent },
                            Duration::from_millis(1_000),
                            &mut reconciler,
                            |request, timeout| client.request(request, timeout),
                            unix_time_ms,
                        );
                        if let Err(error) = result {
                            report_error("status:volume-command", error.message());
                            if error.provider_failed() {
                                reconciler.provider_unavailable();
                            }
                        }
                        worker_pending.store(false, Ordering::Release);
                        let latest = worker_desired.load(Ordering::Acquire).min(100);
                        if latest == volume_percent
                            || worker_pending
                                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                                .is_err()
                        {
                            break;
                        }
                    }
                }
            })
            .map_err(|_| "status-volume-worker-spawn")?;
        Ok(Self {
            desired,
            pending,
            wake,
        })
    }

    fn submit(&self, volume_percent: u8) {
        self.desired
            .store(volume_percent.min(100), Ordering::Release);
        if self
            .pending
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            match self.wake.try_send(()) {
                Ok(()) | Err(TrySendError::Full(())) => {}
                Err(TrySendError::Disconnected(())) => {
                    self.pending.store(false, Ordering::Release);
                    report_error("status:volume-command", "volume worker is unavailable");
                }
            }
        }
    }
}

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

#[derive(Default)]
struct ProviderRefreshBatch {
    notification: Option<Result<NotificationHostResponse, String>>,
    status: Option<Result<SystemStatusHostResponse, String>>,
    taskbar: Option<Result<TaskbarStateSnapshot, String>>,
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
    let recoverable_warning = action == "taskbar:appbar-unavailable-owned-shell";
    if !recoverable_warning
        && !action.starts_with("error:")
        && ["error", "failed", "rejected", "unavailable", "exhausted"]
            .iter()
            .any(|marker| action.contains(marker))
    {
        eprintln!("SuperDesktop error: {action}");
    }
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

fn report_error(context: &str, error: impl std::fmt::Display) {
    eprintln!("SuperDesktop error [{context}]: {error}");
    trace_action(&format!("error:{context}:{error}"));
}

fn promote_owned_context_popup(window: &mut Window, kind: &str) -> bool {
    let promoted = hwnd(window)
        .map_err(str::to_owned)
        .and_then(promote_owned_popup_topmost);
    match promoted {
        Ok(()) => {
            trace_action(&format!("{kind}:topmost-established"));
            true
        }
        Err(error) => {
            report_error(&format!("{kind}:topmost"), error);
            window.remove_window();
            false
        }
    }
}

fn force_appbar_unavailable_for_verification() -> bool {
    std::env::var_os("SUPERDESKTOP_VERIFICATION_SURFACE").is_some()
        && std::env::var("SUPERDESKTOP_TEST_FORCE_APPBAR_UNAVAILABLE").as_deref() == Ok("1")
}

fn guard_ui_action(context: &str, action: impl FnOnce()) {
    if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        report_error(context, format!("panic contained: {message}"));
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

fn resolve_superexplorer() -> Result<explorer_bridge::ResolvedExecutable, String> {
    let current = std::env::current_exe().ok();
    let adjacent = current
        .as_ref()
        .and_then(|path| path.parent().map(|parent| parent.join("SuperExplorer.exe")))
        .unwrap_or_else(|| PathBuf::from(r"C:\missing\SuperExplorer.exe"));
    let developer_release = current
        .as_ref()
        .and_then(|path| {
            path.ancestors()
                .find(|ancestor| {
                    ancestor
                        .file_name()
                        .is_some_and(|name| name == "SuperDesktop")
                })
                .and_then(Path::parent)
        })
        .map(|workspace| {
            let release = workspace.join("target/release/SuperExplorer.exe");
            if release.is_file() {
                release
            } else {
                workspace.join("target/debug/SuperExplorer.exe")
            }
        })
        .unwrap_or_else(|| adjacent.clone());
    let resolver = explorer_bridge::ExecutableResolver {
        setting: std::env::var_os("SUPEREXPLORER_PATH").map(PathBuf::from),
        developer_release,
        adjacent,
    };
    resolver
        .resolve()
        .map(|(resolved, _)| resolved)
        .map_err(|trace| {
            format!(
                "SuperExplorer resolver rejected all candidates: {:?}",
                trace.decisions
            )
        })
}

fn launch_superexplorer_at(initial_path: Option<&Path>) {
    match resolve_superexplorer() {
        Ok(resolved) => {
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

struct PendingSuperExplorerFocus {
    process_id: u32,
    executable: PathBuf,
    deadline: Instant,
}

fn focus_superexplorer_window(executable: &Path, process_id: Option<u32>) -> Result<bool, String> {
    let executable = executable
        .canonicalize()
        .map_err(|error| format!("SuperExplorer canonical path failed: {error}"))?
        .to_string_lossy()
        .to_ascii_lowercase();
    let windows = snapshot_task_windows()?;
    let candidate = windows.into_iter().find(|window| {
        if window.tool_window || window.cloaked || window.owned_transient {
            return false;
        }
        if process_id.is_some_and(|process_id| window.process_id == process_id) {
            return true;
        }
        PathBuf::from(&window.application_identity)
            .canonicalize()
            .is_ok_and(|observed| observed.to_string_lossy().to_ascii_lowercase() == executable)
    });
    let Some(candidate) = candidate else {
        return Ok(false);
    };
    platform_win::common::taskbar::apply_window_action(
        candidate.hwnd_identity,
        platform_win::common::taskbar::WindowAction::RestoreAndActivate,
    )?;
    Ok(true)
}

fn request_superexplorer_foreground() -> Option<PendingSuperExplorerFocus> {
    let resolved = match resolve_superexplorer() {
        Ok(resolved) => resolved,
        Err(error) => {
            report_error("win-e:resolve", error);
            return None;
        }
    };
    match focus_superexplorer_window(&resolved.path, None) {
        Ok(true) => {
            trace_action("win-e:superexplorer-activated");
            return None;
        }
        Ok(false) => {}
        Err(error) => report_error("win-e:existing-window", error),
    }
    let spec = explorer_bridge::build_default_launch(&resolved);
    match explorer_bridge::ProcessLauncher.launch(&spec) {
        explorer_bridge::LaunchOutcome::Launched { process_id } => {
            trace_action("win-e:superexplorer-launched");
            Some(PendingSuperExplorerFocus {
                process_id,
                executable: resolved.path,
                deadline: Instant::now() + Duration::from_secs(3),
            })
        }
        explorer_bridge::LaunchOutcome::ValidationFailed(error) => {
            report_error("win-e:launch-validation", error);
            None
        }
        explorer_bridge::LaunchOutcome::SpawnFailed(error) => {
            report_error("win-e:launch", error);
            None
        }
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
            let available = (0..)
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
                });
            match available {
                Some(position) => position,
                None => {
                    report_error(
                        "desktop:grid",
                        "no free desktop grid position; retaining the default position",
                    );
                    default
                }
            }
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

fn taskbar_local_commands(
    pinned: bool,
    target_minimized: Option<bool>,
    application_window_count: usize,
) -> Vec<CommandDescriptor> {
    let mut commands = vec![
        CommandDescriptor {
            id: CommandId("local:taskbar-minimize".into()),
            label: "Minimize".into(),
            enabled: target_minimized.is_some_and(|minimized| !minimized),
            risk: CommandRisk::Normal,
            children: Vec::new(),
        },
        CommandDescriptor {
            id: CommandId("local:taskbar-maximize".into()),
            label: "Maximize".into(),
            enabled: target_minimized.is_some(),
            risk: CommandRisk::Normal,
            children: Vec::new(),
        },
        CommandDescriptor {
            id: CommandId("local:taskbar-pin".into()),
            label: if pinned {
                "Unpin from taskbar"
            } else {
                "Pin to taskbar"
            }
            .into(),
            enabled: true,
            risk: CommandRisk::Normal,
            children: Vec::new(),
        },
    ];
    let grouped = application_window_count > 1;
    commands.push(CommandDescriptor {
        id: CommandId(
            if grouped {
                "local:taskbar-close-all"
            } else {
                "local:taskbar-close-window"
            }
            .into(),
        ),
        label: if grouped {
            "Close all windows"
        } else {
            "Close window"
        }
        .into(),
        enabled: application_window_count > 0,
        risk: CommandRisk::Destructive,
        children: Vec::new(),
    });
    commands
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

fn reconcile_minimized_window_shelf(
    shelf: &Rc<RefCell<MinimizedWindowShelf>>,
    windows: &[OwnedTaskWindow],
) {
    let report = shelf.borrow_mut().reconcile(windows);
    for window_identity in report.newly_shelved {
        trace_action(&format!("task:minimized-shelved:{window_identity}"));
    }
    for (window_identity, error) in report.failures {
        report_error(
            "task:minimized-shelf",
            format!("{window_identity}: {error}"),
        );
    }
}

fn reconcile_minimized_window_shelf_snapshot(shelf: &Rc<RefCell<MinimizedWindowShelf>>) {
    match snapshot_task_windows() {
        Ok(windows) => reconcile_minimized_window_shelf(shelf, &windows),
        Err(error) => report_error("task:minimized-shelf-snapshot", error),
    }
}

fn activate_task(
    stable_id: &str,
    observed_foreground: bool,
    observed_minimized: bool,
    minimized_window_shelf: &Rc<RefCell<MinimizedWindowShelf>>,
) {
    trace_action("task");
    trace_action(&format!("task:identity:{stable_id}"));
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
    let action = if observed_foreground {
        platform_win::common::taskbar::WindowAction::Minimize
    } else {
        platform_win::common::taskbar::WindowAction::RestoreAndActivate
    };
    let trace = match action {
        platform_win::common::taskbar::WindowAction::Minimize => "task:left-minimized",
        platform_win::common::taskbar::WindowAction::RestoreAndActivate if observed_minimized => {
            "task:left-restored-activated"
        }
        platform_win::common::taskbar::WindowAction::RestoreAndActivate => "task:left-activated",
        platform_win::common::taskbar::WindowAction::Activate => "task:left-activated",
        _ => "task:left-action",
    };
    if platform_win::common::taskbar::apply_window_action_to_owned_identity(
        window.hwnd_identity,
        window.process_id,
        &window.window_identity,
        action,
    )
    .is_ok()
    {
        if action == platform_win::common::taskbar::WindowAction::Minimize {
            reconcile_minimized_window_shelf_snapshot(minimized_window_shelf);
        }
        trace_action(trace);
    } else {
        trace_action("task:left-action-rejected");
    }
}

fn show_desktop_observation(
    window: platform_win::common::taskbar::OwnedTaskWindow,
) -> ShowDesktopObservation {
    ShowDesktopObservation {
        target: ShowDesktopTarget {
            hwnd_identity: window.hwnd_identity,
            process_id: window.process_id,
            window_identity: window.window_identity,
        },
        visible: window.visible,
        tool_window: window.tool_window,
        cloaked: window.cloaked,
        owned_transient: window.owned_transient,
        minimized: window.minimized,
    }
}

fn run_show_desktop_cycle(
    session: &Rc<RefCell<ShowDesktopSession>>,
    minimized_window_shelf: &Rc<RefCell<MinimizedWindowShelf>>,
) {
    let Ok(windows) = snapshot_task_windows() else {
        trace_action("show-desktop:snapshot-rejected");
        return;
    };
    let windows = minimized_window_shelf.borrow().task_windows(windows);
    let snapshot = windows
        .into_iter()
        .map(show_desktop_observation)
        .collect::<Vec<_>>();
    let plan = session.borrow().plan(&snapshot);
    match plan {
        ShowDesktopPlan::Minimize(targets) => {
            let succeeded = targets
                .into_iter()
                .filter(|target| {
                    platform_win::common::taskbar::apply_window_action_to_owned_identity(
                        target.hwnd_identity,
                        target.process_id,
                        &target.window_identity,
                        platform_win::common::taskbar::WindowAction::Minimize,
                    )
                    .is_ok()
                })
                .collect::<Vec<_>>();
            let active = !succeeded.is_empty();
            if active {
                reconcile_minimized_window_shelf_snapshot(minimized_window_shelf);
            }
            session.borrow_mut().complete_minimize(succeeded);
            trace_action(if active {
                "show-desktop:minimized"
            } else {
                "show-desktop:no-targets"
            });
        }
        ShowDesktopPlan::Restore(targets) => {
            for target in targets {
                if let Err(error) =
                    platform_win::common::taskbar::unshelve_minimized_window_to_owned_identity(
                        target.hwnd_identity,
                        target.process_id,
                        &target.window_identity,
                    )
                {
                    report_error("show-desktop:unshelve", error);
                    continue;
                }
                minimized_window_shelf
                    .borrow_mut()
                    .begin_restore(&target.window_identity);
                if let Err(error) =
                    platform_win::common::taskbar::apply_window_action_to_owned_identity(
                        target.hwnd_identity,
                        target.process_id,
                        &target.window_identity,
                        platform_win::common::taskbar::WindowAction::Restore,
                    )
                {
                    report_error("show-desktop:restore", error);
                }
            }
            session.borrow_mut().complete_restore();
            trace_action("show-desktop:restored");
        }
    }
}

fn apply_taskbar_context_setting(
    settings: &mut TaskbarSettings,
    command: TaskbarContextCommand,
) -> bool {
    match command {
        TaskbarContextCommand::CycleSearchMode => {
            settings.search_mode = match settings.search_mode {
                TaskbarSearchMode::Hidden => TaskbarSearchMode::Icon,
                TaskbarSearchMode::Icon => TaskbarSearchMode::Box,
                TaskbarSearchMode::Box => TaskbarSearchMode::Hidden,
            };
            true
        }
        TaskbarContextCommand::ToggleTaskView => {
            settings.show_task_view = !settings.show_task_view;
            true
        }
        TaskbarContextCommand::ToggleLockTaskbar => {
            settings.locked = !settings.locked;
            true
        }
        _ => false,
    }
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

type TaskFlyoutSlot = Rc<RefCell<Option<gpui::WindowHandle<TaskFlyoutView>>>>;
type AltTabSlot = Rc<RefCell<Option<gpui::WindowHandle<AltTabView>>>>;

fn preview_card_width_for_size(width: i32, height: i32, maximum_width: f32) -> u16 {
    if width <= 0 || height <= 0 || maximum_width <= 0.0 {
        return 1;
    }
    ((width as f32 / height as f32) * 188.0 + 16.0)
        .round()
        .clamp(160.0_f32.min(maximum_width), maximum_width) as u16
}

fn preview_cards(
    stable_id: &str,
    previews_enabled: bool,
    monitor: &MonitorRecord,
) -> Vec<PreviewCard> {
    let group = group_window_ids(stable_id);
    let selected = if group.is_empty() {
        task_hwnd(stable_id).into_iter().collect::<Vec<_>>()
    } else {
        group
    };
    let selected = selected.into_iter().take(4).collect::<Vec<_>>();
    let card_count = selected.len().max(1);
    let scale = monitor.dpi_x as f32 / 96.0;
    let available_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale - 16.0;
    let gaps = 8.0 * card_count.saturating_sub(1) as f32;
    let maximum_card_width = ((available_width - gaps) / card_count as f32)
        .floor()
        .clamp(1.0, 420.0);
    let Ok(windows) = snapshot_task_windows() else {
        return Vec::new();
    };
    let now = unix_time_ms();
    windows
        .into_iter()
        .filter(|window| selected.contains(&window.hwnd_identity))
        .take(4)
        .filter_map(|window| {
            let window_id = shell_core::WindowId::new(window.window_identity).ok()?;
            let preview_available = previews_enabled
                && matches!(
                    platform_win::common::taskbar_preview::admit_live_preview(
                        window.hwnd_identity,
                        false,
                        now,
                        unix_time_ms(),
                    ),
                    platform_win::common::taskbar_preview::PreviewAdmission::Available { .. }
                );
            let preview_width =
                platform_win::common::taskbar_preview::source_client_size(window.hwnd_identity)
                    .map(|(width, height)| {
                        preview_card_width_for_size(width, height, maximum_card_width)
                    })
                    .unwrap_or_else(|_| preview_card_width_for_size(1, 1, maximum_card_width));
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
                preview_width,
            })
        })
        .collect()
}

fn alt_tab_cards() -> Vec<PreviewCard> {
    let Ok(windows) = snapshot_task_windows() else {
        return Vec::new();
    };
    let now = unix_time_ms();
    windows
        .into_iter()
        .filter_map(|window| {
            let window_id = shell_core::WindowId::new(window.window_identity).ok()?;
            let preview_available = matches!(
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
                preview_width: 220,
            })
        })
        .collect()
}

fn alt_tab_options(monitor: &MonitorRecord, card_count: usize) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let work_left = monitor.work_area.left as f32 / scale;
    let work_top = monitor.work_area.top as f32 / scale;
    let work_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let work_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    let columns = card_count.clamp(1, 4);
    let rows = card_count.max(1).div_ceil(columns).min(3);
    let width = (columns as f32 * 228.0 + 16.0).min((work_width - 16.0).max(1.0));
    let height = (rows as f32 * 178.0 + 16.0).min((work_height - 16.0).max(1.0));
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                px(work_left + (work_width - width) / 2.0),
                px(work_top + (work_height - height) / 2.0),
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

fn open_or_cycle_alt_tab(app: &mut App, slot: &AltTabSlot, monitor: &MonitorRecord, delta: i32) {
    if let Some(handle) = *slot.borrow()
        && handle
            .update(app, |view, _, cx| view.cycle(delta, cx))
            .is_ok()
    {
        return;
    }
    slot.borrow_mut().take();
    let cards = alt_tab_cards();
    if cards.is_empty() {
        return;
    }
    let dismiss_slot = Rc::clone(slot);
    let opened = app.open_window(
        alt_tab_options(monitor, cards.len().min(12)),
        move |window, cx| {
            let destination_hwnd = hwnd(window).unwrap_or_default();
            if promote_owned_popup_topmost(destination_hwnd).is_err() {
                window.remove_window();
            }
            cx.new(|_| {
                AltTabView::new(
                    cards,
                    delta,
                    Rc::new(apply_flyout_action),
                    Rc::new(move |window, _| {
                        window.remove_window();
                        *dismiss_slot.borrow_mut() = None;
                    }),
                    destination_hwnd,
                )
            })
        },
    );
    if let Ok(handle) = opened {
        *slot.borrow_mut() = Some(handle);
        trace_action("alt-tab:opened");
    }
}

fn close_alt_tab(app: &mut App, slot: &AltTabSlot, commit: bool) {
    let Some(handle) = slot.borrow_mut().take() else {
        return;
    };
    let action = handle
        .update(app, |view, window, _| {
            let action = commit.then(|| view.selected_action()).flatten();
            window.remove_window();
            action
        })
        .ok()
        .flatten();
    if let Some(action) = action {
        apply_flyout_action(action);
        trace_action("alt-tab:committed");
    } else {
        trace_action("alt-tab:cancelled");
    }
}

fn schedule_preview_close(
    app: &mut App,
    controller: Rc<RefCell<HoverPreviewController>>,
    slot: TaskFlyoutSlot,
    active_popup: Rc<Cell<isize>>,
    token: u64,
) {
    let background = app.background_executor().clone();
    let foreground = app.foreground_executor().clone();
    let async_app = app.to_async();
    foreground
        .spawn(async move {
            background
                .timer(Duration::from_millis(HOVER_PREVIEW_CLOSE_GRACE_MS))
                .await;
            if let Err(error) = async_app.try_update(|app| {
                if controller.borrow().can_close(token)
                    && let Some(handle) = slot.borrow_mut().take()
                {
                    let _ = handle.update(app, |_, window, _| window.remove_window());
                    active_popup.set(0);
                    trace_action("task-preview:hover-closed");
                }
            }) {
                report_error("task-preview:close-update", error);
            }
        })
        .detach();
}

fn schedule_preview_pointer_monitor(
    app: &mut App,
    slot: TaskFlyoutSlot,
    active_popup: Rc<Cell<isize>>,
    taskbar_hwnd: isize,
    popup_hwnd: isize,
) {
    let background = app.background_executor().clone();
    let foreground = app.foreground_executor().clone();
    let async_app = app.to_async();
    foreground
        .spawn(async move {
            let mut outside_since = None::<Instant>;
            loop {
                background.timer(Duration::from_millis(50)).await;
                if active_popup.get() != popup_hwnd {
                    return;
                }
                let cursor_root = platform_win::common::taskbar::cursor_root_window().unwrap_or(0);
                if cursor_root == taskbar_hwnd || cursor_root == popup_hwnd {
                    outside_since = None;
                    continue;
                }
                let start = *outside_since.get_or_insert_with(Instant::now);
                if start.elapsed() < Duration::from_millis(HOVER_PREVIEW_CLOSE_GRACE_MS) {
                    continue;
                }
                if let Err(error) = async_app.try_update(|app| {
                    if active_popup.get() == popup_hwnd {
                        if let Some(handle) = slot.borrow_mut().take() {
                            let _ = handle.update(app, |_, window, _| window.remove_window());
                        }
                        active_popup.set(0);
                        trace_action("task-preview:hover-closed");
                    }
                }) {
                    report_error("task-preview:pointer-monitor-update", error);
                }
                return;
            }
        })
        .detach();
}

fn open_task_preview(
    stable_id: &str,
    app: &mut App,
    slot: &TaskFlyoutSlot,
    monitor: &MonitorRecord,
    previews_enabled: bool,
    controller: Rc<RefCell<HoverPreviewController>>,
    taskbar_hwnd: isize,
    active_popup: Rc<Cell<isize>>,
    owned_shell: bool,
    taskbar_rows: u8,
    source: PreviewOpenSource,
) {
    let cards = preview_cards(stable_id, previews_enabled, monitor);
    if cards.is_empty() {
        return;
    }
    if let Some(existing) = slot.borrow_mut().take() {
        let _ = existing.update(app, |_, window, _| window.remove_window());
    }
    let dismiss_slot = Rc::clone(slot);
    let hover_slot = Rc::clone(slot);
    let hover_controller = Rc::clone(&controller);
    let popup_identity = Rc::clone(&active_popup);
    let monitor_slot = Rc::clone(slot);
    let topmost_established = Rc::new(Cell::new(false));
    let topmost_for_open = Rc::clone(&topmost_established);
    let anchor_physical_x = physical_cursor_position().ok().map(|(x, _)| x);
    let opened = app.open_window(
        task_flyout_options(
            monitor,
            &cards,
            anchor_physical_x,
            owned_shell,
            taskbar_rows,
            source,
        ),
        move |window, cx| {
            if source.activates_window() {
                window.activate_window();
            }
            let destination_hwnd = hwnd(window).unwrap_or_default();
            if promote_owned_popup_topmost(destination_hwnd).is_err() {
                trace_action("task-preview:topmost-rejected");
                window.remove_window();
            } else {
                topmost_for_open.set(true);
                trace_action("task-preview:topmost-established");
            }
            let dismiss_slot = Rc::clone(&dismiss_slot);
            let hover_slot = Rc::clone(&hover_slot);
            let hover_controller = Rc::clone(&hover_controller);
            if topmost_for_open.get() {
                popup_identity.set(destination_hwnd);
                schedule_preview_pointer_monitor(
                    cx,
                    Rc::clone(&monitor_slot),
                    Rc::clone(&popup_identity),
                    taskbar_hwnd,
                    destination_hwnd,
                );
            } else {
                popup_identity.set(0);
            }
            let dismiss_popup_identity = Rc::clone(&popup_identity);
            let hover_popup_identity = Rc::clone(&popup_identity);
            cx.new(move |cx| {
                TaskFlyoutView::new(
                    cards,
                    Rc::new(apply_flyout_action),
                    Rc::new(move |window, _| {
                        window.remove_window();
                        *dismiss_slot.borrow_mut() = None;
                        dismiss_popup_identity.set(0);
                    }),
                    Rc::new(move |hovered, app| {
                        let token = if hovered {
                            hover_controller.borrow_mut().enter_popup()
                        } else {
                            hover_controller.borrow_mut().leave_popup()
                        };
                        if !hovered {
                            schedule_preview_close(
                                app,
                                Rc::clone(&hover_controller),
                                Rc::clone(&hover_slot),
                                Rc::clone(&hover_popup_identity),
                                token,
                            );
                        }
                    }),
                    destination_hwnd,
                    source.assigns_keyboard_focus(),
                    cx,
                )
            })
        },
    );
    if let Ok(handle) = opened {
        if topmost_established.get() {
            *slot.borrow_mut() = Some(handle);
            trace_action("task-preview:hover-opened");
        } else {
            let _ = handle.update(app, |_, window, _| window.remove_window());
        }
    }
}

fn schedule_preview_open(
    app: &mut App,
    stable_id: String,
    controller: Rc<RefCell<HoverPreviewController>>,
    slot: TaskFlyoutSlot,
    monitor: MonitorRecord,
    previews_enabled: bool,
    taskbar_hwnd: isize,
    active_popup: Rc<Cell<isize>>,
    owned_shell: bool,
    taskbar_rows: u8,
    token: u64,
) {
    let background = app.background_executor().clone();
    let foreground = app.foreground_executor().clone();
    let async_app = app.to_async();
    foreground
        .spawn(async move {
            background
                .timer(Duration::from_millis(HOVER_PREVIEW_DELAY_MS))
                .await;
            if let Err(error) = async_app.try_update(|app| {
                if controller.borrow().can_open(&stable_id, token) {
                    open_task_preview(
                        &stable_id,
                        app,
                        &slot,
                        &monitor,
                        previews_enabled,
                        Rc::clone(&controller),
                        taskbar_hwnd,
                        Rc::clone(&active_popup),
                        owned_shell,
                        taskbar_rows,
                        PreviewOpenSource::Hover,
                    );
                }
            }) {
                report_error("task-preview:open-update", error);
            }
        })
        .detach();
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

fn task_icon_source_edge(monitors: &[MonitorRecord]) -> u32 {
    let maximum_dpi = monitors
        .iter()
        .flat_map(|monitor| [monitor.dpi_x, monitor.dpi_y])
        .filter(|dpi| *dpi > 0)
        .max()
        .unwrap_or(96)
        .max(96);
    let logical_edge = WindowsGuiMetrics::TASK_ICON_EDGE.ceil() as u64;
    let physical_edge = (logical_edge * u64::from(maximum_dpi)).div_ceil(96);
    u32::try_from(physical_edge)
        .unwrap_or(u32::MAX)
        .clamp(32, 64)
}

fn visible_tasks(
    pin_order: &[String],
    combine_groups: bool,
    icon_edge: u32,
    icon_cache: &mut BTreeMap<String, Option<IconData>>,
    attention_windows: &BTreeSet<(isize, u32)>,
    minimized_window_shelf: Option<&Rc<RefCell<MinimizedWindowShelf>>>,
) -> Result<Vec<AccessibleTask>, &'static str> {
    snapshot_task_windows()
        .map_err(|_| "task-window-snapshot")
        .map(|windows| {
            let windows = if let Some(shelf) = minimized_window_shelf {
                reconcile_minimized_window_shelf(shelf, &windows);
                shelf.borrow().task_windows(windows)
            } else {
                windows
            };
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
                                icon_edge,
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
    shell: bool,
    taskbar_rows: u8,
) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = (monitor.bounds.right - monitor.bounds.left) as f32 / scale;
    let height = if taskbar {
        WindowsGuiMetrics::taskbar_height(taskbar_rows)
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
                    (if shell {
                        monitor.bounds.bottom
                    } else {
                        monitor.work_area.bottom
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
        is_movable: taskbar,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Opaque,
        ..Default::default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TaskbarPhysicalGeometry {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    bottom: i32,
}

#[derive(Clone)]
struct TaskbarAutoHideRuntime {
    monitor: MonitorRecord,
    state: Rc<RefCell<AutoHideState>>,
    enabled: Rc<RefCell<bool>>,
    visibility_hold: Rc<dyn Fn() -> bool>,
    fast_hidden: Arc<AtomicBool>,
    fast_enabled: Arc<AtomicBool>,
    fast_rows: Arc<AtomicU64>,
    stop_reveal_worker: Arc<AtomicBool>,
    reveal_worker: Rc<RefCell<Option<std::thread::JoinHandle<()>>>>,
}

fn reconcile_taskbar_auto_hide(
    runtime: &TaskbarAutoHideRuntime,
    shell: bool,
    taskbar_settings: &settings_store::TaskbarSettings,
    attention_hold: bool,
    context_open: bool,
    settings_open: bool,
    leases: &Rc<RefCell<Vec<ControlledShellCapability>>>,
    view: &TaskbarView,
    window: &mut gpui::Window,
    epoch: Instant,
) {
    let taskbar_height =
        taskbar_physical_geometry(&runtime.monitor, shell, taskbar_settings.rows).height;
    let anchor_bottom = if shell {
        runtime.monitor.bounds.bottom
    } else {
        runtime.monitor.work_area.bottom
    };
    let Some(endpoints) = auto_hide_endpoints(
        runtime.monitor.bounds.left,
        runtime.monitor.bounds.right,
        anchor_bottom,
        taskbar_height,
    ) else {
        return;
    };
    let cursor = physical_cursor_position()
        .ok()
        .map(|(x, y)| taskbar_ui::PhysicalPoint { x, y });
    if cursor.is_none() {
        trace_action("taskbar:auto-hide-cursor-unavailable");
    }
    let enabled = taskbar_settings.auto_hide;
    runtime.fast_enabled.store(enabled, Ordering::Release);
    runtime
        .fast_rows
        .store(u64::from(taskbar_settings.rows), Ordering::Release);
    if *runtime.state.borrow() == AutoHideState::Hidden
        && !runtime.fast_hidden.load(Ordering::Acquire)
    {
        *runtime.state.borrow_mut() = AutoHideState::Visible;
    }
    let was_enabled = *runtime.enabled.borrow();
    let transition_pending = was_enabled != enabled;
    let view_attention = view
        .overlays
        .values()
        .any(|overlay| overlay.attention || overlay.attention_phase_on || overlay.attention_steady);
    let visibility_hold = (runtime.visibility_hold)()
        || context_open
        || settings_open
        || view
            .keyboard_focus
            .as_ref()
            .is_some_and(|focus| focus.is_focused(window))
        || attention_hold
        || view_attention
        || owned_taskbar_resize_active()
        || transition_pending;
    let prior = *runtime.state.borrow();
    let (next, effect) = reduce_auto_hide(
        prior,
        AutoHideInput {
            enabled,
            now_ms: epoch.elapsed().as_millis() as u64,
            pointer: cursor,
            visibility_hold,
            endpoints,
        },
    );
    let applied = match effect {
        AutoHideEffect::Show => hwnd(window).is_ok_and(|raw| {
            set_owned_taskbar_auto_hide_clip(raw, false).is_ok()
                && move_owned_taskbar_client(
                    raw,
                    endpoints.visible.left,
                    endpoints.visible.top,
                    endpoints.visible.width(),
                    endpoints.visible.height(),
                )
                .is_ok()
        }),
        AutoHideEffect::Hide => {
            hwnd(window).is_ok_and(|raw| set_owned_taskbar_auto_hide_clip(raw, true).is_ok())
        }
        AutoHideEffect::NoChange => true,
    };
    if applied {
        *runtime.state.borrow_mut() = next;
        if effect == AutoHideEffect::Show {
            runtime.fast_hidden.store(false, Ordering::Release);
        } else if effect == AutoHideEffect::Hide {
            runtime.fast_hidden.store(true, Ordering::Release);
        }
        if effect == AutoHideEffect::Show {
            trace_action("taskbar:auto-hide-shown");
        } else if effect == AutoHideEffect::Hide {
            trace_action("taskbar:auto-hide-hidden");
        }
    } else {
        trace_action("taskbar:auto-hide-endpoint-rejected");
    }

    if was_enabled == enabled {
        return;
    }
    let mut transition_ok = !shell;
    if shell {
        if let Ok(raw) = hwnd(window) {
            let mut leases = leases.borrow_mut();
            if let Some(lease) = leases.iter_mut().find(|lease| lease.owns_window(raw)) {
                transition_ok = if enabled {
                    match lease.remove_appbar() {
                        Ok(_) => {
                            trace_action("taskbar:auto-hide-appbar-removed");
                            true
                        }
                        Err(_) => {
                            trace_action("taskbar:auto-hide-appbar-remove-rejected");
                            false
                        }
                    }
                } else {
                    match lease.register_appbar() {
                        Err(_) => {
                            trace_action("taskbar:auto-hide-owned-workarea-restored");
                            true
                        }
                        Ok(())
                            if lease
                                .reserve_bottom(
                                    ScreenRect {
                                        left: runtime.monitor.bounds.left,
                                        top: runtime.monitor.bounds.top,
                                        right: runtime.monitor.bounds.right,
                                        bottom: runtime.monitor.bounds.bottom,
                                    },
                                    taskbar_height,
                                )
                                .is_ok() =>
                        {
                            trace_action("taskbar:auto-hide-appbar-restored");
                            true
                        }
                        Ok(()) => {
                            let _ = lease.remove_appbar();
                            trace_action("taskbar:auto-hide-appbar-restore-rejected");
                            false
                        }
                    }
                };
            } else {
                trace_action("taskbar:auto-hide-lease-missing");
            }
        } else {
            trace_action("taskbar:auto-hide-hwnd-rejected");
        }
    }
    if transition_ok {
        *runtime.enabled.borrow_mut() = enabled;
    }
}

fn taskbar_physical_geometry(
    monitor: &MonitorRecord,
    shell: bool,
    rows: u8,
) -> TaskbarPhysicalGeometry {
    let scale = monitor.dpi_x as f32 / 96.0;
    let height = (WindowsGuiMetrics::taskbar_height(rows) * scale).round() as i32;
    let bottom = if shell {
        monitor.bounds.bottom
    } else {
        monitor.work_area.bottom
    };
    TaskbarPhysicalGeometry {
        left: monitor.bounds.left,
        top: bottom - height,
        width: monitor.bounds.right - monitor.bounds.left,
        height,
        bottom,
    }
}

const fn taskbar_uses_monitor_bounds(shell: bool, explorer_shell_present: bool) -> bool {
    shell || !explorer_shell_present
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StartWindowGeometry {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn start_window_geometry(
    monitor: &MonitorRecord,
    shell: bool,
    taskbar_rows: u8,
    alignment: TaskbarAlignment,
) -> StartWindowGeometry {
    let scale = monitor.dpi_x as f32 / 96.0;
    let monitor_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let width = (monitor_width - WindowsGuiMetrics::START_HORIZONTAL_MARGIN * 2.0)
        .clamp(1.0, WindowsGuiMetrics::START_WIDTH);
    let work_left = monitor.work_area.left as f32 / scale;
    let work_right = monitor.work_area.right as f32 / scale;
    let work_top = monitor.work_area.top as f32 / scale;
    let taskbar_bottom = if shell {
        monitor.bounds.bottom as f32 / scale
    } else {
        monitor.work_area.bottom as f32 / scale
    };
    let start_bottom = (taskbar_bottom
        - WindowsGuiMetrics::taskbar_height(taskbar_rows)
        - WindowsGuiMetrics::START_TASKBAR_GAP)
        .max(work_top + 1.0);
    let height = (start_bottom - work_top).clamp(1.0, WindowsGuiMetrics::START_MAX_HEIGHT);
    let centered_left = work_left + (monitor_width - width).max(0.0) / 2.0;
    let desired_left = match alignment {
        TaskbarAlignment::Left => work_left + WindowsGuiMetrics::START_HORIZONTAL_MARGIN,
        TaskbarAlignment::Center => centered_left,
    };
    let maximum_left = (work_right - width).max(work_left);
    StartWindowGeometry {
        left: desired_left.clamp(work_left, maximum_left),
        top: (start_bottom - height).max(work_top),
        width,
        height,
    }
}

fn start_options(
    monitor: &MonitorRecord,
    shell: bool,
    taskbar_rows: u8,
    alignment: TaskbarAlignment,
) -> WindowOptions {
    let geometry = start_window_geometry(monitor, shell, taskbar_rows, alignment);
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreviewOpenSource {
    Hover,
    Click,
}

impl PreviewOpenSource {
    const fn activates_window(self) -> bool {
        matches!(self, Self::Click)
    }

    const fn assigns_keyboard_focus(self) -> bool {
        matches!(self, Self::Click)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TaskFlyoutGeometry {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn task_flyout_geometry(
    monitor: &MonitorRecord,
    card_widths: &[u16],
    anchor_physical_x: Option<i32>,
    owned_shell: bool,
    taskbar_rows: u8,
) -> TaskFlyoutGeometry {
    const WINDOW_SHADOW_INSET: f32 = 8.0;
    let scale = monitor.dpi_x.max(96) as f32 / 96.0;
    let work_left = monitor.bounds.left as f32 / scale;
    let work_right = monitor.bounds.right as f32 / scale;
    let work_top = monitor.bounds.top as f32 / scale;
    let taskbar_bottom = if owned_shell {
        monitor.bounds.bottom
    } else {
        monitor.work_area.bottom
    } as f32
        / scale;
    let taskbar_top = taskbar_bottom - WindowsGuiMetrics::taskbar_height(taskbar_rows);
    let popup_bottom = (taskbar_top - WindowsGuiMetrics::POPUP_GAP).max(work_top + 1.0);
    let available_width = (work_right - work_left).max(1.0);
    let available_height = (popup_bottom - work_top).max(1.0);
    let inset_x = WINDOW_SHADOW_INSET.min(((available_width - 1.0) / 2.0).max(0.0));
    let inset_y = WINDOW_SHADOW_INSET.min(((available_height - 1.0) / 2.0).max(0.0));
    let card_count = card_widths.len().clamp(1, 4);
    let requested_width = card_widths
        .iter()
        .take(4)
        .map(|width| f32::from(*width))
        .sum::<f32>()
        + 16.0
        + 8.0 * card_count.saturating_sub(1) as f32;
    let width = requested_width.min((available_width - inset_x * 2.0).max(1.0));
    let height = 260.0_f32.min((available_height - inset_y).max(1.0));
    let fallback_anchor = work_left + available_width / 2.0;
    let anchor = anchor_physical_x
        .map(|x| x as f32 / scale)
        .unwrap_or(fallback_anchor);
    let left = (anchor - width / 2.0).clamp(
        work_left + inset_x,
        (work_right - inset_x - width).max(work_left + inset_x),
    );
    TaskFlyoutGeometry {
        left,
        top: (popup_bottom - height).max(work_top),
        width,
        height,
    }
}

fn task_flyout_options(
    monitor: &MonitorRecord,
    cards: &[PreviewCard],
    anchor_physical_x: Option<i32>,
    owned_shell: bool,
    taskbar_rows: u8,
    source: PreviewOpenSource,
) -> WindowOptions {
    let card_widths = cards
        .iter()
        .map(|card| card.preview_width)
        .collect::<Vec<_>>();
    let geometry = task_flyout_geometry(
        monitor,
        &card_widths,
        anchor_physical_x,
        owned_shell,
        taskbar_rows,
    );
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(geometry.left), px(geometry.top)),
            size: size(px(geometry.width), px(geometry.height)),
        })),
        titlebar: None,
        focus: source.assigns_keyboard_focus(),
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
struct SystemFlyoutGeometry {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn system_flyout_geometry(
    monitor: &MonitorRecord,
    shell: bool,
    kind: SystemFlyoutKind,
    input_profile_count: usize,
    notification_count: usize,
    taskbar_rows: u8,
) -> SystemFlyoutGeometry {
    let scale = monitor.dpi_x as f32 / 96.0;
    let work_left = monitor.work_area.left as f32 / scale;
    let work_top = monitor.work_area.top as f32 / scale;
    let work_right = monitor.work_area.right as f32 / scale;
    let work_bottom = monitor.work_area.bottom as f32 / scale;
    let (preferred_width, preferred_height): (f32, f32) = match kind {
        SystemFlyoutKind::Input => (
            WindowsGuiMetrics::SYSTEM_FLYOUT_WIDTH,
            112.0 + input_profile_count.clamp(1, 6) as f32 * 56.0,
        ),
        SystemFlyoutKind::Volume => (WindowsGuiMetrics::SYSTEM_FLYOUT_WIDTH, 184.0),
        SystemFlyoutKind::NetworkPower => (WindowsGuiMetrics::SYSTEM_FLYOUT_WIDTH, 640.0),
        SystemFlyoutKind::Calendar => (
            WindowsGuiMetrics::CALENDAR_FLYOUT_WIDTH,
            if notification_count == 0 {
                520.0
            } else {
                720.0
            },
        ),
    };
    let gap = WindowsGuiMetrics::POPUP_GAP;
    let taskbar_height = WindowsGuiMetrics::taskbar_height(taskbar_rows);
    let usable_width = (work_right - work_left - gap * 2.0).max(1.0);
    let taskbar_bottom = if shell {
        monitor.bounds.bottom as f32 / scale
    } else {
        work_bottom
    };
    let popup_bottom = (taskbar_bottom - taskbar_height - gap).max(work_top + 1.0);
    let usable_height = (popup_bottom - work_top).max(1.0);
    let width = preferred_width.min(usable_width);
    let height = preferred_height.min(usable_height);
    SystemFlyoutGeometry {
        left: (work_right - gap - width).max(work_left),
        top: (popup_bottom - height).max(work_top),
        width,
        height,
    }
}

fn system_flyout_options(
    monitor: &MonitorRecord,
    shell: bool,
    kind: SystemFlyoutKind,
    input_profile_count: usize,
    notification_count: usize,
    taskbar_rows: u8,
) -> WindowOptions {
    let geometry = system_flyout_geometry(
        monitor,
        shell,
        kind,
        input_profile_count,
        notification_count,
        taskbar_rows,
    );
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct NotificationOverflowBounds {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn notification_overflow_bounds(
    monitor: &MonitorRecord,
    shell: bool,
    icon_count: usize,
    taskbar_rows: u8,
) -> NotificationOverflowBounds {
    let scale = monitor.dpi_x as f32 / 96.0;
    let logical_width = WindowsGuiMetrics::NOTIFICATION_OVERFLOW_WIDTH;
    let rows = WindowsGuiMetrics::overflow_rows(icon_count).min(6) as f32;
    let logical_height = WindowsGuiMetrics::NOTIFICATION_OVERFLOW_PADDING * 2.0
        + rows * WindowsGuiMetrics::NOTIFICATION_OVERFLOW_CELL;
    let work_left = monitor.work_area.left as f32 / scale;
    let work_top = monitor.work_area.top as f32 / scale;
    let work_right = monitor.work_area.right as f32 / scale;
    let work_bottom = monitor.work_area.bottom as f32 / scale;
    let taskbar_height = WindowsGuiMetrics::taskbar_height(taskbar_rows);
    let taskbar_bottom = if shell {
        monitor.bounds.bottom as f32 / scale
    } else {
        work_bottom
    };
    let bottom = taskbar_bottom - taskbar_height - WindowsGuiMetrics::POPUP_GAP;
    let width = logical_width.min(work_right - work_left);
    let height = logical_height.min((bottom - work_top).max(1.0));
    let right = work_right - WindowsGuiMetrics::POPUP_EDGE_MARGIN;
    NotificationOverflowBounds {
        left: (right - width).max(work_left),
        top: (bottom - height).max(work_top),
        width,
        height,
    }
}

fn notification_overflow_options(
    monitor: &MonitorRecord,
    shell: bool,
    icon_count: usize,
    taskbar_rows: u8,
) -> WindowOptions {
    let bounds = notification_overflow_bounds(monitor, shell, icon_count, taskbar_rows);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(bounds.left), px(bounds.top)),
            size: size(px(bounds.width), px(bounds.height)),
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct JumpListGeometry {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn jump_list_geometry(
    monitor: &MonitorRecord,
    shell: bool,
    taskbar_rows: u8,
    anchor_physical_x: Option<i32>,
    entry_count: usize,
    group_count: usize,
) -> JumpListGeometry {
    let scale = monitor.dpi_x as f32 / 96.0;
    let work_left = monitor.work_area.left as f32 / scale;
    let work_right = monitor.work_area.right as f32 / scale;
    let work_top = monitor.work_area.top as f32 / scale;
    let taskbar_bottom = if shell {
        monitor.bounds.bottom as f32 / scale
    } else {
        monitor.work_area.bottom as f32 / scale
    };
    let popup_bottom = taskbar_bottom
        - WindowsGuiMetrics::taskbar_height(taskbar_rows)
        - WindowsGuiMetrics::POPUP_GAP;
    let width = WindowsGuiMetrics::PREVIEW_WIDTH
        .min((work_right - work_left - WindowsGuiMetrics::POPUP_EDGE_MARGIN * 2.0).max(1.0));
    let entries = entry_count.max(1) as f32;
    let gaps = entry_count.saturating_sub(1) as f32 * 2.0;
    let separators = group_count.saturating_sub(1) as f32;
    let headings = group_count.max(1) as f32 * 24.0;
    let preferred_height = (8.0 + headings + entries * 32.0 + gaps + separators).min(480.0);
    let height = preferred_height.min((popup_bottom - work_top).max(1.0));
    let fallback_anchor = work_left + (work_right - work_left) / 2.0;
    let anchor = anchor_physical_x
        .map(|x| x as f32 / scale)
        .unwrap_or(fallback_anchor);
    let left = (anchor - width / 2.0).clamp(
        work_left + WindowsGuiMetrics::POPUP_EDGE_MARGIN,
        (work_right - WindowsGuiMetrics::POPUP_EDGE_MARGIN - width)
            .max(work_left + WindowsGuiMetrics::POPUP_EDGE_MARGIN),
    );
    JumpListGeometry {
        left,
        top: (popup_bottom - height).max(work_top),
        width,
        height,
    }
}

fn jump_list_options(
    monitor: &MonitorRecord,
    shell: bool,
    taskbar_rows: u8,
    anchor_physical_x: Option<i32>,
    entry_count: usize,
    group_count: usize,
) -> WindowOptions {
    let geometry = jump_list_geometry(
        monitor,
        shell,
        taskbar_rows,
        anchor_physical_x,
        entry_count,
        group_count,
    );
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

fn taskbar_context_options(
    monitor: &MonitorRecord,
    shell: bool,
    rows: u8,
    anchor: gpui::Point<gpui::Pixels>,
) -> WindowOptions {
    let (left, top, _, _) = taskbar_context_placement(monitor, shell, rows, anchor);
    let width = WindowsGuiMetrics::TASKBAR_CONTEXT_WIDTH;
    let height = 244.0;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(left), px(top)),
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

fn system_control_context_options(
    monitor: &MonitorRecord,
    shell: bool,
    rows: u8,
    anchor: gpui::Point<gpui::Pixels>,
    kind: SystemControlContextKind,
) -> WindowOptions {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = WindowsGuiMetrics::SYSTEM_CONTEXT_WIDTH;
    let height = match kind {
        SystemControlContextKind::Input => {
            WindowsGuiMetrics::CONTEXT_PADDING * 2.0 + WindowsGuiMetrics::CONTEXT_ROW_HEIGHT
        }
        SystemControlContextKind::Volume => {
            WindowsGuiMetrics::CONTEXT_PADDING * 2.0 + WindowsGuiMetrics::CONTEXT_ROW_HEIGHT * 2.0
        }
    };
    let monitor_left = monitor.bounds.left as f32 / scale;
    let monitor_right = monitor.bounds.right as f32 / scale;
    let monitor_top = monitor.bounds.top as f32 / scale;
    let taskbar_bottom = if shell {
        monitor.bounds.bottom
    } else {
        monitor.work_area.bottom
    } as f32
        / scale;
    let taskbar_top = taskbar_bottom - WindowsGuiMetrics::taskbar_height(rows);
    let left = (monitor_left + anchor.x.as_f32() - width / 2.0)
        .clamp(monitor_left, (monitor_right - width).max(monitor_left));
    let top = (taskbar_top - height - WindowsGuiMetrics::POPUP_GAP).max(monitor_top);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(left), px(top)),
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

fn taskbar_context_placement(
    monitor: &MonitorRecord,
    shell: bool,
    rows: u8,
    anchor: gpui::Point<gpui::Pixels>,
) -> (f32, f32, f32, f32) {
    let scale = monitor.dpi_x as f32 / 96.0;
    let width = WindowsGuiMetrics::TASKBAR_CONTEXT_WIDTH;
    let height = 244.0;
    let monitor_left = monitor.bounds.left as f32 / scale;
    let monitor_right = monitor.bounds.right as f32 / scale;
    let monitor_top = monitor.bounds.top as f32 / scale;
    let taskbar_bottom = if shell {
        monitor.bounds.bottom
    } else {
        monitor.work_area.bottom
    } as f32
        / scale;
    let taskbar_top = taskbar_bottom - WindowsGuiMetrics::taskbar_height(rows);
    let left = (monitor_left + anchor.x.as_f32() - width / 2.0)
        .clamp(monitor_left, (monitor_right - width).max(monitor_left));
    let top = (taskbar_top - height - WindowsGuiMetrics::POPUP_GAP).max(monitor_top);
    (left, top, width, height)
}

fn taskbar_settings_options(monitor: &MonitorRecord) -> WindowOptions {
    let (left, top, width, height) = taskbar_settings_placement(monitor);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(left), px(top)),
            size: size(px(width), px(height)),
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

fn taskbar_settings_placement(monitor: &MonitorRecord) -> (f32, f32, f32, f32) {
    let scale = (monitor.dpi_x.max(96)) as f32 / 96.0;
    let available_width = (monitor.work_area.right - monitor.work_area.left) as f32 / scale;
    let available_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / scale;
    let bounded = |available: f32, minimum: f32, preferred: f32| {
        if available >= minimum {
            available.min(preferred).max(minimum)
        } else {
            available.max(1.0)
        }
    };
    let width = bounded(available_width, 640.0, 1100.0);
    let height = bounded(available_height, 480.0, 860.0);
    let left = monitor.work_area.left as f32 / scale + (available_width - width) / 2.0;
    let top = monitor.work_area.top as f32 / scale + (available_height - height) / 2.0;
    (left, top, width, height)
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
            second: local.second,
        },
        taskbar_clock_locale(),
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

fn taskbar_clock_locale() -> ClockLocale {
    let locale = std::env::var("SUPERDESKTOP_LOCALE")
        .ok()
        .or_else(platform_win::common::taskbar_status::user_locale_name);
    clock_locale_from_tag(locale.as_deref())
}

fn clock_locale_from_tag(locale: Option<&str>) -> ClockLocale {
    if locale.is_some_and(|locale| locale.to_ascii_lowercase().starts_with("zh")) {
        ClockLocale::ZhTw
    } else {
        ClockLocale::En
    }
}

fn system_status_command(action: SystemStatusAction) -> SystemStatusCommand {
    match action {
        SystemStatusAction::ActivateInputProfile(profile_id) => {
            SystemStatusCommand::ActivateInputProfile { profile_id }
        }
        SystemStatusAction::OpenLanguagePreferences => SystemStatusCommand::OpenLanguagePreferences,
        SystemStatusAction::SetVolume(volume_percent) => {
            SystemStatusCommand::SetVolume { volume_percent }
        }
        SystemStatusAction::SetMute(muted) => SystemStatusCommand::SetMute { muted },
        SystemStatusAction::RefreshWifi => SystemStatusCommand::RefreshWifi,
        SystemStatusAction::ConnectWifi {
            interface_id,
            profile_name,
        } => SystemStatusCommand::ConnectWifi {
            interface_id,
            profile_name,
        },
        SystemStatusAction::DisconnectWifi { interface_id } => {
            SystemStatusCommand::DisconnectWifi { interface_id }
        }
    }
}

fn adjacent_input_profile_id(snapshot: &SystemStatusSnapshot, direction: i32) -> Option<String> {
    let StatusAvailability::Available(input) = &snapshot.input else {
        return None;
    };
    if input.profiles.len() < 2 {
        return None;
    }
    let active = input
        .profiles
        .iter()
        .position(|profile| profile.id == input.active_profile_id)
        .unwrap_or(0);
    let count = input.profiles.len() as i32;
    let target = (active as i32 + direction).rem_euclid(count) as usize;
    input.profiles.get(target).map(|profile| profile.id.clone())
}

const SYSTEM_STATUS_COMMAND_MAX_ATTEMPTS: usize = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
struct SystemStatusCommandExecution {
    response: SystemStatusHostResponse,
    recovered_stale_generation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SystemStatusCommandFailure {
    ProviderUnavailable,
    Transport(&'static str),
    InvalidResponse,
    ResyncTransport(&'static str),
    ResyncInvalidResponse,
    ResyncDidNotAdvance,
}

impl SystemStatusCommandFailure {
    fn message(&self) -> String {
        match self {
            Self::ProviderUnavailable => "system status provider is unavailable".into(),
            Self::Transport(reason) => format!("status command transport failed: {reason}"),
            Self::InvalidResponse => "status command returned an invalid response".into(),
            Self::ResyncTransport(reason) => {
                format!("status generation resynchronization failed: {reason}")
            }
            Self::ResyncInvalidResponse => {
                "status generation resynchronization returned an invalid response".into()
            }
            Self::ResyncDidNotAdvance => {
                "status generation resynchronization did not advance the host generation".into()
            }
        }
    }

    const fn provider_failed(&self) -> bool {
        matches!(self, Self::Transport(_) | Self::ResyncTransport(_))
    }
}

fn execute_system_status_command<R, N>(
    command: SystemStatusCommand,
    command_timeout: Duration,
    reconciler: &mut StatusReconciler,
    mut request: R,
    mut now_ms: N,
) -> Result<SystemStatusCommandExecution, SystemStatusCommandFailure>
where
    R: FnMut(&SystemStatusHostRequest, Duration) -> Result<SystemStatusHostResponse, &'static str>,
    N: FnMut() -> u64,
{
    let mut recovered_stale_generation = false;
    let mut command_host_generation = None;
    for attempt in 0..SYSTEM_STATUS_COMMAND_MAX_ATTEMPTS {
        let Some(expected_host_generation) = command_host_generation.or_else(|| {
            reconciler
                .snapshot()
                .map(|snapshot| snapshot.host_generation)
        }) else {
            return Err(SystemStatusCommandFailure::ProviderUnavailable);
        };
        let correlation_id = format!(
            "system-status-{}",
            NEXT_PROVIDER_REQUEST.fetch_add(1, Ordering::Relaxed)
        );
        let command_request = SystemStatusHostRequest::Command {
            request: SystemStatusCommandRequest {
                correlation_id,
                expected_host_generation,
                deadline_unix_ms: now_ms().saturating_add(command_timeout.as_millis() as u64),
                command: command.clone(),
            },
        };
        let response = request(
            &command_request,
            command_timeout + Duration::from_millis(250),
        )
        .map_err(SystemStatusCommandFailure::Transport)?;
        let SystemStatusHostResponse::Terminal(terminal) = &response else {
            return Err(SystemStatusCommandFailure::InvalidResponse);
        };
        if terminal.terminal == SystemStatusTerminalKind::StaleGeneration
            && attempt + 1 < SYSTEM_STATUS_COMMAND_MAX_ATTEMPTS
        {
            trace_action("status:command-stale-generation");
            let resync = request(
                &SystemStatusHostRequest::Snapshot,
                Duration::from_millis(500),
            )
            .map_err(SystemStatusCommandFailure::ResyncTransport)?;
            let SystemStatusHostResponse::Snapshot(snapshot) = &resync else {
                return Err(SystemStatusCommandFailure::ResyncInvalidResponse);
            };
            if snapshot.host_generation == expected_host_generation {
                return Err(SystemStatusCommandFailure::ResyncDidNotAdvance);
            }
            command_host_generation = Some(snapshot.host_generation);
            reconciler.apply(resync);
            recovered_stale_generation = true;
            trace_action("status:command-generation-resynchronized");
            trace_action("status:command-retrying");
            continue;
        }

        reconciler.apply(response.clone());
        match request(
            &SystemStatusHostRequest::Snapshot,
            Duration::from_millis(500),
        ) {
            Ok(snapshot @ SystemStatusHostResponse::Snapshot(_)) => {
                reconciler.apply(snapshot);
                trace_action("status:command-final-snapshot");
            }
            Ok(_) => trace_action("status:command-final-snapshot-invalid"),
            Err(_) => trace_action("status:command-final-snapshot-failed"),
        }
        return Ok(SystemStatusCommandExecution {
            response,
            recovered_stale_generation,
        });
    }
    unreachable!("the bounded command loop always returns on its final attempt")
}

fn apply_system_status_action(
    action: SystemStatusAction,
    app: &mut App,
    client: &Rc<RefCell<SystemStatusClient>>,
    reconciler: &Rc<RefCell<StatusReconciler>>,
    start_window: &Rc<RefCell<Option<gpui::WindowHandle<StartView>>>>,
) {
    if let SystemStatusAction::ActivateInputProfile(profile_id) = &action {
        match platform_win::common::system_status::request_input_profile(
            profile_id,
            Duration::from_secs(5),
        ) {
            Ok(_) => trace_action("status:input-profile-observed"),
            Err(error) => report_error("status:input-profile", error),
        }
        if let Some(start) = *start_window.borrow() {
            let _ = start.update(app, |_, window, cx| {
                window.activate_window();
                cx.notify();
            });
            trace_action("start:ime-focus-restored");
        }
        return;
    }
    let restore_start_focus = matches!(&action, SystemStatusAction::ActivateInputProfile(_));
    let command_timeout = if restore_start_focus {
        Duration::from_millis(5_000)
    } else {
        Duration::from_millis(1_000)
    };
    let command = system_status_command(action);
    let execution = {
        let mut client = client.borrow_mut();
        let mut reconciler = reconciler.borrow_mut();
        execute_system_status_command(
            command,
            command_timeout,
            &mut reconciler,
            |request, timeout| client.request(request, timeout),
            unix_time_ms,
        )
    };
    match execution {
        Ok(execution) => {
            if let SystemStatusHostResponse::Terminal(terminal) = &execution.response
                && !matches!(
                    terminal.terminal,
                    SystemStatusTerminalKind::Observed | SystemStatusTerminalKind::Accepted
                )
            {
                report_error(
                    "status:command",
                    format!("{:?}: {}", terminal.terminal, terminal.message),
                );
            }
            if execution.recovered_stale_generation {
                trace_action("status:command-generation-recovered");
            }
            trace_action("status:command-terminal");
        }
        Err(SystemStatusCommandFailure::ProviderUnavailable) => {
            trace_action("status:command-provider-unavailable");
        }
        Err(failure) => {
            report_error("status:command", failure.message());
            if failure.provider_failed() {
                reconciler.borrow_mut().provider_unavailable();
                trace_action("status:command-provider-failed");
            } else {
                trace_action("status:command-rejected");
            }
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

fn apply_system_status_action_or_queue_volume(
    action: SystemStatusAction,
    app: &mut App,
    client: &Rc<RefCell<SystemStatusClient>>,
    reconciler: &Rc<RefCell<StatusReconciler>>,
    start_window: &Rc<RefCell<Option<gpui::WindowHandle<StartView>>>>,
    volume: &VolumeCommandCoordinator,
) {
    if let SystemStatusAction::SetVolume(volume_percent) = action {
        volume.submit(volume_percent);
        trace_action("status:volume-coalesced");
    } else {
        apply_system_status_action(action, app, client, reconciler, start_window);
    }
}

fn fixed_label() -> &'static str {
    match std::env::var("SUPERDESKTOP_LOCALE").as_deref() {
        Ok("zh-CN") => "超级资源管理器",
        Ok("zh-TW") => "超級檔案總管",
        _ => "SuperExplorer",
    }
}

fn system_flyout_presentation() -> SystemFlyoutPresentation {
    let theme = match std::env::var("SUPERDESKTOP_THEME").as_deref() {
        Ok("dark") => SystemFlyoutTheme::Dark,
        Ok("high-contrast") => SystemFlyoutTheme::HighContrast,
        _ => SystemFlyoutTheme::Light,
    };
    let traditional_chinese = std::env::var("SUPERDESKTOP_LOCALE")
        .ok()
        .or_else(platform_win::common::taskbar_status::user_locale_name)
        .is_some_and(|locale| locale.eq_ignore_ascii_case("zh-TW"));
    SystemFlyoutPresentation::new(theme, traditional_chinese)
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
    let shell_hotkeys = Rc::new(if shell {
        match platform_win::common::shell_hotkey::ShellHotkeys::start() {
            Ok(hotkey) => {
                trace_action("win-e:hook-active");
                Some(hotkey)
            }
            Err(error) => {
                report_error("win-e:hook", error);
                None
            }
        }
    } else {
        None
    });
    let snapshot = snapshot_real_monitors()?;
    let (mut settings_store, settings_target) =
        platform_win::common::settings_file::production_settings_store()
            .map_err(|_| "settings-store-init")?;
    let mut persisted_settings = settings_store
        .load(&settings_target)
        .map_err(|_| "settings-store-load")?
        .settings;
    let verification_surface = std::env::var("SUPERDESKTOP_VERIFICATION_SURFACE").ok();
    if verification_surface.is_some()
        && let Ok(rows) = std::env::var("SUPERDESKTOP_VERIFICATION_TASKBAR_ROWS")
        && let Ok(rows) = rows.parse::<u8>()
    {
        persisted_settings.taskbar.rows = rows.clamp(1, 3);
    }
    let state_matrix = std::env::var_os("SUPERDESKTOP_VERIFICATION_STATE_MATRIX").is_some();
    let task_icon_edge = Rc::new(Cell::new(task_icon_source_edge(&snapshot.monitors)));
    trace_action(&format!(
        "taskbar:icon-source-edge:{}",
        task_icon_edge.get()
    ));
    let task_icon_cache = Rc::new(RefCell::new(BTreeMap::new()));
    let minimized_window_shelf = Rc::new(RefCell::new(MinimizedWindowShelf::default()));
    let initial_tasks = if state_matrix {
        verification_state_tasks()
    } else {
        visible_tasks(
            &persisted_settings.taskbar.pins,
            persisted_settings.taskbar.combine_groups,
            task_icon_edge.get(),
            &mut task_icon_cache.borrow_mut(),
            &BTreeSet::new(),
            shell.then_some(&minimized_window_shelf),
        )?
    };
    let wallpaper = std::env::var_os("SUPERDESKTOP_WALLPAPER_PATH")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| current_wallpaper_path().ok());
    let auto_hide_verification = verification_surface.as_deref() == Some("taskbar-auto-hide");
    let interactive = verification_surface.is_some() && !auto_hide_verification;
    let desktop_namespace = Rc::new(RefCell::new(DesktopNamespaceRuntime::default()));
    let desktop_operations = Rc::new(RefCell::new(DesktopOperationController::default()));
    let desktop_transfers = ProductionTransferRuntime::default();
    let provider_client = Rc::new(RefCell::new(ProviderClient::adjacent()?));
    let verification_notifyicon_compatibility = verification_surface.as_deref() == Some("taskbar")
        && std::env::var("SUPERDESKTOP_VERIFICATION_NOTIFYICON_COMPAT")
            .is_ok_and(|value| value == "1");
    let mut initial_notification_client =
        NotificationClient::adjacent(shell || verification_notifyicon_compatibility)?;
    if shell
        && !matches!(
            initial_notification_client.request(
                &NotificationMutation::Health,
                Duration::from_millis(750),
            ),
            Ok(NotificationHostResponse::Health(health)) if health.healthy
        )
    {
        return Err("notification-compatibility-handshake");
    }
    let initial_notification_snapshot = match initial_notification_client
        .request(&NotificationMutation::Snapshot, Duration::from_millis(750))
    {
        Ok(NotificationHostResponse::Snapshot(snapshot)) => Some(snapshot),
        _ => None,
    };
    let notification_client = Rc::new(RefCell::new(initial_notification_client));
    let notification_snapshot = Rc::new(RefCell::new(initial_notification_snapshot));
    let status_client = Rc::new(RefCell::new(SystemStatusClient::adjacent()?));
    let volume_commands = VolumeCommandCoordinator::start()?;
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
            let alt_tab_window = Rc::new(RefCell::new(None::<gpui::WindowHandle<AltTabView>>));
            let alt_tab_monitor = snapshot
                .monitors
                .iter()
                .find(|monitor| monitor.primary)
                .or_else(|| snapshot.monitors.first())
                .cloned();
            let provider_refresh_batch = Arc::new(Mutex::new(ProviderRefreshBatch::default()));
            let provider_refresh_stop = Arc::new(AtomicBool::new(false));
            let provider_batch_for_worker = Arc::clone(&provider_refresh_batch);
            let provider_stop_for_worker = Arc::clone(&provider_refresh_stop);
            let provider_refresh_worker = Rc::new(RefCell::new(
                std::thread::Builder::new()
                    .name("superdesktop-provider-refresh".into())
                    .spawn(move || {
                    let mut status = SystemStatusClient::adjacent().ok();
                    let mut taskbar = TaskbarStateClient::adjacent(shell).ok();
                    if let Some(taskbar) = taskbar.as_mut() {
                        let _ = taskbar.ensure_started();
                    }
                    let mut tick = 0u8;
                    while !provider_stop_for_worker.load(Ordering::Acquire) {
                        tick = tick.wrapping_add(1);
                        let status_result = tick.is_multiple_of(10).then(|| {
                            status
                                .as_mut()
                                .ok_or_else(|| "status-refresh-unavailable".into())
                                .and_then(|client| {
                                    client
                                        .request(
                                            &SystemStatusHostRequest::Snapshot,
                                            Duration::from_millis(250),
                                        )
                                        .map_err(ToOwned::to_owned)
                                })
                        });
                        let taskbar_result = tick.is_multiple_of(2).then(|| {
                            taskbar
                                .as_mut()
                                .ok_or_else(|| "taskbar-refresh-unavailable".into())
                                .and_then(|client| {
                                    client
                                        .request_snapshot(Duration::from_millis(100))
                                        .map_err(ToOwned::to_owned)
                                })
                        });
                        if let Ok(mut batch) = provider_batch_for_worker.lock() {
                            if let Some(result) = status_result {
                                batch.status = Some(result);
                            }
                            if let Some(result) = taskbar_result {
                                batch.taskbar = Some(result);
                            }
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    })
                    .ok(),
            ));
            let mut desktop_handles = Vec::new();
            let mut taskbar_handles = Vec::new();
            let mut taskbar_monitor_names = Vec::new();
            let mut shell_show_desktop_sessions = Vec::new();
            let mut taskbar_auto_hide = Vec::<TaskbarAutoHideRuntime>::new();
            let mut system_flyout_windows = Vec::new();
            let leases = Rc::new(RefCell::new(Vec::<ControlledShellCapability>::new()));
            let attention_runtime = Rc::new(RefCell::new(AttentionRuntime::default()));
            let taskbar_context_window =
                Rc::new(RefCell::new(None::<gpui::WindowHandle<TaskbarContextView>>));
            let taskbar_settings_window =
                Rc::new(RefCell::new(None::<gpui::WindowHandle<TaskbarSettingsView>>));
            let init_error = Rc::new(RefCell::new(None::<&'static str>));
            for monitor in snapshot.monitors.clone() {
                if !matches!(
                    verification_surface.as_deref(),
                    Some("taskbar" | "taskbar-auto-hide")
                ) {
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
                    let desktop = cx.open_window(options(&monitor, false, interactive, shell, 2), move |window, cx| {
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
                let notification_overflow_settings = Rc::clone(&persisted_settings);
                let taskbar_tasks = initial_tasks.clone();
                let taskbar_error = Rc::clone(&init_error);
                let taskbar_leases = Rc::clone(&leases);
                let taskbar_resize_leases = Rc::clone(&leases);
                let taskbar_resize_store = Rc::clone(&settings_store);
                let taskbar_resize_target = Rc::clone(&settings_target);
                let taskbar_resize_settings = Rc::clone(&persisted_settings);
                let taskbar_resize_monitor = taskbar_monitor.clone();
                let taskbar_resize_settle = Rc::new(RefCell::new(None::<Instant>));
                let auto_hide_resize_settle = Rc::clone(&taskbar_resize_settle);
                let auto_hide_state = Rc::new(RefCell::new(AutoHideState::Visible));
                let auto_hide_enabled =
                    Rc::new(RefCell::new(persisted_settings.borrow().taskbar.auto_hide));
                let auto_hide_fast_hidden = Arc::new(AtomicBool::new(false));
                let auto_hide_fast_enabled = Arc::new(AtomicBool::new(
                    persisted_settings.borrow().taskbar.auto_hide,
                ));
                let auto_hide_fast_rows = Arc::new(AtomicU64::new(u64::from(
                    persisted_settings.borrow().taskbar.rows,
                )));
                let auto_hide_worker_stop = Arc::new(AtomicBool::new(false));
                let auto_hide_worker_handle =
                    Rc::new(RefCell::new(None::<std::thread::JoinHandle<()>>));
                let start_window = Rc::new(RefCell::new(None::<gpui::WindowHandle<StartView>>));
                let start_window_for_taskbar = Rc::clone(&start_window);
                let start_window_for_status = Rc::clone(&start_window);
                let show_desktop_session = Rc::new(RefCell::new(ShowDesktopSession::default()));
                shell_show_desktop_sessions.push(Rc::clone(&show_desktop_session));
                let show_desktop_session_for_taskbar = Rc::clone(&show_desktop_session);
                let show_desktop_session_for_context = Rc::clone(&show_desktop_session);
                let minimized_shelf_for_taskbar = Rc::clone(&minimized_window_shelf);
                let minimized_shelf_for_task = Rc::clone(&minimized_window_shelf);
                let minimized_shelf_for_context = Rc::clone(&minimized_window_shelf);
                let minimized_shelf_for_jump_list = Rc::clone(&minimized_window_shelf);
                let start_provider_for_taskbar = Rc::clone(&provider_client);
                let start_settings_store = Rc::clone(&settings_store);
                let start_settings_target = Rc::clone(&settings_target);
                let start_persisted_settings = Rc::clone(&persisted_settings);
                let start_monitor = taskbar_monitor.clone();
                let flyout_window =
                    Rc::new(RefCell::new(None::<gpui::WindowHandle<TaskFlyoutView>>));
                let flyout_window_for_taskbar = Rc::clone(&flyout_window);
                let flyout_window_for_hover = Rc::clone(&flyout_window);
                let flyout_monitor = taskbar_monitor.clone();
                let flyout_hover_monitor = taskbar_monitor.clone();
                let hover_preview_controller =
                    Rc::new(RefCell::new(HoverPreviewController::default()));
                let hover_preview_for_click = Rc::clone(&hover_preview_controller);
                let hover_preview_for_task = Rc::clone(&hover_preview_controller);
                let taskbar_hwnd_identity = Rc::new(Cell::new(0_isize));
                let active_preview_hwnd = Rc::new(Cell::new(0_isize));
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
                let notification_client_for_center = Rc::clone(&notification_client);
                let notification_snapshot_for_center = Rc::clone(&notification_snapshot);
                let notification_overflow_window = Rc::new(RefCell::new(None::<
                    gpui::WindowHandle<NotificationOverflowView>,
                >));
                let notification_overflow_window_for_taskbar =
                    Rc::clone(&notification_overflow_window);
                let notification_overflow_monitor = taskbar_monitor.clone();
                let status_for_taskbar = Rc::clone(&status_reconciler);
                let status_client_for_taskbar = Rc::clone(&status_client);
                let status_commands_for_taskbar = Rc::clone(&status_reconciler);
                let volume_commands_for_taskbar = volume_commands.clone();
                let system_flyout_window = Rc::new(RefCell::new(None::<(
                    SystemFlyoutKind,
                    gpui::WindowHandle<SystemFlyoutView>,
                )>));
                system_flyout_windows.push(Rc::clone(&system_flyout_window));
                let system_flyout_window_for_taskbar = Rc::clone(&system_flyout_window);
                let system_flyout_window_for_context = Rc::clone(&system_flyout_window);
                let system_flyout_monitor = taskbar_monitor.clone();
                let system_flyout_settings = Rc::clone(&persisted_settings);
                let system_flyout_status = Rc::clone(&status_reconciler);
                let system_flyout_client = Rc::clone(&status_client);
                let system_flyout_volume = volume_commands.clone();
                let system_flyout_start = Rc::clone(&start_window);
                let system_context_window = Rc::new(RefCell::new(None::<(
                    SystemControlContextKind,
                    u64,
                    gpui::WindowHandle<SystemControlContextView>,
                )>));
                let system_context_generation = Rc::new(Cell::new(0_u64));
                let system_context_window_for_taskbar = Rc::clone(&system_context_window);
                let system_context_window_for_flyout = Rc::clone(&system_context_window);
                let system_context_generation_for_taskbar = Rc::clone(&system_context_generation);
                let system_context_monitor = taskbar_monitor.clone();
                let system_context_settings = Rc::clone(&persisted_settings);
                let system_context_status_client = Rc::clone(&status_client);
                let system_context_status = Rc::clone(&status_reconciler);
                let system_context_start = Rc::clone(&start_window);
                let context_window_for_taskbar = Rc::clone(&taskbar_context_window);
                let settings_window_for_context = Rc::clone(&taskbar_settings_window);
                let context_monitor = taskbar_monitor.clone();
                let context_settings_store = Rc::clone(&settings_store);
                let context_settings_target = Rc::clone(&settings_target);
                let context_persisted_settings = Rc::clone(&persisted_settings);
                let production_taskbar_settings = persisted_settings.borrow().taskbar.clone();
                let auto_hide_monitor = taskbar_monitor.clone();
                let close_monitor = taskbar_monitor.clone();
                let close_settings = Rc::clone(&persisted_settings);
                let close_worker_stop = Arc::clone(&auto_hide_worker_stop);
                let close_worker_handle = Rc::clone(&auto_hide_worker_handle);
                let close_provider_stop = Arc::clone(&provider_refresh_stop);
                let close_provider_worker = Rc::clone(&provider_refresh_worker);
                let worker_monitor = taskbar_monitor.clone();
                let worker_hidden = Arc::clone(&auto_hide_fast_hidden);
                let worker_enabled = Arc::clone(&auto_hide_fast_enabled);
                let worker_rows = Arc::clone(&auto_hide_fast_rows);
                let worker_stop = Arc::clone(&auto_hide_worker_stop);
                let worker_handle_slot = Rc::clone(&auto_hide_worker_handle);
                let flyout_window_for_context_seed = Rc::clone(&flyout_window);
                let taskbar_monitor_name = taskbar_monitor.device_name.clone();
                let explorer_shell_present_at_open = match trusted_explorer_shell_present() {
                    Ok(present) => present,
                    Err(error) => {
                        report_error("taskbar:explorer-presence", error);
                        true
                    }
                };
                let taskbar_bounds_mode =
                    taskbar_uses_monitor_bounds(shell, explorer_shell_present_at_open);
                // Once Explorer is absent, every owned surface and worker must use the
                // same physical monitor anchor as the taskbar itself. Mixing the raw
                // preview flag with the effective taskbar mode creates a full native-
                // taskbar-height gap and breaks auto-hide at the physical screen edge.
                let shell = taskbar_bounds_mode;
                let taskbar = cx.open_window(
                    options(
                        &monitor,
                        true,
                        interactive,
                        taskbar_bounds_mode,
                        production_taskbar_settings.rows,
                    ),
                    move |window, cx| {
                    if let Ok(raw) = hwnd(window) {
                        taskbar_hwnd_identity.set(raw);
                    }
                    if interactive {
                        window.activate_window();
                    }
                    let geometry = taskbar_physical_geometry(
                        &taskbar_monitor,
                        taskbar_bounds_mode,
                        production_taskbar_settings.rows,
                    );
                    let width = geometry.width;
                    let height = geometry.height;
                    window.on_window_should_close(cx, move |window, _| {
                        close_worker_stop.store(true, Ordering::Release);
                        if let Some(worker) = close_worker_handle.borrow_mut().take() {
                            let _ = worker.join();
                        }
                        close_provider_stop.store(true, Ordering::Release);
                        if let Some(worker) = close_provider_worker.borrow_mut().take() {
                            let _ = worker.join();
                        }
                        let close_geometry = taskbar_physical_geometry(
                            &close_monitor,
                            shell,
                            close_settings.borrow().taskbar.rows,
                        );
                        if let Ok(raw) = hwnd(window) {
                            let _ = move_owned_taskbar_client(
                                raw,
                                close_geometry.left,
                                close_geometry.top,
                                close_geometry.width,
                                close_geometry.height,
                            );
                            trace_action("taskbar:auto-hide-close-visible");
                        }
                        true
                    });
                    let configured = hwnd(window).and_then(|value| {
                        let mut lease = ControlledShellCapability::attach_controlled_window(value)
                            .map_err(|_| "taskbar-capability-attach")?;
                        let force_appbar_unavailable = force_appbar_unavailable_for_verification();
                        let appbar_available = if force_appbar_unavailable {
                            false
                        } else {
                            !shell
                                || production_taskbar_settings.auto_hide
                                || lease.register_appbar().is_ok()
                        };
                        lease
                            .register_shell_hook()
                            .map_err(|_| "taskbar-hook-register")?;
                        if !appbar_available && (shell || force_appbar_unavailable) {
                            trace_action("taskbar:appbar-unavailable-owned-shell");
                            trace_action("taskbar:appbar-fallback-geometry-active");
                        } else if shell && production_taskbar_settings.auto_hide {
                            trace_action("taskbar:auto-hide-appbar-skipped");
                        } else {
                            trace_action("taskbar:preview-shell-hook-owned");
                        }
                        platform_win::common::taskbar::set_owned_taskbar_resizable(
                            value,
                            !production_taskbar_settings.locked,
                        )
                        .map_err(|_| "taskbar-resize-style-configure")?;
                        configure_and_show_taskbar_window(
                            value,
                            geometry.left,
                            geometry.top,
                            width,
                            height,
                        )
                        .map_err(|_| "taskbar-window-configure")?;
                        let worker_anchor_bottom = if shell {
                            worker_monitor.bounds.bottom
                        } else {
                            worker_monitor.work_area.bottom
                        };
                        let reveal_worker = std::thread::Builder::new()
                            .name("superdesktop-auto-hide-reveal".into())
                            .spawn(move || {
                                while !worker_stop.load(Ordering::Acquire) {
                                    if worker_enabled.load(Ordering::Acquire)
                                        && worker_hidden.load(Ordering::Acquire)
                                    {
                                        let rows = worker_rows.load(Ordering::Acquire).clamp(1, 3)
                                            as u8;
                                        let height = taskbar_physical_geometry(
                                            &worker_monitor,
                                            shell,
                                            rows,
                                        )
                                        .height;
                                        if let Some(endpoints) = auto_hide_endpoints(
                                            worker_monitor.bounds.left,
                                            worker_monitor.bounds.right,
                                            worker_anchor_bottom,
                                            height,
                                        ) && let Ok((x, y)) = physical_cursor_position()
                                        {
                                            if y >= worker_anchor_bottom.saturating_sub(4) {
                                                trace_action(&format!(
                                                    "taskbar:auto-hide-fast-cursor:{x}:{y}:{worker_anchor_bottom}"
                                                ));
                                            }
                                            if endpoints
                                                .reveal
                                                .contains(taskbar_ui::PhysicalPoint { x, y })
                                            && post_owned_taskbar_reveal(
                                                value,
                                                endpoints.visible.left,
                                                endpoints.visible.top,
                                                endpoints.visible.height(),
                                            )
                                            .is_ok()
                                            {
                                                worker_hidden.store(false, Ordering::Release);
                                                trace_action("taskbar:auto-hide-fast-shown");
                                            }
                                        }
                                    }
                                    std::thread::sleep(Duration::from_millis(50));
                                }
                            })
                            .map_err(|_| "taskbar-auto-hide-worker-spawn")?;
                        *worker_handle_slot.borrow_mut() = Some(reveal_worker);
                        if lease.appbar_registered() && !production_taskbar_settings.auto_hide {
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
                        }
                        taskbar_leases.borrow_mut().push(lease);
                        if shell {
                            trace_action("taskbar:appbar-owned");
                        }
                        Ok(())
                    });
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
                        let previews_enabled_for_click =
                            production_taskbar_settings.previews_enabled;
                        let previews_enabled_for_hover =
                            production_taskbar_settings.previews_enabled;
                        let preview_owned_shell_for_click = shell;
                        let preview_owned_shell_for_hover = shell;
                        let preview_taskbar_rows_for_click = production_taskbar_settings.rows;
                        let preview_taskbar_rows_for_hover = production_taskbar_settings.rows;
                        let start_taskbar_rows = production_taskbar_settings.rows;
                        let jump_list_taskbar_rows = production_taskbar_settings.rows;
                        let taskbar_hwnd_for_click = Rc::clone(&taskbar_hwnd_identity);
                        let taskbar_hwnd_for_hover = Rc::clone(&taskbar_hwnd_identity);
                        let active_preview_for_click = Rc::clone(&active_preview_hwnd);
                        let active_preview_for_hover = Rc::clone(&active_preview_hwnd);
                        let active_preview_for_context = Rc::clone(&active_preview_hwnd);
                        let hover_preview_for_context = Rc::clone(&hover_preview_controller);
                        let flyout_window_for_context =
                            Rc::clone(&flyout_window_for_context_seed);
                        let mut view = TaskbarView {
                        accessible_root_name: "SuperTaskbar".into(),
                        layout: TaskbarLayout::calculate(
                            production_taskbar_settings.rows,
                            taskbar_monitor.dpi_x,
                            width as f32,
                            &[],
                            &[],
                        ),
                        tasks: taskbar_tasks,
                        status: status(status_for_taskbar.borrow().snapshot()),
                        system_snapshot: status_for_taskbar.borrow().snapshot().cloned(),
                        system_flyout: None,
                        notification_area: NotificationAreaModel::default(),
                        overlays: taskbar_overlays,
                        show_labels: production_taskbar_settings.show_labels,
                        search_mode: production_taskbar_settings.search_mode,
                        show_task_view: production_taskbar_settings.show_task_view,
                        alignment: production_taskbar_settings.alignment,
                        locked: production_taskbar_settings.locked,
                        callbacks: Some(TaskbarCallbacks {
                            start: Rc::new(move |app| {
                                guard_ui_action("start", || {
                                trace_action("start");
                                let existing_start = *start_window_for_taskbar.borrow();
                                if let Some(existing) = existing_start {
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
                                let start_alignment = {
                                    start_persisted_settings.borrow().taskbar.alignment
                                };
                                let opened = app.open_window(start_options(
                                    &start_monitor,
                                    shell,
                                    start_taskbar_rows,
                                    start_alignment,
                                ), move |window, cx| {
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
                                    Err(error) => report_error("start:open", error),
                                }
                                });
                            }),
                            show_desktop: Rc::new(move |_| {
                                run_show_desktop_cycle(
                                    &show_desktop_session_for_taskbar,
                                    &minimized_shelf_for_taskbar,
                                )
                            }),
                            task_view: Rc::new(move |app| {
                                let existing_task_view = *task_view_window_for_taskbar.borrow();
                                if let Some(existing) = existing_task_view {
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
                            task: Rc::new(move |stable_id, observed_active, observed_minimized, app| {
                                let group_ids = group_window_ids(stable_id);
                                if group_ids.len() <= 1 {
                                    activate_task(
                                        stable_id,
                                        observed_active,
                                        observed_minimized,
                                        &minimized_shelf_for_task,
                                    );
                                    return;
                                }
                                open_task_preview(
                                    stable_id,
                                    app,
                                    &flyout_window_for_taskbar,
                                    &flyout_monitor,
                                    previews_enabled_for_click,
                                    Rc::clone(&hover_preview_for_click),
                                    taskbar_hwnd_for_click.get(),
                                    Rc::clone(&active_preview_for_click),
                                    preview_owned_shell_for_click,
                                    preview_taskbar_rows_for_click,
                                    PreviewOpenSource::Click,
                                );
                            }),
                            task_hover: Rc::new(move |stable_id, hovered, app| {
                                if !previews_enabled_for_hover {
                                    return;
                                }
                                if hovered {
                                    trace_action("task-preview:hover-enter");
                                    let token = hover_preview_for_task
                                        .borrow_mut()
                                        .enter_task(stable_id.to_owned());
                                    schedule_preview_open(
                                        app,
                                        stable_id.to_owned(),
                                        Rc::clone(&hover_preview_for_task),
                                        Rc::clone(&flyout_window_for_hover),
                                        flyout_hover_monitor.clone(),
                                        true,
                                        taskbar_hwnd_for_hover.get(),
                                        Rc::clone(&active_preview_for_hover),
                                        preview_owned_shell_for_hover,
                                        preview_taskbar_rows_for_hover,
                                        token,
                                    );
                                } else {
                                    trace_action("task-preview:hover-leave");
                                    let token = hover_preview_for_task
                                        .borrow_mut()
                                        .leave_task(stable_id);
                                    schedule_preview_close(
                                        app,
                                        Rc::clone(&hover_preview_for_task),
                                        Rc::clone(&flyout_window_for_hover),
                                        Rc::clone(&active_preview_for_hover),
                                        token,
                                    );
                                }
                            }),
                            task_context: Rc::new(move |stable_id, app| {
                                trace_action("taskbar:jump-list-requested");
                                hover_preview_for_context.borrow_mut().cancel();
                                if let Some(preview) = flyout_window_for_context.borrow_mut().take() {
                                    let _ = preview.update(app, |_, window, _| window.remove_window());
                                }
                                active_preview_for_context.set(0);
                                trace_action("task-preview:context-cancelled");
                                let existing_jump_list = *jump_list_window_for_taskbar.borrow();
                                if let Some(existing) = existing_jump_list {
                                    if existing
                                        .update(app, |_, window, _| window.remove_window())
                                        .is_ok()
                                    {
                                        *jump_list_window_for_taskbar.borrow_mut() = None;
                                        return;
                                    }
                                    *jump_list_window_for_taskbar.borrow_mut() = None;
                                }
                                let invoke_minimized_shelf =
                                    Rc::clone(&minimized_shelf_for_jump_list);
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
                                    .filter(|window| {
                                        window.application_identity == application_id
                                            && window.visible
                                            && !window.tool_window
                                            && !window.cloaked
                                            && !window.owned_transient
                                    })
                                    .cloned()
                                    .collect::<Vec<_>>();
                                let target_window = windows
                                    .iter()
                                    .find(|window| selected_ids.contains(&window.hwnd_identity))
                                    .cloned();
                                let pinned = jump_persisted_settings
                                    .borrow()
                                    .taskbar
                                    .pins
                                    .contains(&application_id);
                                let local = taskbar_local_commands(
                                    pinned,
                                    target_window.as_ref().map(|window| window.minimized),
                                    application_windows.len(),
                                );
                                let model = query_jump_list(&jump_list_provider, &application_id, local);
                                let jump_entry_count = model.entries().len();
                                let jump_group_count = model
                                    .groups()
                                    .values()
                                    .filter(|entries| !entries.is_empty())
                                    .count();
                                let jump_anchor_x = physical_cursor_position().ok().map(|(x, _)| x);
                                        let dismiss_slot = Rc::clone(&jump_list_window_for_taskbar);
                                        let minimized_shelf_for_action =
                                            Rc::clone(&invoke_minimized_shelf);
                                let invoke_store = Rc::clone(&jump_settings_store);
                                let invoke_target = Rc::clone(&jump_settings_target);
                                let invoke_settings = Rc::clone(&jump_persisted_settings);
                                let invoke_application = application_id.clone();
                                let jump_topmost = Rc::new(Cell::new(false));
                                let jump_topmost_for_open = Rc::clone(&jump_topmost);
                                let opened = app.open_window(
                                    jump_list_options(
                                        &jump_list_monitor,
                                        shell,
                                        jump_list_taskbar_rows,
                                        jump_anchor_x,
                                        jump_entry_count,
                                        jump_group_count,
                                    ),
                                    move |window, cx| {
                                        window.activate_window();
                                        jump_topmost_for_open.set(promote_owned_context_popup(
                                            window,
                                            "taskbar:jump-list",
                                        ));
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            JumpListView::new(
                                                model,
                                                Rc::new(move |command| {
                                                    let minimizing = command.id.0
                                                        == "local:taskbar-minimize";
                                                    let apply_target = |action| {
                                                        target_window.as_ref().is_some_and(|target| {
                                                            platform_win::common::taskbar::apply_window_action_to_owned_identity(
                                                                target.hwnd_identity,
                                                                target.process_id,
                                                                &target.window_identity,
                                                                action,
                                                            )
                                                            .is_ok()
                                                        })
                                                    };
                                                    let completed = match command.id.0.as_str() {
                                                        "local:taskbar-minimize" => apply_target(
                                                            platform_win::common::taskbar::WindowAction::Minimize,
                                                        ),
                                                        "local:taskbar-maximize" => apply_target(
                                                            platform_win::common::taskbar::WindowAction::Maximize,
                                                        ),
                                                        "local:taskbar-close-window" => apply_target(
                                                            platform_win::common::taskbar::WindowAction::Close,
                                                        ),
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
                                                        "local:taskbar-close-all" => application_windows.iter().all(|target| {
                                                            platform_win::common::taskbar::apply_window_action_to_owned_identity(
                                                                target.hwnd_identity,
                                                                target.process_id,
                                                                &target.window_identity,
                                                                platform_win::common::taskbar::WindowAction::Close,
                                                            )
                                                            .is_ok()
                                                        }),
                                                        _ => activate_jump_command(command),
                                                    };
                                                    if minimizing && completed {
                                                        reconcile_minimized_window_shelf_snapshot(
                                                            &minimized_shelf_for_action,
                                                        );
                                                    }
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
                                                window,
                                                cx,
                                            )
                                        })
                                    },
                                );
                                if let Ok(handle) = opened {
                                    if jump_topmost.get() {
                                        *jump_list_window_for_taskbar.borrow_mut() = Some(handle);
                                        trace_action("taskbar:jump-list-opened");
                                    } else {
                                        let _ = handle.update(app, |_, window, _| {
                                            window.remove_window()
                                        });
                                    }
                                }
                            }),
                            taskbar_context: Rc::new(move |anchor, app| {
                                let existing_context = *context_window_for_taskbar.borrow();
                                if let Some(existing) = existing_context {
                                    let _ = existing.update(app, |_, window, _| window.remove_window());
                                    *context_window_for_taskbar.borrow_mut() = None;
                                }
                                let context_slot = Rc::clone(&context_window_for_taskbar);
                                let settings_slot = Rc::clone(&settings_window_for_context);
                                let settings_monitor = context_monitor.clone();
                                let settings_store = Rc::clone(&context_settings_store);
                                let settings_target = Rc::clone(&context_settings_target);
                                let live_settings = Rc::clone(&context_persisted_settings);
                                let context_show_desktop = Rc::clone(&show_desktop_session_for_context);
                                let context_minimized_shelf = Rc::clone(&minimized_shelf_for_context);
                                let context_rows = live_settings.borrow().taskbar.rows;
                                let context_topmost = Rc::new(Cell::new(false));
                                let context_topmost_for_open = Rc::clone(&context_topmost);
                                let opened = app.open_window(
                                    taskbar_context_options(
                                        &context_monitor,
                                        shell,
                                        context_rows,
                                        anchor,
                                    ),
                                    move |window, cx| {
                                        window.activate_window();
                                        context_topmost_for_open.set(promote_owned_context_popup(
                                            window,
                                            "taskbar:context",
                                        ));
                                        let dismiss_slot = Rc::clone(&context_slot);
                                        let settings_slot_for_action = Rc::clone(&settings_slot);
                                        let monitor_for_action = settings_monitor.clone();
                                        let store_for_action = Rc::clone(&settings_store);
                                        let target_for_action = Rc::clone(&settings_target);
                                        let live_for_action = Rc::clone(&live_settings);
                                        let context_show_desktop_for_action = Rc::clone(&context_show_desktop);
                                        let context_minimized_shelf_for_action = Rc::clone(&context_minimized_shelf);
                                        let context_settings = live_for_action.borrow().taskbar.clone();
                                        cx.new(move |cx| {
                                            TaskbarContextView::new(
                                                context_settings.locked,
                                                context_settings.search_mode,
                                                context_settings.show_task_view,
                                                Rc::new(move |command, app| match command {
                                                    TaskbarContextCommand::CycleSearchMode
                                                    | TaskbarContextCommand::ToggleTaskView
                                                    | TaskbarContextCommand::ToggleLockTaskbar => {
                                                        let mut updated =
                                                            live_for_action.borrow().clone();
                                                        let changed = apply_taskbar_context_setting(
                                                            &mut updated.taskbar,
                                                            command,
                                                        );
                                                        debug_assert!(changed);
                                                        match store_for_action
                                                            .borrow_mut()
                                                            .save(&target_for_action, &updated)
                                                        {
                                                            Ok(saved) => {
                                                                *live_for_action.borrow_mut() =
                                                                    saved;
                                                                trace_action(match command {
                                                                    TaskbarContextCommand::CycleSearchMode => "taskbar:search-mode-cycled",
                                                                    TaskbarContextCommand::ToggleTaskView => "taskbar:task-view-toggled",
                                                                    _ => "taskbar:lock-toggled",
                                                                });
                                                            }
                                                            Err(_) => trace_action(
                                                                "taskbar:context-setting-save-rejected",
                                                            ),
                                                        }
                                                    }
                                                    TaskbarContextCommand::ShowDesktop => {
                                                        run_show_desktop_cycle(
                                                            &context_show_desktop_for_action,
                                                            &context_minimized_shelf_for_action,
                                                        );
                                                        trace_action("taskbar:context-show-desktop");
                                                    }
                                                    TaskbarContextCommand::OpenTaskManager => {
                                                        trace_action(if platform_win::common::taskbar::launch_task_manager().is_ok() {
                                                            "taskbar:task-manager-launched"
                                                        } else {
                                                            "taskbar:task-manager-rejected"
                                                        });
                                                    }
                                                    TaskbarContextCommand::OpenTaskbarSettings => {
                                                        let existing_settings = *settings_slot_for_action.borrow();
                                                        if let Some(existing) = existing_settings {
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
                                                    TaskbarContextCommand::ReturnToDefaultExplorer => {
                                                        match crate::restore_default_explorer_registration().and_then(|_| {
                                                            platform_win::common::explorer_recovery::recover_explorer_shell()
                                                                .map(|_| ())
                                                                .map_err(str::to_owned)
                                                        }) {
                                                            Ok(_) => {
                                                                trace_action("explorer-return:verified");
                                                                app.quit();
                                                            }
                                                            Err(error) => report_error(
                                                                "explorer-return",
                                                                error,
                                                            ),
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
                                    if context_topmost.get() {
                                        *context_window_for_taskbar.borrow_mut() = Some(handle);
                                        trace_action("taskbar:context-opened");
                                    } else {
                                        let _ = handle.update(app, |_, window, _| {
                                            window.remove_window()
                                        });
                                    }
                                }
                            }),
                            resize_rows: Rc::new(move |rows, window, _| {
                                let rows = rows.clamp(1, 3);
                                let current = taskbar_resize_settings.borrow().clone();
                                if current.taskbar.rows != rows {
                                    if current.taskbar.locked {
                                        trace_action("taskbar:resize-locked");
                                        return false;
                                    }
                                    let mut updated = current;
                                    updated.taskbar.rows = rows;
                                    match taskbar_resize_store
                                        .borrow_mut()
                                        .save(&taskbar_resize_target, &updated)
                                    {
                                        Ok(saved) => {
                                            *taskbar_resize_settings.borrow_mut() = saved;
                                            *taskbar_resize_settle.borrow_mut() =
                                                Some(Instant::now() + Duration::from_millis(350));
                                            trace_action("taskbar:resize-saved");
                                            return false;
                                        }
                                        Err(_) => {
                                            trace_action("taskbar:resize-save-rejected");
                                            return false;
                                        }
                                    }
                                }
                                if taskbar_resize_settle
                                    .borrow()
                                    .is_some_and(|deadline| Instant::now() < deadline)
                                {
                                    return false;
                                }
                                let geometry = taskbar_physical_geometry(
                                    &taskbar_resize_monitor,
                                    shell,
                                    rows,
                                );
                                let Ok(raw) = hwnd(window) else {
                                    trace_action("taskbar:resize-hwnd-rejected");
                                    return false;
                                };
                                if configure_and_show_taskbar_window(
                                    raw,
                                    geometry.left,
                                    geometry.top,
                                    geometry.width,
                                    geometry.height,
                                )
                                .is_err()
                                {
                                    trace_action("taskbar:resize-snap-rejected");
                                    return false;
                                }
                                if shell && !taskbar_resize_settings.borrow().taskbar.auto_hide {
                                    let mut leases = taskbar_resize_leases.borrow_mut();
                                    let Some(lease) =
                                        leases.iter_mut().find(|lease| lease.owns_window(raw))
                                    else {
                                        trace_action("taskbar:resize-lease-missing");
                                        return false;
                                    };
                                    if lease.appbar_registered() {
                                        if lease
                                            .reserve_bottom(
                                            ScreenRect {
                                                left: taskbar_resize_monitor.bounds.left,
                                                top: taskbar_resize_monitor.bounds.top,
                                                right: taskbar_resize_monitor.bounds.right,
                                                bottom: taskbar_resize_monitor.bounds.bottom,
                                            },
                                            geometry.height,
                                        )
                                            .is_err()
                                        {
                                            trace_action("taskbar:resize-appbar-rejected");
                                            return false;
                                        }
                                        trace_action("taskbar:resize-appbar-synced");
                                    } else {
                                        trace_action("taskbar:resize-owned-workarea-synced");
                                    }
                                }
                                *taskbar_resize_settle.borrow_mut() = None;
                                trace_action("taskbar:resize-applied");
                                true
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
                                let dismiss_slot =
                                    Rc::clone(&notification_overflow_window_for_taskbar);
                                let event_client = Rc::clone(&notification_client_for_overflow);
                                let opened = app.open_window(
                                    notification_overflow_options(
                                        &notification_overflow_monitor,
                                        shell,
                                        nodes.len(),
                                        notification_overflow_settings.borrow().taskbar.rows,
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
                                apply_system_status_action_or_queue_volume(
                                    action,
                                    app,
                                    &status_client_for_taskbar,
                                    &status_commands_for_taskbar,
                                    &start_window_for_status,
                                    &volume_commands_for_taskbar,
                                );
                            }),
                            system_flyout: Rc::new(move |kind, app| {
                                if let Some((_, _, handle)) =
                                    system_context_window_for_flyout.borrow_mut().take()
                                {
                                    let _ = handle.update(app, |_, window, _| window.remove_window());
                                    trace_action("status:context-dismissed-for-flyout");
                                }
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
                                if kind == SystemFlyoutKind::NetworkPower {
                                    apply_system_status_action(
                                        SystemStatusAction::RefreshWifi,
                                        app,
                                        &system_flyout_client,
                                        &system_flyout_status,
                                        &system_flyout_start,
                                    );
                                }
                                let snapshot = system_flyout_status.borrow().snapshot().cloned();
                                let notifications =
                                    notification_snapshot_for_center.borrow().clone();
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
                                let action_volume = system_flyout_volume.clone();
                                let center_client = Rc::clone(&notification_client_for_center);
                                let center_snapshot = Rc::clone(&notification_snapshot_for_center);
                                let dismiss_slot =
                                    Rc::clone(&system_flyout_window_for_taskbar);
                                let presentation = system_flyout_presentation();
                                let taskbar_rows =
                                    system_flyout_settings.borrow().taskbar.rows;
                                let opened = app.open_window(
                                    system_flyout_options(
                                        &system_flyout_monitor,
                                        shell,
                                        kind,
                                        input_profile_count,
                                        notifications
                                            .as_ref()
                                            .map_or(0, |snapshot| snapshot.notifications.len()),
                                        taskbar_rows,
                                    ),
                                    move |window, cx| {
                                        window.activate_window();
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            SystemFlyoutView::new(
                                                kind,
                                                snapshot,
                                                flyout_status,
                                                notifications,
                                                presentation,
                                                Rc::new(move |action, app| {
                                                    apply_system_status_action_or_queue_volume(
                                                        action,
                                                        app,
                                                        &action_client,
                                                        &action_status,
                                                        &action_start,
                                                        &action_volume,
                                                    );
                                                }),
                                                Rc::new(move |action, _| {
                                                    let result = match action {
                                                        NotificationCenterAction::Dismiss {
                                                            notification_id,
                                                            expected_generation,
                                                        } => center_client
                                                            .borrow_mut()
                                                            .dismiss_notification(
                                                                notification_id,
                                                                expected_generation,
                                                                Duration::from_millis(750),
                                                            ),
                                                        NotificationCenterAction::ClearAll {
                                                            expected_generation,
                                                        } => center_client
                                                            .borrow_mut()
                                                            .clear_notifications(
                                                                expected_generation,
                                                                Duration::from_millis(750),
                                                            ),
                                                    }
                                                    .map_err(ToOwned::to_owned);
                                                    if let Ok(snapshot) = &result {
                                                        *center_snapshot.borrow_mut() =
                                                            Some(snapshot.clone());
                                                        trace_action(
                                                            "notification:center-reconciled",
                                                        );
                                                    } else {
                                                        trace_action(
                                                            "notification:center-action-rejected",
                                                        );
                                                    }
                                                    result
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
                                } else {
                                    trace_action("status:flyout-open-failed");
                                }
                            }),
                            system_context: Rc::new(move |kind, anchor, app| {
                                if let Some((open_kind, _, handle)) =
                                    system_context_window_for_taskbar.borrow_mut().take()
                                {
                                    let _ = handle.update(app, |_, window, _| window.remove_window());
                                    if open_kind == kind {
                                        trace_action("status:context-closed");
                                        return;
                                    }
                                }
                                if let Some((_, handle)) =
                                    system_flyout_window_for_context.borrow_mut().take()
                                {
                                    let _ = handle.update(app, |_, window, _| window.remove_window());
                                    trace_action("status:flyout-dismissed-for-context");
                                }
                                let generation = system_context_generation_for_taskbar
                                    .get()
                                    .saturating_add(1);
                                system_context_generation_for_taskbar.set(generation);
                                let dismiss_slot = Rc::clone(&system_context_window_for_taskbar);
                                let action_client = Rc::clone(&system_context_status_client);
                                let action_status = Rc::clone(&system_context_status);
                                let action_start = Rc::clone(&system_context_start);
                                let system_context_topmost = Rc::new(Cell::new(false));
                                let system_context_topmost_for_open =
                                    Rc::clone(&system_context_topmost);
                                let opened = app.open_window(
                                    system_control_context_options(
                                        &system_context_monitor,
                                        shell,
                                        system_context_settings.borrow().taskbar.rows,
                                        anchor,
                                        kind,
                                    ),
                                    move |window, cx| {
                                        window.activate_window();
                                        system_context_topmost_for_open.set(
                                            promote_owned_context_popup(
                                                window,
                                                "status:context",
                                            ),
                                        );
                                        let dismiss_slot = Rc::clone(&dismiss_slot);
                                        cx.new(move |cx| {
                                            SystemControlContextView::new(
                                                kind,
                                                Rc::new(move |command, app| match command {
                                                    SystemControlContextCommand::LanguagePreferences => {
                                                        apply_system_status_action(
                                                            SystemStatusAction::OpenLanguagePreferences,
                                                            app,
                                                            &action_client,
                                                            &action_status,
                                                            &action_start,
                                                        );
                                                        trace_action("status:input-context-language-preferences");
                                                    }
                                                    SystemControlContextCommand::OpenVolumeMixer => {
                                                        trace_action(if platform_win::common::taskbar::open_volume_mixer().is_ok() {
                                                            "status:volume-mixer-launched"
                                                        } else {
                                                            "status:volume-mixer-rejected"
                                                        });
                                                    }
                                                    SystemControlContextCommand::OpenSoundSettings => {
                                                        trace_action(if platform_win::common::taskbar::open_sound_settings().is_ok() {
                                                            "status:sound-settings-launched"
                                                        } else {
                                                            "status:sound-settings-rejected"
                                                        });
                                                    }
                                                }),
                                                Rc::new(move |window, _| {
                                                    window.remove_window();
                                                    let is_current = dismiss_slot
                                                        .borrow()
                                                        .as_ref()
                                                        .is_some_and(|(_, current, _)| *current == generation);
                                                    if is_current {
                                                        *dismiss_slot.borrow_mut() = None;
                                                    }
                                                    trace_action("status:context-dismissed");
                                                }),
                                                window,
                                                cx,
                                            )
                                        })
                                    },
                                );
                                if let Ok(handle) = opened {
                                    if system_context_topmost.get() {
                                        *system_context_window_for_taskbar.borrow_mut() =
                                            Some((kind, generation, handle));
                                        trace_action(match kind {
                                            SystemControlContextKind::Input => "status:input-context-opened",
                                            SystemControlContextKind::Volume => "status:volume-context-opened",
                                        });
                                    } else {
                                        let _ = handle.update(app, |_, window, _| {
                                            window.remove_window()
                                        });
                                    }
                                } else {
                                    trace_action("status:context-open-failed");
                                }
                            }),
                            rendered: Rc::new(trace_rendered_frame),
                        }),
                        keyboard_focus: focus_handle,
                        resize_subscription: None,
                    };
                    view.attach_resize_observer(window, cx);
                    view
                    })
                });
                let Ok(taskbar) = taskbar else {
                    trace_action("taskbar:init-error:window-open");
                    *terminal_for_app.borrow_mut() = Some(Err("taskbar-window-open"));
                    cx.quit();
                    return;
                };
                let hold_start = Rc::clone(&start_window);
                let hold_task_flyout = Rc::clone(&flyout_window);
                let hold_jump_list = Rc::clone(&jump_list_window);
                let hold_task_view = Rc::clone(&task_view_window);
                let hold_notification_overflow = Rc::clone(&notification_overflow_window);
                let hold_system_flyout = Rc::clone(&system_flyout_window);
                let visibility_hold: Rc<dyn Fn() -> bool> = Rc::new(move || {
                    hold_start.borrow().is_some()
                        || hold_task_flyout.borrow().is_some()
                        || hold_jump_list.borrow().is_some()
                        || hold_task_view.borrow().is_some()
                        || hold_notification_overflow.borrow().is_some()
                        || hold_system_flyout.borrow().is_some()
                        || auto_hide_resize_settle.borrow().is_some()
                });
                taskbar_auto_hide.push(TaskbarAutoHideRuntime {
                    monitor: auto_hide_monitor,
                    state: auto_hide_state,
                    enabled: auto_hide_enabled,
                    visibility_hold,
                    fast_hidden: auto_hide_fast_hidden,
                    fast_enabled: auto_hide_fast_enabled,
                    fast_rows: auto_hide_fast_rows,
                    stop_reveal_worker: auto_hide_worker_stop,
                    reveal_worker: auto_hide_worker_handle,
                });
                taskbar_monitor_names.push(taskbar_monitor_name);
                taskbar_handles.push(taskbar);
            }
            if let Some(error) = init_error.borrow_mut().take() {
                trace_action(&format!("taskbar:init-error:{error}"));
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
                            if let Err(error) = transfer_app.try_update(|app| {
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
                            }) {
                                report_error("desktop:transfer-update", error);
                            }
                        }
                    })
                    .detach();
            }

            let auto_hide_handles = taskbar_handles.clone();
            if !auto_hide_handles.is_empty() && !state_matrix {
                let auto_hide_background = cx.background_executor().clone();
                let auto_hide_foreground = cx.foreground_executor().clone();
                let auto_hide_app = cx.to_async();
                let auto_hide_runtimes = taskbar_auto_hide.clone();
                let auto_hide_settings = Rc::clone(&persisted_settings);
                let auto_hide_attention = Rc::clone(&attention_runtime);
                let auto_hide_context = Rc::clone(&taskbar_context_window);
                let auto_hide_settings_window = Rc::clone(&taskbar_settings_window);
                let auto_hide_leases = Rc::clone(&leases);
                auto_hide_foreground
                    .spawn(async move {
                        let epoch = Instant::now();
                        loop {
                            auto_hide_background.timer(Duration::from_millis(50)).await;
                            let settings = auto_hide_settings.borrow().taskbar.clone();
                            let attention_hold = !auto_hide_attention
                                .borrow()
                                .active_windows()
                                .is_empty();
                            let context_open = auto_hide_context.borrow().is_some();
                            let settings_open = auto_hide_settings_window.borrow().is_some();
                            if let Err(error) = auto_hide_app.try_update(|app| {
                                for (index, handle) in auto_hide_handles.iter().enumerate() {
                                    let Some(runtime) = auto_hide_runtimes.get(index) else {
                                        continue;
                                    };
                                    let _ = handle.update(app, |view, window, _| {
                                        reconcile_taskbar_auto_hide(
                                            runtime,
                                            shell,
                                            &settings,
                                            attention_hold,
                                            context_open,
                                            settings_open,
                                            &auto_hide_leases,
                                            view,
                                            window,
                                            epoch,
                                        );
                                    });
                                }
                            }) {
                                report_error("taskbar:auto-hide-update", error);
                            }
                        }
                    })
                    .detach();
            }

            let refresh_handles = taskbar_handles.clone();
            let refresh_monitor_names = taskbar_monitor_names.clone();
            let refresh_show_desktop_sessions = shell_show_desktop_sessions.clone();
            if !refresh_handles.is_empty() && !state_matrix {
                let refresh_background = cx.background_executor().clone();
                let refresh_foreground = cx.foreground_executor().clone();
                let refresh_app = cx.to_async();
                let refresh_status_reconciler = Rc::clone(&status_reconciler);
                let refresh_status_client = Rc::clone(&status_client);
                let refresh_hotkey_start_window =
                    Rc::new(RefCell::new(None::<gpui::WindowHandle<StartView>>));
                let refresh_taskbar_state_reconciler = Rc::clone(&taskbar_state_reconciler);
                let refresh_provider_batch = Arc::clone(&provider_refresh_batch);
                let refresh_system_flyouts = system_flyout_windows.clone();
                let refresh_notification_snapshot = Rc::clone(&notification_snapshot);
                let refresh_notification_client = Rc::clone(&notification_client);
                let refresh_settings = Rc::clone(&persisted_settings);
                let refresh_task_icons = Rc::clone(&task_icon_cache);
                let refresh_task_icon_edge = Rc::clone(&task_icon_edge);
                let refresh_minimized_window_shelf = Rc::clone(&minimized_window_shelf);
                let refresh_attention = Rc::clone(&attention_runtime);
                let refresh_shell_hotkeys = Rc::clone(&shell_hotkeys);
                let refresh_auto_hide_context = Rc::clone(&taskbar_context_window);
                let refresh_auto_hide_settings_window = Rc::clone(&taskbar_settings_window);
                let refresh_auto_hide_leases = Rc::clone(&leases);
                let refresh_alt_tab_window = Rc::clone(&alt_tab_window);
                let refresh_alt_tab_monitor = alt_tab_monitor.clone();
                refresh_foreground
                    .spawn(async move {
                        let auto_hide_epoch = Instant::now();
                        let mut notification_tick = 0u8;
                        let mut pending_superexplorer_focus = None::<PendingSuperExplorerFocus>;
                        let mut last_monitor_geometries =
                            vec![None::<TaskbarPhysicalGeometry>; refresh_handles.len()];
                        loop {
                            refresh_background.timer(Duration::from_millis(50)).await;
                            let shell_hotkey_action = refresh_shell_hotkeys
                                .as_ref()
                                .as_ref()
                                .and_then(|hotkeys| hotkeys.take_requested());
                            if shell_hotkey_action
                                == Some(platform_win::common::shell_hotkey::ShellHotkeyAction::OpenExplorer)
                            {
                                trace_action("shell-hotkey:win-e");
                                pending_superexplorer_focus = request_superexplorer_foreground();
                            }
                            if let Some(pending) = pending_superexplorer_focus.as_ref() {
                                match focus_superexplorer_window(
                                    &pending.executable,
                                    Some(pending.process_id),
                                ) {
                                    Ok(true) => {
                                        trace_action("win-e:superexplorer-activated");
                                        pending_superexplorer_focus = None;
                                    }
                                    Ok(false) if Instant::now() >= pending.deadline => {
                                        report_error(
                                            "win-e:focus-timeout",
                                            "SuperExplorer window did not appear within 3 seconds",
                                        );
                                        pending_superexplorer_focus = None;
                                    }
                                    Ok(false) => {}
                                    Err(error) => {
                                        report_error("win-e:focus", error);
                                        pending_superexplorer_focus = None;
                                    }
                                }
                            }
                            notification_tick = notification_tick.wrapping_add(1);
                            let refreshed_geometries = if notification_tick.is_multiple_of(10) {
                                let explorer_shell_present =
                                    match trusted_explorer_shell_present() {
                                        Ok(present) => present,
                                        Err(error) => {
                                            report_error("taskbar:explorer-presence", error);
                                            true
                                        }
                                    };
                                let taskbar_bounds_mode = taskbar_uses_monitor_bounds(
                                    shell,
                                    explorer_shell_present,
                                );
                                match snapshot_real_monitors() {
                                    Ok(snapshot) => {
                                        let next_icon_edge =
                                            task_icon_source_edge(&snapshot.monitors);
                                        if refresh_task_icon_edge.replace(next_icon_edge)
                                            != next_icon_edge
                                        {
                                            refresh_task_icons.borrow_mut().clear();
                                            trace_action(&format!(
                                                "taskbar:icon-source-edge:{next_icon_edge}"
                                            ));
                                        }
                                        refresh_monitor_names
                                            .iter()
                                            .enumerate()
                                            .map(|(index, device_name)| {
                                            let next = snapshot
                                                .monitors
                                                .iter()
                                                .find(|monitor| {
                                                    monitor.device_name == *device_name
                                                })
                                                .map(|monitor| {
                                                    taskbar_physical_geometry(
                                                        monitor,
                                                        taskbar_bounds_mode,
                                                        refresh_settings.borrow().taskbar.rows,
                                                    )
                                                });
                                            if last_monitor_geometries.get(index).copied().flatten()
                                                == next
                                            {
                                                None
                                            } else {
                                                if let Some(slot) =
                                                    last_monitor_geometries.get_mut(index)
                                                {
                                                    *slot = next;
                                                }
                                                next
                                            }
                                            })
                                            .collect::<Vec<_>>()
                                    }
                                    Err(error) => {
                                        report_error("taskbar:monitor-refresh", error);
                                        vec![None; refresh_handles.len()]
                                    }
                                }
                            } else {
                                vec![None; refresh_handles.len()]
                            };
                            let direct_notification_result =
                                notification_tick.is_multiple_of(10).then(|| {
                                    refresh_notification_client
                                        .borrow_mut()
                                        .request(
                                            &NotificationMutation::Snapshot,
                                            Duration::from_millis(100),
                                        )
                                        .map_err(ToOwned::to_owned)
                                });
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
                                refresh_task_icon_edge.get(),
                                &mut refresh_task_icons.borrow_mut(),
                                &attention_windows,
                                shell.then_some(&refresh_minimized_window_shelf),
                            ) else {
                                continue;
                            };
                            let (_, status_result, taskbar_result) =
                                refresh_provider_batch.lock().map_or(
                                    (None, None, None),
                                    |mut batch| {
                                        (
                                            batch.notification.take(),
                                            batch.status.take(),
                                            batch.taskbar.take(),
                                        )
                                    },
                                );
                            let notification_result = direct_notification_result;
                            let notification_update = notification_result.map(|result| match result {
                                Ok(NotificationHostResponse::Snapshot(snapshot)) => Some(snapshot),
                                Ok(_) => None,
                                Err(reason) => {
                                    trace_action(&format!("notification:provider-error:{reason}"));
                                    None
                                }
                            });
                            if let Some(Some(snapshot)) = notification_update.as_ref() {
                                *refresh_notification_snapshot.borrow_mut() =
                                    Some(snapshot.clone());
                            }
                            if let Some(result) = status_result {
                                match result {
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
                            if let Some(result) = taskbar_result {
                                match result {
                                    Ok(snapshot) => {
                                        refresh_taskbar_state_reconciler
                                            .borrow_mut()
                                            .apply(snapshot);
                                    }
                                    Err(reason) if reason == "taskbar-state-disabled" => {}
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
                            if let Err(error) = refresh_app.try_update(|app| {
                                if let Some(action) = shell_hotkey_action {
                                    match action {
                                        ShellHotkeyAction::OpenExplorer => {}
                                        ShellHotkeyAction::ShowDesktop => {
                                            if let Some(session) =
                                                refresh_show_desktop_sessions.first()
                                            {
                                                run_show_desktop_cycle(
                                                    session,
                                                    &refresh_minimized_window_shelf,
                                                );
                                                trace_action("shell-hotkey:show-desktop");
                                            }
                                        }
                                        ShellHotkeyAction::CycleInput
                                        | ShellHotkeyAction::CycleInputPrevious => {
                                            if let Some(profile_id) = current_system_snapshot
                                                .as_ref()
                                                .and_then(|snapshot| {
                                                    adjacent_input_profile_id(
                                                        snapshot,
                                                        if action
                                                            == ShellHotkeyAction::CycleInput
                                                        {
                                                            1
                                                        } else {
                                                            -1
                                                        },
                                                    )
                                                })
                                            {
                                                apply_system_status_action(
                                                    SystemStatusAction::ActivateInputProfile(
                                                        profile_id,
                                                    ),
                                                    app,
                                                    &refresh_status_client,
                                                    &refresh_status_reconciler,
                                                    &refresh_hotkey_start_window,
                                                );
                                                trace_action("shell-hotkey:cycle-input");
                                            } else {
                                                report_error(
                                                    "shell-hotkey:cycle-input",
                                                    "no alternate input profile is available",
                                                );
                                            }
                                        }
                                        ShellHotkeyAction::OpenSearch => {
                                            let callback = refresh_handles.iter().find_map(|handle| {
                                                handle
                                                    .update(app, |view, _, _| {
                                                        view.callbacks.as_ref().map(|callbacks| {
                                                            Rc::clone(&callbacks.start)
                                                        })
                                                    })
                                                    .ok()
                                                    .flatten()
                                            });
                                            if let Some(callback) = callback {
                                                callback(app);
                                                trace_action("shell-hotkey:search");
                                            }
                                        }
                                        ShellHotkeyAction::ToggleStart => {
                                            let callback = refresh_handles.iter().find_map(|handle| {
                                                handle
                                                    .update(app, |view, _, _| {
                                                        view.callbacks.as_ref().map(|callbacks| {
                                                            Rc::clone(&callbacks.start)
                                                        })
                                                    })
                                                    .ok()
                                                    .flatten()
                                            });
                                            if let Some(callback) = callback {
                                                callback(app);
                                                trace_action("shell-hotkey:start-toggle");
                                            } else {
                                                report_error(
                                                    "shell-hotkey:start-toggle",
                                                    "no live taskbar Start callback is available",
                                                );
                                            }
                                        }
                                        ShellHotkeyAction::OpenScreenSnip => {
                                            trace_action("shell-hotkey:screen-snip-requested");
                                            if let Err(error) = std::thread::Builder::new()
                                                .name("superdesktop-screen-snip".into())
                                                .spawn(|| {
                                                    match platform_win::common::shell_hotkey::open_screen_snipping_overlay() {
                                                        Ok(()) => trace_action("shell-hotkey:screen-snip-accepted"),
                                                        Err(error) => report_error("shell-hotkey:screen-snip", error),
                                                    }
                                                })
                                            {
                                                report_error("shell-hotkey:screen-snip-worker", error);
                                            }
                                        }
                                        ShellHotkeyAction::OpenTaskView => {
                                            let callback = refresh_handles.iter().find_map(|handle| {
                                                handle
                                                    .update(app, |view, _, _| {
                                                        view.callbacks.as_ref().map(|callbacks| {
                                                            Rc::clone(&callbacks.task_view)
                                                        })
                                                    })
                                                    .ok()
                                                    .flatten()
                                            });
                                            if let Some(callback) = callback {
                                                callback(app);
                                                trace_action("shell-hotkey:task-view");
                                            }
                                        }
                                        ShellHotkeyAction::OpenNetworkPower
                                        | ShellHotkeyAction::OpenNotifications => {
                                            let callback = refresh_handles.iter().find_map(|handle| {
                                                handle
                                                    .update(app, |view, _, _| {
                                                        view.callbacks.as_ref().map(|callbacks| {
                                                            Rc::clone(&callbacks.system_flyout)
                                                        })
                                                    })
                                                    .ok()
                                                    .flatten()
                                            });
                                            if let Some(callback) = callback {
                                                let kind = if action
                                                    == ShellHotkeyAction::OpenNetworkPower
                                                {
                                                    SystemFlyoutKind::NetworkPower
                                                } else {
                                                    SystemFlyoutKind::Calendar
                                                };
                                                callback(kind, app);
                                                trace_action("shell-hotkey:system-flyout");
                                            }
                                        }
                                        ShellHotkeyAction::AltTabForward
                                        | ShellHotkeyAction::AltTabBackward => {
                                            if let Some(monitor) = &refresh_alt_tab_monitor {
                                                open_or_cycle_alt_tab(
                                                    app,
                                                    &refresh_alt_tab_window,
                                                    monitor,
                                                    if action == ShellHotkeyAction::AltTabForward {
                                                        1
                                                    } else {
                                                        -1
                                                    },
                                                );
                                            }
                                        }
                                        ShellHotkeyAction::AltTabCommit
                                        | ShellHotkeyAction::AltTabCancel => {
                                            close_alt_tab(
                                                app,
                                                &refresh_alt_tab_window,
                                                action == ShellHotkeyAction::AltTabCommit,
                                            );
                                        }
                                    }
                                }
                                let mut alive = false;
                                for (index, handle) in refresh_handles.iter().enumerate() {
                                    let auto_hide = None::<TaskbarAutoHideRuntime>;
                                    let refreshed_geometry = refreshed_geometries
                                        .get(index)
                                        .copied()
                                        .flatten();
                                    if handle
                                        .update(app, |view, window, cx| {
                                            alive = true;
                                            if let Some(geometry) = refreshed_geometry {
                                                match hwnd(window) {
                                                    Ok(raw) => match move_owned_taskbar_client(
                                                        raw,
                                                        geometry.left,
                                                        geometry.top,
                                                        geometry.width,
                                                        geometry.height,
                                                    ) {
                                                        Ok(true) => trace_action(
                                                            "taskbar:monitor-geometry-reconciled",
                                                        ),
                                                        Ok(false) => {}
                                                        Err(error) => report_error(
                                                            "taskbar:monitor-geometry",
                                                            error,
                                                        ),
                                                    },
                                                    Err(error) => report_error(
                                                        "taskbar:monitor-hwnd",
                                                        error,
                                                    ),
                                                }
                                            }
                                            if let Some(runtime) = &auto_hide {
                                                let taskbar_height = taskbar_physical_geometry(
                                                    &runtime.monitor,
                                                    shell,
                                                    taskbar_settings.rows,
                                                )
                                                .height;
                                                let anchor_bottom = if shell {
                                                    runtime.monitor.bounds.bottom
                                                } else {
                                                    runtime.monitor.work_area.bottom
                                                };
                                                if let Some(endpoints) = auto_hide_endpoints(
                                                    runtime.monitor.bounds.left,
                                                    runtime.monitor.bounds.right,
                                                    anchor_bottom,
                                                    taskbar_height,
                                                ) {
                                                    let attention_hold = !attention_windows.is_empty()
                                                        || view.overlays.values().any(|overlay| {
                                                            overlay.attention
                                                                || overlay.attention_phase_on
                                                                || overlay.attention_steady
                                                        });
                                                    let cursor = physical_cursor_position()
                                                        .ok()
                                                        .map(|(x, y)| {
                                                            taskbar_ui::PhysicalPoint { x, y }
                                                        });
                                                    if cursor.is_none() {
                                                        trace_action(
                                                            "taskbar:auto-hide-cursor-unavailable",
                                                        );
                                                    }
                                                    let enabled = taskbar_settings.auto_hide;
                                                    let was_enabled = *runtime.enabled.borrow();
                                                    let transition_pending = was_enabled != enabled;
                                                    let visibility_hold =
                                                        (runtime.visibility_hold)()
                                                            || refresh_auto_hide_context
                                                                .borrow()
                                                                .is_some()
                                                            || refresh_auto_hide_settings_window
                                                                .borrow()
                                                                .is_some()
                                                            || view
                                                                .keyboard_focus
                                                                .as_ref()
                                                                .is_some_and(|focus| {
                                                                    focus.is_focused(window)
                                                                })
                                                            || attention_hold
                                                            || owned_taskbar_resize_active()
                                                            || transition_pending;
                                                    let prior = *runtime.state.borrow();
                                                    let (next, effect) = reduce_auto_hide(
                                                        prior,
                                                        AutoHideInput {
                                                            enabled,
                                                            now_ms: auto_hide_epoch
                                                                .elapsed()
                                                                .as_millis()
                                                                as u64,
                                                            pointer: cursor,
                                                            visibility_hold,
                                                            endpoints,
                                                        },
                                                    );
                                                    let endpoint = match effect {
                                                        AutoHideEffect::Show => Some(endpoints.visible),
                                                        AutoHideEffect::Hide => Some(endpoints.hidden),
                                                        AutoHideEffect::NoChange => None,
                                                    };
                                                    let applied = endpoint.is_none_or(|rect| {
                                                        hwnd(window).is_ok_and(|raw| {
                                                            move_owned_taskbar_client(
                                                                raw,
                                                                rect.left,
                                                                rect.top,
                                                                rect.width(),
                                                                rect.height(),
                                                            )
                                                            .is_ok()
                                                        })
                                                    });
                                                    if applied {
                                                        *runtime.state.borrow_mut() = next;
                                                        if effect == AutoHideEffect::Show {
                                                            trace_action("taskbar:auto-hide-shown");
                                                        } else if effect == AutoHideEffect::Hide {
                                                            trace_action("taskbar:auto-hide-hidden");
                                                        }
                                                    } else {
                                                        trace_action("taskbar:auto-hide-endpoint-rejected");
                                                    }
                                                    if was_enabled != enabled {
                                                        let mut transition_ok = !shell;
                                                        if shell {
                                                            if let Ok(raw) = hwnd(window) {
                                                                let mut leases = refresh_auto_hide_leases
                                                                    .borrow_mut();
                                                                if let Some(lease) = leases
                                                                    .iter_mut()
                                                                    .find(|lease| lease.owns_window(raw))
                                                                {
                                                                    transition_ok = if enabled {
                                                                        match lease.remove_appbar() {
                                                                            Ok(_) => {
                                                                                trace_action("taskbar:auto-hide-appbar-removed");
                                                                                true
                                                                            }
                                                                            Err(_) => {
                                                                                trace_action("taskbar:auto-hide-appbar-remove-rejected");
                                                                                false
                                                                            }
                                                                        }
                                                                    } else {
                                                                        match lease.register_appbar() {
                                                                            Err(_) => {
                                                                                trace_action("taskbar:auto-hide-owned-workarea-restored");
                                                                                true
                                                                            }
                                                                            Ok(()) if lease
                                                                                .reserve_bottom(
                                                                                    ScreenRect {
                                                                                        left: runtime.monitor.bounds.left,
                                                                                        top: runtime.monitor.bounds.top,
                                                                                        right: runtime.monitor.bounds.right,
                                                                                        bottom: runtime.monitor.bounds.bottom,
                                                                                    },
                                                                                    taskbar_height,
                                                                                )
                                                                                .is_ok() =>
                                                                            {
                                                                                trace_action("taskbar:auto-hide-appbar-restored");
                                                                                true
                                                                            }
                                                                            Ok(()) => {
                                                                                let _ = lease.remove_appbar();
                                                                                trace_action("taskbar:auto-hide-appbar-restore-rejected");
                                                                                false
                                                                            }
                                                                        }
                                                                    };
                                                                } else {
                                                                    trace_action("taskbar:auto-hide-lease-missing");
                                                                }
                                                            } else {
                                                                trace_action("taskbar:auto-hide-hwnd-rejected");
                                                            }
                                                        }
                                                        if transition_ok {
                                                            *runtime.enabled.borrow_mut() = enabled;
                                                        }
                                                    }
                                                }
                                            }
                                            let mut settings_changed = false;
                                            let mut row_geometry_ready =
                                                view.layout.rows.get() == taskbar_settings.rows;
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
                                            if view.locked != taskbar_settings.locked {
                                                view.locked = taskbar_settings.locked;
                                                if let Ok(raw) = hwnd(window) {
                                                    trace_action(
                                                        if platform_win::common::taskbar::set_owned_taskbar_resizable(
                                                            raw,
                                                            !taskbar_settings.locked,
                                                        )
                                                        .is_ok()
                                                        {
                                                            "taskbar:lock-style-synced"
                                                        } else {
                                                            "taskbar:lock-style-rejected"
                                                        },
                                                    );
                                                }
                                                if let Some(resize_rows) = view
                                                    .callbacks
                                                    .as_ref()
                                                    .map(|callbacks| {
                                                        Rc::clone(&callbacks.resize_rows)
                                                    })
                                                {
                                                    resize_rows(
                                                        taskbar_settings.rows,
                                                        window,
                                                        cx,
                                                    );
                                                }
                                                settings_changed = true;
                                            }
                                            if view.layout.rows.get() != taskbar_settings.rows {
                                                if let Some(resize_rows) = view
                                                    .callbacks
                                                    .as_ref()
                                                    .map(|callbacks| {
                                                        Rc::clone(&callbacks.resize_rows)
                                                    })
                                                {
                                                    row_geometry_ready = resize_rows(
                                                        taskbar_settings.rows,
                                                        window,
                                                        cx,
                                                    );
                                                }
                                                settings_changed |= row_geometry_ready;
                                            }
                                            if settings_changed || view.tasks != tasks {
                                                let running = tasks
                                                    .iter()
                                                    .map(|task| task.stable_id.clone())
                                                    .collect::<Vec<_>>();
                                                view.layout = TaskbarLayout::calculate(
                                                    if row_geometry_ready {
                                                        taskbar_settings.rows
                                                    } else {
                                                        view.layout.rows.get()
                                                    },
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
                                            let current_notifications =
                                                refresh_notification_snapshot.borrow().clone();
                                            if view.reconcile(
                                                current_system_snapshot.clone(),
                                                current_status.clone(),
                                                current_notifications,
                                            ) {
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
                            }) {
                                report_error("taskbar:refresh-update", error);
                            }
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
                let auto_hide_for_timer = taskbar_auto_hide.clone();
                let auto_hide_config_for_timer = Rc::clone(&persisted_settings);
                let provider_stop_for_timer = Arc::clone(&provider_refresh_stop);
                let provider_worker_for_timer = Rc::clone(&provider_refresh_worker);
                foreground
                    .spawn(async move {
                        background.timer(duration).await;
                        let mut retry_count = 0_u8;
                        loop {
                            match async_app.try_update(|app| {
                            provider_stop_for_timer.store(true, Ordering::Release);
                            if let Some(worker) = provider_worker_for_timer.borrow_mut().take() {
                                let _ = worker.join();
                            }
                            let rows = auto_hide_config_for_timer.borrow().taskbar.rows;
                            for (index, handle) in taskbar_handles.iter().enumerate() {
                                let Some(runtime) = auto_hide_for_timer.get(index) else {
                                    continue;
                                };
                                runtime.stop_reveal_worker.store(true, Ordering::Release);
                                if let Some(worker) = runtime.reveal_worker.borrow_mut().take() {
                                    let _ = worker.join();
                                }
                                let geometry =
                                    taskbar_physical_geometry(&runtime.monitor, shell, rows);
                                let _ = handle.update(app, |_, window, _| {
                                    if let Ok(raw) = hwnd(window)
                                        && move_owned_taskbar_client(
                                            raw,
                                            geometry.left,
                                            geometry.top,
                                            geometry.width,
                                            geometry.height,
                                        )
                                        .is_ok()
                                    {
                                        trace_action("taskbar:auto-hide-teardown-visible");
                                    }
                                });
                                *runtime.state.borrow_mut() = AutoHideState::Visible;
                            }
                            for lease in leases_for_timer.borrow_mut().iter_mut() {
                                lease.teardown();
                            }
                            for handle in &desktop_handles {
                                let _ = handle.update(app, |_, window, _| window.remove_window());
                            }
                            for handle in &taskbar_handles {
                                let _ = handle.update(app, |_, window, _| window.remove_window());
                            }
                            for slot in &system_flyout_windows {
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
                            }) {
                                Ok(()) => break,
                                Err(error) if retry_count < 20 => {
                                    retry_count = retry_count.saturating_add(1);
                                    report_error("shutdown:async-update-retry", error);
                                    background.timer(Duration::from_millis(10)).await;
                                }
                                Err(error) => {
                                    report_error("shutdown:async-update-rejected", error);
                                    break;
                                }
                            }
                        }
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
        ICON_CACHE_LIMIT, MonitorRecord, SystemStatusCommandFailure, apply_taskbar_context_setting,
        execute_system_status_command, prune_icon_cache, reconcile_desktop_item_positions,
        start_window_geometry, taskbar_physical_geometry,
    };
    use crate::status_client::StatusReconciler;
    use desktop_ui::AccessibleNode;
    use gpui::{WindowBounds, point, px};
    use platform_win::common::appbar_shell_hook::OwnedShellHookEvent;
    use platform_win::common::monitor_dpi_start::ScreenRect;
    use settings_store::{TaskbarAlignment, TaskbarSearchMode, TaskbarSettings};
    use shell_provider_protocol::{
        AudioStatus, ClockCalendarStatus, InputProfile, InputProfileKind, InputStatus,
        NetworkStatus, PowerStatus, StatusAvailability, SystemStatusCommand,
        SystemStatusCommandRequest, SystemStatusCommandTerminal, SystemStatusHostRequest,
        SystemStatusHostResponse, SystemStatusSnapshot, SystemStatusTerminalKind,
    };
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use std::path::Path;
    use std::time::Instant;
    use taskbar_ui::TaskbarContextCommand;
    use taskbar_ui::WindowsGuiMetrics;

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
            && !production.contains("StartMenuExperienceHost")
            && !production.contains("SearchHost")
            && !production.contains("ShellExperienceHost")
            && !production.contains("explorer.exe")
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

    fn status_snapshot(host_generation: u64, snapshot_generation: u64) -> SystemStatusSnapshot {
        SystemStatusSnapshot {
            host_generation,
            snapshot_generation,
            network: StatusAvailability::Available(NetworkStatus {
                connected: true,
                internet: true,
                display_name: "fixture".into(),
                wifi: StatusAvailability::Unavailable {
                    reason: "fixture".into(),
                },
            }),
            audio: StatusAvailability::Available(AudioStatus {
                endpoint_id: "fixture-audio".into(),
                volume_percent: 40,
                muted: false,
            }),
            power: StatusAvailability::Available(PowerStatus {
                ac_online: true,
                charging: false,
                battery_percent: None,
            }),
            clock: StatusAvailability::Available(ClockCalendarStatus {
                unix_ms: 1,
                locale: "en-US".into(),
                time_zone: "UTC".into(),
            }),
            input: StatusAvailability::Available(InputStatus {
                active_profile_id: "input".into(),
                profiles: vec![InputProfile {
                    id: "input".into(),
                    language_tag: "en-US".into(),
                    display_name: "English".into(),
                    input_method_name: "US keyboard".into(),
                    kind: InputProfileKind::LegacyKeyboardLayout,
                    language_id: 0x0409,
                    tsf_class_id: None,
                    tsf_profile_id: None,
                    hkl: None,
                }],
            }),
            overflowed: false,
        }
    }

    fn status_terminal(
        correlation_id: &str,
        host_generation: u64,
        terminal: SystemStatusTerminalKind,
    ) -> SystemStatusHostResponse {
        SystemStatusHostResponse::Terminal(SystemStatusCommandTerminal {
            correlation_id: correlation_id.into(),
            host_generation,
            observed_snapshot_generation: (terminal == SystemStatusTerminalKind::Observed)
                .then_some(2),
            terminal,
            message: String::new(),
        })
    }

    fn run_status_script(
        command: SystemStatusCommand,
        responses: Vec<Result<SystemStatusHostResponse, &'static str>>,
        now_values: Vec<u64>,
    ) -> (
        Result<super::SystemStatusCommandExecution, SystemStatusCommandFailure>,
        Vec<SystemStatusHostRequest>,
        StatusReconciler,
    ) {
        run_status_script_from(1, command, responses, now_values)
    }

    fn run_status_script_from(
        initial_host_generation: u64,
        command: SystemStatusCommand,
        responses: Vec<Result<SystemStatusHostResponse, &'static str>>,
        now_values: Vec<u64>,
    ) -> (
        Result<super::SystemStatusCommandExecution, SystemStatusCommandFailure>,
        Vec<SystemStatusHostRequest>,
        StatusReconciler,
    ) {
        let mut reconciler = StatusReconciler::default();
        assert!(
            reconciler.apply(SystemStatusHostResponse::Snapshot(status_snapshot(
                initial_host_generation,
                1,
            )))
        );
        let mut responses = VecDeque::from(responses);
        let mut requests = Vec::new();
        let mut now_values = VecDeque::from(now_values);
        let result = execute_system_status_command(
            command,
            std::time::Duration::from_millis(1_000),
            &mut reconciler,
            |request, _| {
                requests.push(request.clone());
                responses.pop_front().expect("scripted response")
            },
            || now_values.pop_front().expect("scripted clock"),
        );
        assert!(responses.is_empty(), "unused scripted responses");
        (result, requests, reconciler)
    }

    #[test]
    fn status_command_ordinary_success_and_provider_failure_are_not_retried() {
        for terminal in [
            SystemStatusTerminalKind::Observed,
            SystemStatusTerminalKind::ProviderFailure,
        ] {
            let (result, requests, reconciler) = run_status_script(
                SystemStatusCommand::SetVolume { volume_percent: 44 },
                vec![
                    Ok(status_terminal("host", 1, terminal)),
                    Ok(SystemStatusHostResponse::Snapshot(status_snapshot(1, 3))),
                ],
                vec![100],
            );
            let execution = result.expect("terminal execution");
            assert!(!execution.recovered_stale_generation);
            assert_eq!(requests.len(), 2);
            assert_eq!(reconciler.snapshot().unwrap().snapshot_generation, 3);
        }
    }

    #[test]
    fn command_host_generation_is_used_when_observer_lineage_is_numerically_newer() {
        let (result, requests, reconciler) = run_status_script_from(
            20,
            SystemStatusCommand::SetVolume { volume_percent: 55 },
            vec![
                Ok(status_terminal(
                    "observer-generation-rejected",
                    10,
                    SystemStatusTerminalKind::StaleGeneration,
                )),
                Ok(SystemStatusHostResponse::Snapshot(status_snapshot(10, 4))),
                Ok(status_terminal(
                    "command-host-observed",
                    10,
                    SystemStatusTerminalKind::Observed,
                )),
                Ok(SystemStatusHostResponse::Snapshot(status_snapshot(10, 6))),
            ],
            vec![100, 200],
        );
        assert!(
            result
                .expect("command-host retry")
                .recovered_stale_generation
        );
        let retry = requests
            .iter()
            .filter_map(|request| match request {
                SystemStatusHostRequest::Command { request } => Some(request),
                _ => None,
            })
            .nth(1)
            .expect("retry request");
        assert_eq!(retry.expected_host_generation, 10);
        assert_eq!(reconciler.snapshot().unwrap().host_generation, 20);
    }

    #[test]
    fn volume_and_mute_resynchronize_once_then_complete_with_fresh_identity() {
        for command in [
            SystemStatusCommand::SetVolume { volume_percent: 52 },
            SystemStatusCommand::SetMute { muted: true },
        ] {
            let (result, requests, reconciler) = run_status_script(
                command.clone(),
                vec![
                    Ok(status_terminal(
                        "stale",
                        2,
                        SystemStatusTerminalKind::StaleGeneration,
                    )),
                    Ok(SystemStatusHostResponse::Snapshot(status_snapshot(2, 1))),
                    Ok(status_terminal(
                        "observed",
                        2,
                        SystemStatusTerminalKind::Observed,
                    )),
                    Ok(SystemStatusHostResponse::Snapshot(status_snapshot(2, 3))),
                ],
                vec![1_000, 2_000],
            );
            let execution = result.expect("recovered execution");
            assert!(execution.recovered_stale_generation);
            assert_eq!(requests.len(), 4);
            let commands = requests
                .iter()
                .filter_map(|request| match request {
                    SystemStatusHostRequest::Command { request } => Some(request),
                    _ => None,
                })
                .collect::<Vec<&SystemStatusCommandRequest>>();
            assert_eq!(commands.len(), 2);
            assert_eq!(commands[0].expected_host_generation, 1);
            assert_eq!(commands[1].expected_host_generation, 2);
            assert_ne!(commands[0].correlation_id, commands[1].correlation_id);
            assert_eq!(commands[0].deadline_unix_ms, 2_000);
            assert_eq!(commands[1].deadline_unix_ms, 3_000);
            assert_eq!(commands[0].command, command);
            assert_eq!(commands[1].command, command);
            assert_eq!(reconciler.snapshot().unwrap().host_generation, 2);
            assert_eq!(reconciler.snapshot().unwrap().snapshot_generation, 3);
        }
    }

    #[test]
    fn second_stale_terminal_stops_at_two_attempts() {
        let (result, requests, _) = run_status_script(
            SystemStatusCommand::SetVolume { volume_percent: 60 },
            vec![
                Ok(status_terminal(
                    "stale-one",
                    2,
                    SystemStatusTerminalKind::StaleGeneration,
                )),
                Ok(SystemStatusHostResponse::Snapshot(status_snapshot(2, 1))),
                Ok(status_terminal(
                    "stale-two",
                    3,
                    SystemStatusTerminalKind::StaleGeneration,
                )),
                Ok(SystemStatusHostResponse::Snapshot(status_snapshot(3, 1))),
            ],
            vec![10, 20],
        );
        let execution = result.expect("bounded final terminal");
        let SystemStatusHostResponse::Terminal(terminal) = execution.response else {
            panic!("terminal response")
        };
        assert_eq!(terminal.terminal, SystemStatusTerminalKind::StaleGeneration);
        assert_eq!(requests.len(), 4);
        assert_eq!(
            requests
                .iter()
                .filter(|request| matches!(request, SystemStatusHostRequest::Command { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn invalid_command_response_and_failed_resync_never_replay() {
        let (invalid, requests, _) = run_status_script(
            SystemStatusCommand::SetMute { muted: true },
            vec![Ok(SystemStatusHostResponse::Snapshot(status_snapshot(
                1, 2,
            )))],
            vec![1],
        );
        assert_eq!(invalid, Err(SystemStatusCommandFailure::InvalidResponse));
        assert_eq!(requests.len(), 1);

        let (resync, requests, _) = run_status_script(
            SystemStatusCommand::SetMute { muted: true },
            vec![
                Ok(status_terminal(
                    "stale",
                    2,
                    SystemStatusTerminalKind::StaleGeneration,
                )),
                Err("fixture-disconnected"),
            ],
            vec![1],
        );
        assert_eq!(
            resync,
            Err(SystemStatusCommandFailure::ResyncTransport(
                "fixture-disconnected"
            ))
        );
        assert_eq!(requests.len(), 2);
    }

    #[test]
    fn win_e_is_shell_scoped_and_routes_only_to_verified_superexplorer_foreground() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "if shell {",
            "shell_hotkey::ShellHotkeys::start()",
            "request_superexplorer_foreground()",
            "focus_superexplorer_window(",
            "WindowAction::RestoreAndActivate",
            "build_default_launch(&resolved)",
            "Some(pending.process_id)",
            "win-e:focus-timeout",
            "ShellHotkeyAction::ShowDesktop",
            "ShellHotkeyAction::CycleInput",
            "ShellHotkeyAction::OpenTaskView",
            "ShellHotkeyAction::OpenNetworkPower",
            "ShellHotkeyAction::OpenNotifications",
            "ShellHotkeyAction::OpenScreenSnip",
            "ShellHotkeyAction::ToggleStart",
            "open_screen_snipping_overlay()",
            "shell-hotkey:screen-snip-requested",
            "shell-hotkey:screen-snip-accepted",
            "superdesktop-screen-snip",
            "adjacent_input_profile_id",
        ] {
            assert!(
                production.contains(required),
                "missing Win+E route: {required}"
            );
        }
        for forbidden in [
            "explorer.exe /e",
            "ShellExecuteW",
            "keybd_event",
            "SendInput",
        ] {
            assert!(
                !production.contains(forbidden),
                "delegated Win+E route: {forbidden}"
            );
        }
    }

    #[test]
    fn show_desktop_product_path_is_exact_identity_owned_and_never_delegates() {
        let source = include_str!("surface_runtime.rs");
        let implementation = source
            .split("fn run_show_desktop_cycle")
            .nth(1)
            .and_then(|tail| tail.split("fn task_hwnd").next())
            .expect("show desktop implementation source");
        for required in [
            "snapshot_task_windows()",
            "minimized_window_shelf.borrow().task_windows(windows)",
            "ShowDesktopPlan::Minimize",
            "ShowDesktopPlan::Restore",
            "apply_window_action_to_owned_identity",
            "WindowAction::Minimize",
            "WindowAction::Restore",
            "complete_minimize",
            "complete_restore",
        ] {
            assert!(
                implementation.contains(required),
                "missing owned route: {required}"
            );
        }
        for forbidden in [
            "explorer.exe",
            "Shell_TrayWnd",
            "ShellExperienceHost",
            "SystemSettings",
            "Win+D",
            "keybd_event",
            "SendInput",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "forbidden show desktop delegation: {forbidden}"
            );
        }
    }

    #[test]
    fn owned_shell_shelves_minimized_windows_before_grouping_and_retains_task_models() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        let visible = production
            .split("fn visible_tasks(")
            .nth(1)
            .and_then(|tail| tail.split("fn verification_state_tasks").next())
            .expect("visible task composition");
        let reconcile = visible
            .find("reconcile_minimized_window_shelf(shelf, &windows)")
            .expect("shelf reconciliation");
        let grouping = visible.find("let mut grouped").expect("task grouping");
        assert!(reconcile < grouping);
        for required in [
            "window.visible && !window.tool_window && !window.cloaked && !window.owned_transient",
            "minimized: windows.iter().all(|window| window.minimized)",
            "shell.then_some(&minimized_window_shelf)",
            "shell.then_some(&refresh_minimized_window_shelf)",
            "task:minimized-shelved",
            "report_error(\n            \"task:minimized-shelf\"",
            "action == platform_win::common::taskbar::WindowAction::Minimize",
            "reconcile_minimized_window_shelf_snapshot(minimized_window_shelf)",
            "shelf.borrow().task_windows(windows)",
        ] {
            assert!(
                production.contains(required),
                "missing shelf route: {required}"
            );
        }
        assert!(!visible.contains("SW_HIDE"));
    }

    #[test]
    fn taskbar_context_settings_mutate_exactly_one_supported_field() {
        let original = TaskbarSettings::default();
        let mut search = original.clone();
        assert!(apply_taskbar_context_setting(
            &mut search,
            TaskbarContextCommand::CycleSearchMode
        ));
        assert_eq!(search.search_mode, TaskbarSearchMode::Icon);
        assert_eq!(search.show_task_view, original.show_task_view);
        assert_eq!(search.locked, original.locked);

        let mut task_view = original.clone();
        assert!(apply_taskbar_context_setting(
            &mut task_view,
            TaskbarContextCommand::ToggleTaskView
        ));
        assert!(!task_view.show_task_view);
        assert_eq!(task_view.search_mode, original.search_mode);
        assert_eq!(task_view.locked, original.locked);

        let mut lock = original.clone();
        assert!(apply_taskbar_context_setting(
            &mut lock,
            TaskbarContextCommand::ToggleLockTaskbar
        ));
        assert!(!lock.locked);
        assert_eq!(lock.search_mode, original.search_mode);
        assert_eq!(lock.show_task_view, original.show_task_view);

        let mut non_setting = original.clone();
        assert!(!apply_taskbar_context_setting(
            &mut non_setting,
            TaskbarContextCommand::ShowDesktop
        ));
        assert_eq!(non_setting, original);
    }

    #[test]
    fn task_hover_preview_path_is_owned_delayed_and_exact_identity_resolved() {
        let source = include_str!("surface_runtime.rs");
        let implementation = source
            .split("fn preview_cards")
            .nth(1)
            .and_then(|tail| tail.split("fn progress_for_task").next())
            .expect("hover preview implementation source");
        for required in [
            "snapshot_task_windows()",
            "admit_live_preview",
            "HOVER_PREVIEW_DELAY_MS",
            "HOVER_PREVIEW_CLOSE_GRACE_MS",
            "can_open",
            "can_close",
            "TaskFlyoutView::new",
        ] {
            assert!(
                implementation.contains(required),
                "missing hover preview route: {required}"
            );
        }
        for forbidden in ["explorer.exe", "Shell_TrayWnd", "SendInput", "keybd_event"] {
            assert!(
                !implementation.contains(forbidden),
                "forbidden hover delegation: {forbidden}"
            );
        }
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
        for forbidden in [
            "StartMenuExperienceHost",
            "SearchHost",
            "ShellExperienceHost",
            "explorer.exe",
        ] {
            assert!(!production.contains(forbidden));
        }
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
        assert!(source.contains("second: local.second"));
        assert!(source.contains("taskbar_clock_locale()"));
        assert!(source.contains("SUPERDESKTOP_VERIFICATION_TASKBAR_ROWS"));
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
            3
        );
        assert!(production.contains("superdesktop-volume-command"));
        assert!(production.contains("status:volume-coalesced"));
        assert!(production.contains("superdesktop-provider-refresh"));
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
    fn task_icon_source_edge_covers_maximum_monitor_dpi_and_clamps() {
        let monitor = |dpi_x, dpi_y| MonitorRecord {
            device_name: format!("dpi-{dpi_x}-{dpi_y}"),
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
                bottom: 1040,
            },
            dpi_x,
            dpi_y,
        };
        assert_eq!(super::task_icon_source_edge(&[]), 32);
        assert_eq!(super::task_icon_source_edge(&[monitor(0, 0)]), 32);
        assert_eq!(super::task_icon_source_edge(&[monitor(96, 96)]), 32);
        assert_eq!(super::task_icon_source_edge(&[monitor(144, 144)]), 36);
        assert_eq!(super::task_icon_source_edge(&[monitor(192, 192)]), 48);
        assert_eq!(
            super::task_icon_source_edge(&[monitor(96, 96), monitor(288, 240)]),
            64
        );
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
    fn start_geometry_defaults_left_centers_on_request_and_preserves_bottom_gap() {
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
        let left_geometry = start_window_geometry(&monitor, false, 1, TaskbarAlignment::Left);
        let center_geometry = start_window_geometry(&monitor, false, 1, TaskbarAlignment::Center);
        let logical_width = 1920.0 / 1.75;
        let logical_bottom = 1000.0 / 1.75 - 40.0;
        assert_eq!(left_geometry.width, 640.0);
        assert!((left_geometry.left - 12.0).abs() < 0.01);
        assert!((center_geometry.left - (logical_width - 640.0) / 2.0).abs() < 0.01);
        assert!((logical_bottom - left_geometry.top - left_geometry.height - 12.0).abs() < 0.01);
        assert_eq!(left_geometry.top, center_geometry.top);
        assert_eq!(left_geometry.height, center_geometry.height);

        let mut small = monitor;
        small.work_area.right = 500;
        small.work_area.bottom = 400;
        small.dpi_x = 96;
        small.dpi_y = 96;
        let geometry = start_window_geometry(&small, false, 1, TaskbarAlignment::Left);
        assert_eq!((geometry.left, geometry.top), (12.0, 0.0));
        assert_eq!((geometry.width, geometry.height), (476.0, 348.0));
    }

    #[test]
    fn start_geometry_uses_matching_preview_and_shell_taskbar_anchors() {
        let monitor = MonitorRecord {
            device_name: "stale-work-area".into(),
            primary: true,
            bounds: ScreenRect {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2160,
            },
            work_area: ScreenRect {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2020,
            },
            dpi_x: 168,
            dpi_y: 168,
        };
        let preview = start_window_geometry(&monitor, false, 2, TaskbarAlignment::Left);
        let shell = start_window_geometry(&monitor, true, 2, TaskbarAlignment::Left);
        let preview_bottom = 2020.0 / 1.75 - 80.0 - 12.0;
        let shell_bottom = 2160.0 / 1.75 - 80.0 - 12.0;
        assert!((preview.top + preview.height - preview_bottom).abs() < 0.01);
        assert!((shell.top + shell.height - shell_bottom).abs() < 0.01);
        assert!((shell.top - preview.top - 80.0).abs() < 0.01);
    }

    #[test]
    fn start_geometry_matrix_preserves_windows_ratios_without_taskbar_overlap() {
        for dpi in [96, 144, 168, 216] {
            let monitor = MonitorRecord {
                device_name: format!("dpi-{dpi}"),
                primary: false,
                bounds: ScreenRect {
                    left: -3840,
                    top: -300,
                    right: 0,
                    bottom: 2160,
                },
                work_area: ScreenRect {
                    left: -3840,
                    top: -300,
                    right: 0,
                    bottom: 2040,
                },
                dpi_x: dpi,
                dpi_y: dpi,
            };
            let scale = dpi as f32 / 96.0;
            let work_left = -3840.0 / scale;
            let work_top = -300.0 / scale;
            let work_width = 3840.0 / scale;
            for shell in [false, true] {
                for rows in 1..=3 {
                    for alignment in [TaskbarAlignment::Left, TaskbarAlignment::Center] {
                        let geometry = start_window_geometry(&monitor, shell, rows, alignment);
                        let taskbar_bottom = if shell { 2160.0 } else { 2040.0 } / scale;
                        let start_bottom = taskbar_bottom - 40.0 * rows as f32 - 12.0;
                        assert_eq!(geometry.width, 640.0_f32.min(work_width - 24.0));
                        assert_eq!(geometry.height, 720.0_f32.min(start_bottom - work_top));
                        let expected_left = match alignment {
                            TaskbarAlignment::Left => work_left + 12.0,
                            TaskbarAlignment::Center => {
                                work_left + (work_width - geometry.width) / 2.0
                            }
                        };
                        assert!((geometry.left - expected_left).abs() < 0.01);
                        assert!(geometry.top >= work_top);
                        assert!((geometry.top + geometry.height - start_bottom).abs() < 0.01);
                    }
                }
            }
        }
    }

    #[test]
    fn start_geometry_clamps_both_alignments_inside_extremely_narrow_work_area() {
        let monitor = MonitorRecord {
            device_name: "narrow-offset".into(),
            primary: false,
            bounds: ScreenRect {
                left: 300,
                top: 100,
                right: 310,
                bottom: 500,
            },
            work_area: ScreenRect {
                left: 300,
                top: 100,
                right: 310,
                bottom: 460,
            },
            dpi_x: 96,
            dpi_y: 96,
        };
        for alignment in [TaskbarAlignment::Left, TaskbarAlignment::Center] {
            let geometry = start_window_geometry(&monitor, false, 1, alignment);
            assert_eq!(geometry.width, 1.0);
            assert!(geometry.left >= 300.0);
            assert!(geometry.left + geometry.width <= 310.0);
        }
    }

    #[test]
    fn taskbar_preview_and_shell_keep_distinct_bottom_anchors_at_175_percent() {
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
            dpi_x: 168,
            dpi_y: 168,
        };
        let preview = taskbar_physical_geometry(&monitor, false, 2);
        assert_eq!(
            (preview.bottom, preview.height, preview.top),
            (1040, 140, 900)
        );
        let shell = taskbar_physical_geometry(&monitor, true, 3);
        assert_eq!((shell.bottom, shell.height, shell.top), (1080, 210, 870));
        assert_eq!((shell.left, shell.width), (-1920, 1920));
        assert_eq!(taskbar_physical_geometry(&monitor, true, 0).height, 70);
        assert_eq!(taskbar_physical_geometry(&monitor, true, 99).height, 210);
    }

    #[test]
    fn preview_taskbar_reanchors_when_work_area_expands_after_explorer_exit() {
        let mut monitor = MonitorRecord {
            device_name: "primary".into(),
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
                bottom: 1040,
            },
            dpi_x: 96,
            dpi_y: 96,
        };
        let before = taskbar_physical_geometry(&monitor, false, 1);
        monitor.work_area.bottom = monitor.bounds.bottom;
        let after = taskbar_physical_geometry(&monitor, false, 1);
        assert_eq!(before.bottom, 1040);
        assert_eq!(after.bottom, 1080);
        assert_eq!(after.top, 1040);
        assert!(!super::taskbar_uses_monitor_bounds(false, true));
        assert!(super::taskbar_uses_monitor_bounds(false, false));
        assert!(super::taskbar_uses_monitor_bounds(true, true));
    }

    #[test]
    fn popup_slots_end_read_borrows_before_mutation_and_start_panics_are_contained() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "guard_ui_action(\"start\"",
            "let existing_start = *start_window_for_taskbar.borrow()",
            "let existing_task_view = *task_view_window_for_taskbar.borrow()",
            "let existing_jump_list = *jump_list_window_for_taskbar.borrow()",
            "let existing_context = *context_window_for_taskbar.borrow()",
            "let existing_settings = *settings_slot_for_action.borrow()",
            "let start_alignment = {",
            "start_persisted_settings.borrow().taskbar.alignment",
            "start_options(",
            "ShellHotkeyAction::OpenSearch",
            "ShellHotkeyAction::ToggleStart",
            "Rc::clone(&callbacks.start)",
            "shell-hotkey:start-toggle",
            "no live taskbar Start callback is available",
            "taskbar:monitor-geometry-reconciled",
            "\"taskbar:monitor-geometry\"",
        ] {
            assert!(
                production.contains(required),
                "missing lifecycle guard: {required}"
            );
        }
        for forbidden in [
            "if let Some(existing) = *start_window_for_taskbar.borrow()",
            "if let Some(existing) = *task_view_window_for_taskbar.borrow()",
            "if let Some(existing) = *jump_list_window_for_taskbar.borrow()",
            "if let Some(existing) = *context_window_for_taskbar.borrow()",
            "if let Some(existing) = *settings_slot_for_action.borrow()",
        ] {
            assert!(
                !production.contains(forbidden),
                "borrow can panic: {forbidden}"
            );
        }
    }

    #[test]
    fn task_context_cancels_preview_before_every_fallible_resolution_path() {
        let source = include_str!("surface_runtime.rs");
        let callback = source
            .split("task_context: Rc::new")
            .nth(1)
            .and_then(|tail| tail.split("taskbar_context: Rc::new").next())
            .expect("task context callback source");
        let cancel = callback
            .find("hover_preview_for_context.borrow_mut().cancel()")
            .expect("preview generation cancellation");
        let remove = callback
            .find("flyout_window_for_context.borrow_mut().take()")
            .expect("visible preview removal");
        let snapshot = callback
            .find("snapshot_task_windows()")
            .expect("task snapshot");
        let provider = callback
            .find("query_jump_list(")
            .expect("Jump List provider query");
        assert!(cancel < remove && remove < snapshot && snapshot < provider);
        assert!(callback.contains("active_preview_for_context.set(0)"));
    }

    #[test]
    fn async_runtime_updates_are_fallible_and_preserve_existing_gpui_api() {
        let gpui = include_str!("../../../vendor/gpui/src/app/async_context.rs");
        for required in [
            "pub fn try_update<R>",
            "app.try_borrow_mut()?",
            "bail!(\"app is quitting\")",
            "pub fn update<R>",
        ] {
            assert!(
                gpui.contains(required),
                "missing fallible GPUI API: {required}"
            );
        }

        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for forbidden in [
            "async_app.update(",
            "transfer_app.update(",
            "auto_hide_app.update(",
            "refresh_app.update(",
        ] {
            assert!(
                !production.contains(forbidden),
                "panic-prone async update remains: {forbidden}"
            );
        }
        for required in [
            "task-preview:close-update",
            "task-preview:pointer-monitor-update",
            "task-preview:open-update",
            "desktop:transfer-update",
            "taskbar:auto-hide-update",
            "taskbar:refresh-update",
            "shutdown:async-update-retry",
        ] {
            assert!(
                production.contains(required),
                "missing rejection trace: {required}"
            );
        }
    }

    #[test]
    fn every_independent_context_popup_uses_one_time_fail_closed_promotion() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        let helper = production
            .split("fn promote_owned_context_popup")
            .nth(1)
            .and_then(|tail| tail.split("fn guard_ui_action").next())
            .expect("context popup promotion helper");
        assert!(helper.contains("and_then(promote_owned_popup_topmost)"));
        assert!(helper.contains("window.remove_window()"));
        assert!(helper.contains("report_error"));
        for forbidden in ["loop {", "timer(", "sleep(", "spawn("] {
            assert!(
                !helper.contains(forbidden),
                "recurring promotion: {forbidden}"
            );
        }
        assert_eq!(
            production.matches("promote_owned_context_popup(").count(),
            4
        );
        for required in [
            "\"taskbar:jump-list\"",
            "\"taskbar:context\"",
            "\"status:context\"",
            "{kind}:topmost-established",
        ] {
            assert!(
                production.contains(required),
                "missing promoted route: {required}"
            );
        }
    }

    #[test]
    fn appbar_unavailable_is_degraded_geometry_not_terminal_failure() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        let fallback = production
            .split("if !appbar_available")
            .nth(1)
            .and_then(|tail| tail.split("} else if").next())
            .expect("AppBar fallback branch");
        assert!(fallback.contains("taskbar:appbar-unavailable-owned-shell"));
        assert!(fallback.contains("taskbar:appbar-fallback-geometry-active"));
        assert!(!fallback.contains("SuperDesktop warning [taskbar:appbar]"));
        assert!(!fallback.contains("eprintln!"));
        assert!(!fallback.contains("return"));
        assert!(!fallback.contains("app.quit"));
        assert!(production.contains("refresh_foreground"));
        assert!(production.contains("taskbar:refresh-update"));
    }

    #[test]
    fn explorer_aligned_local_commands_have_one_pin_and_one_final_close() {
        let single = super::taskbar_local_commands(false, Some(false), 1);
        let single_labels = single
            .iter()
            .map(|command| command.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            single_labels,
            ["Minimize", "Maximize", "Pin to taskbar", "Close window"]
        );
        assert_eq!(
            single
                .iter()
                .filter(|command| command.id.0.contains("close"))
                .count(),
            1
        );

        let grouped = super::taskbar_local_commands(true, Some(true), 2);
        let grouped_labels = grouped
            .iter()
            .map(|command| command.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            grouped_labels,
            [
                "Minimize",
                "Maximize",
                "Unpin from taskbar",
                "Close all windows"
            ]
        );
        assert!(!grouped[0].enabled);
        assert_eq!(
            grouped
                .iter()
                .filter(|command| command.id.0.contains("close"))
                .count(),
            1
        );

        let source = include_str!("surface_runtime.rs");
        assert!(source.contains("Err(_) => false"));
        assert!(source.contains("taskbar:jump-list-action-rejected"));
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
        let (left, top, width, height) =
            super::taskbar_context_placement(&monitor, false, 2, point(px(9_999.), px(0.)));
        assert!(left >= -1280.0);
        assert!(left + width <= 0.0);
        assert_eq!(top, 361.3333);
        assert_eq!(height, 244.0);
        let settings = super::taskbar_settings_options(&monitor);
        let Some(WindowBounds::Windowed(settings)) = settings.window_bounds else {
            panic!("bounds")
        };
        let logical_work_width = (monitor.work_area.right - monitor.work_area.left) as f32 / 1.5;
        let logical_work_height = (monitor.work_area.bottom - monitor.work_area.top) as f32 / 1.5;
        assert_eq!(settings.size.width.as_f32(), 1100.0);
        assert!((settings.size.height.as_f32() - logical_work_height).abs() < 0.01);
        assert!(settings.size.width.as_f32() <= logical_work_width);
        assert!(settings.size.height.as_f32() <= logical_work_height);
    }

    #[test]
    fn taskbar_settings_geometry_is_logical_once_across_dpi_and_small_monitors() {
        for dpi in [96, 120, 144, 168, 192, 216] {
            let monitor = MonitorRecord {
                device_name: format!("dpi-{dpi}"),
                primary: true,
                bounds: ScreenRect {
                    left: -1920,
                    top: 0,
                    right: 1920,
                    bottom: 2160,
                },
                work_area: ScreenRect {
                    left: -1920,
                    top: 0,
                    right: 1920,
                    bottom: 2040,
                },
                dpi_x: dpi,
                dpi_y: dpi,
            };
            let (_, _, width, height) = super::taskbar_settings_placement(&monitor);
            let scale = dpi as f32 / 96.0;
            assert!(width <= 3840.0 / scale);
            assert!(height <= 2040.0 / scale);
            assert!(width <= 1100.0);
            assert!(height <= 860.0);
        }
        let compact = MonitorRecord {
            device_name: "compact".into(),
            primary: false,
            bounds: ScreenRect {
                left: -500,
                top: -300,
                right: 0,
                bottom: 100,
            },
            work_area: ScreenRect {
                left: -500,
                top: -300,
                right: 0,
                bottom: 100,
            },
            dpi_x: 96,
            dpi_y: 96,
        };
        let (left, top, width, height) = super::taskbar_settings_placement(&compact);
        assert_eq!((left, top, width, height), (-500.0, -300.0, 500.0, 400.0));
        let source = include_str!("surface_runtime.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let options = source
            .split("fn taskbar_settings_options")
            .nth(1)
            .and_then(|tail| tail.split("fn taskbar_settings_placement").next())
            .unwrap();
        assert!(!options.contains("* scale"));
        assert!(options.contains("size(px(width), px(height))"));
    }

    #[test]
    fn notification_overflow_uses_logical_bounds_and_reserves_owned_taskbar() {
        let monitor = MonitorRecord {
            device_name: "fixture".into(),
            primary: false,
            bounds: ScreenRect {
                left: -1920,
                top: -200,
                right: 0,
                bottom: 1080,
            },
            work_area: ScreenRect {
                left: -1920,
                top: -200,
                right: 0,
                bottom: 1000,
            },
            dpi_x: 168,
            dpi_y: 168,
        };
        let bounds = super::notification_overflow_bounds(&monitor, false, 24, 2);
        assert_eq!(bounds.width, 344.0);
        assert_eq!(bounds.height, 216.0);
        assert!((bounds.left + bounds.width - (-8.0)).abs() < 0.01);
        assert!((bounds.top + bounds.height - (1000.0 / 1.75 - 88.0)).abs() < 0.01);
        assert!(bounds.left >= monitor.work_area.left as f32 / 1.75);
        assert!(bounds.top >= monitor.work_area.top as f32 / 1.75);

        let constrained = MonitorRecord {
            work_area: ScreenRect {
                left: -100,
                top: 0,
                right: 300,
                bottom: 220,
            },
            dpi_x: 480,
            dpi_y: 480,
            ..monitor
        };
        let bounds = super::notification_overflow_bounds(&constrained, false, usize::MAX, 3);
        assert!(bounds.width <= 80.0);
        assert!(bounds.height <= 1.0);
        assert!(bounds.left >= -20.0 && bounds.top >= 0.0);
        assert!(bounds.left + bounds.width <= 60.0);
        assert!(bounds.top + bounds.height <= 1.0);
    }

    #[test]
    fn notification_show_all_admits_empty_and_complete_snapshots_without_delegation() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "notification_overflow: Rc::new(move |nodes, app|",
            "notification_overflow_options(",
            "nodes.len()",
            "NotificationOverflowView::new",
            "notification:owned-overflow-opened",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        assert!(!production.contains("if nodes.is_empty()"));
        assert!(!production.contains("Shell_TrayWnd"));
    }

    #[test]
    fn notification_overflow_geometry_matrix_tracks_mode_rows_and_icon_grid() {
        for dpi in [96, 144, 168, 216] {
            let monitor = MonitorRecord {
                device_name: format!("dpi-{dpi}"),
                primary: false,
                bounds: ScreenRect {
                    left: -3840,
                    top: -300,
                    right: 0,
                    bottom: 2160,
                },
                work_area: ScreenRect {
                    left: -3840,
                    top: -300,
                    right: 0,
                    bottom: 2020,
                },
                dpi_x: dpi,
                dpi_y: dpi,
            };
            let scale = dpi as f32 / 96.0;
            for shell in [false, true] {
                for taskbar_rows in 1..=3 {
                    for (icons, grid_rows) in [(1, 1), (6, 1), (7, 2), (20, 4), (36, 6), (99, 6)] {
                        let geometry = super::notification_overflow_bounds(
                            &monitor,
                            shell,
                            icons,
                            taskbar_rows,
                        );
                        let taskbar_bottom = if shell { 2160.0 } else { 2020.0 } / scale;
                        let panel_bottom = taskbar_bottom - 40.0 * taskbar_rows as f32 - 8.0;
                        assert_eq!(geometry.width, 344.0);
                        assert_eq!(geometry.height, 24.0 + 48.0 * grid_rows as f32);
                        assert!((geometry.top + geometry.height - panel_bottom).abs() < 0.01);
                        assert!(geometry.left >= -3840.0 / scale);
                        assert!(geometry.left + geometry.width <= -8.0 + 0.01);
                        assert!(geometry.top >= -300.0 / scale);
                    }
                }
            }
        }
    }

    #[test]
    fn jump_list_geometry_is_source_anchored_content_sized_and_mode_aware() {
        let monitor = MonitorRecord {
            device_name: "fixture".into(),
            primary: false,
            bounds: ScreenRect {
                left: -3840,
                top: 0,
                right: 0,
                bottom: 2160,
            },
            work_area: ScreenRect {
                left: -3840,
                top: 0,
                right: 0,
                bottom: 2020,
            },
            dpi_x: 168,
            dpi_y: 168,
        };
        let scale = 1.75;
        let preview = super::jump_list_geometry(&monitor, false, 2, Some(-1920), 2, 1);
        let shell = super::jump_list_geometry(&monitor, true, 2, Some(-1920), 2, 1);
        assert_eq!((preview.width, preview.height), (360.0, 98.0));
        assert!((preview.left + preview.width / 2.0 - (-1920.0 / scale)).abs() < 0.01);
        assert!((shell.top - preview.top - 80.0).abs() < 0.01);

        for dpi in [96, 144, 168, 216] {
            let scaled = MonitorRecord {
                dpi_x: dpi,
                dpi_y: dpi,
                ..monitor.clone()
            };
            for shell in [false, true] {
                for rows in 1..=3 {
                    for (entries, groups) in [(0, 0), (2, 1), (8, 3), (100, 4)] {
                        let left = super::jump_list_geometry(
                            &scaled,
                            shell,
                            rows,
                            Some(-3839),
                            entries,
                            groups,
                        );
                        let right = super::jump_list_geometry(
                            &scaled,
                            shell,
                            rows,
                            Some(-1),
                            entries,
                            groups,
                        );
                        let fallback =
                            super::jump_list_geometry(&scaled, shell, rows, None, entries, groups);
                        assert!(left.left >= scaled.work_area.left as f32 / (dpi as f32 / 96.0));
                        assert!(right.left + right.width <= 0.01);
                        assert!(fallback.height <= 480.0 && fallback.height >= 1.0);
                        assert!(fallback.top >= 0.0);
                    }
                }
            }
        }
    }

    #[test]
    fn system_flyout_geometry_reserves_owned_taskbar_and_clamps_every_monitor_origin() {
        let reference = MonitorRecord {
            device_name: "reference".into(),
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
                bottom: 1080,
            },
            dpi_x: 168,
            dpi_y: 168,
        };
        let geometry = super::system_flyout_geometry(
            &reference,
            false,
            super::SystemFlyoutKind::Calendar,
            1,
            10,
            2,
        );
        let logical_right = 1920.0 / 1.75;
        let logical_bottom = 1080.0 / 1.75;
        assert_eq!(geometry.width, 380.0);
        assert!((geometry.height - (logical_bottom - 88.0)).abs() < 0.01);
        assert!((geometry.left + geometry.width - (logical_right - 8.0)).abs() < 0.01);
        assert!((geometry.top + geometry.height - (logical_bottom - 88.0)).abs() < 0.01);

        let negative = MonitorRecord {
            device_name: "negative".into(),
            primary: false,
            bounds: ScreenRect {
                left: -1920,
                top: -200,
                right: 0,
                bottom: 1080,
            },
            work_area: ScreenRect {
                left: -1920,
                top: -200,
                right: 0,
                bottom: 1080,
            },
            ..reference.clone()
        };
        let geometry = super::system_flyout_geometry(
            &negative,
            false,
            super::SystemFlyoutKind::Input,
            64,
            0,
            3,
        );
        assert!(geometry.left >= -1920.0 / 1.75);
        assert!(geometry.top >= -200.0 / 1.75);
        assert!(geometry.left + geometry.width <= -8.0);
        assert!(geometry.top + geometry.height <= 1080.0 / 1.75 - 128.0);

        let constrained = MonitorRecord {
            device_name: "constrained".into(),
            bounds: ScreenRect {
                left: -100,
                top: 0,
                right: 300,
                bottom: 220,
            },
            work_area: ScreenRect {
                left: -100,
                top: 0,
                right: 300,
                bottom: 220,
            },
            dpi_x: 480,
            dpi_y: 480,
            ..reference
        };
        let geometry = super::system_flyout_geometry(
            &constrained,
            false,
            super::SystemFlyoutKind::Calendar,
            1,
            100,
            3,
        );
        assert!(geometry.width >= 1.0 && geometry.height >= 1.0);
        assert!(geometry.left >= -20.0 && geometry.top >= 0.0);
        assert!(geometry.left + geometry.width <= 60.0);
        assert!(geometry.top + geometry.height <= 44.0);

        for dpi in [96, 120, 144, 168, 192, 216] {
            let monitor = MonitorRecord {
                device_name: format!("dpi-{dpi}"),
                primary: true,
                bounds: ScreenRect {
                    left: 0,
                    top: 0,
                    right: 3840,
                    bottom: 2160,
                },
                work_area: ScreenRect {
                    left: 0,
                    top: 0,
                    right: 3840,
                    bottom: 2160,
                },
                dpi_x: dpi,
                dpi_y: dpi,
            };
            let scale = dpi as f32 / 96.0;
            for rows in 1..=3 {
                let geometry = super::system_flyout_geometry(
                    &monitor,
                    false,
                    super::SystemFlyoutKind::Calendar,
                    1,
                    100,
                    rows,
                );
                assert!(geometry.left >= 0.0 && geometry.top >= 0.0);
                assert!(geometry.left + geometry.width <= 3840.0 / scale);
                assert!(geometry.top + geometry.height <= 2160.0 / scale);
                assert!(geometry.height <= 720.0);
            }
        }
    }

    #[test]
    fn system_flyout_geometry_uses_exact_preview_and_shell_taskbar_anchors() {
        let monitor = MonitorRecord {
            device_name: "stale-work-area".into(),
            primary: true,
            bounds: ScreenRect {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2160,
            },
            work_area: ScreenRect {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2020,
            },
            dpi_x: 168,
            dpi_y: 168,
        };
        let preview = super::system_flyout_geometry(
            &monitor,
            false,
            super::SystemFlyoutKind::Volume,
            1,
            0,
            2,
        );
        let shell =
            super::system_flyout_geometry(&monitor, true, super::SystemFlyoutKind::Volume, 1, 0, 2);
        let scale = 1.75;
        let preview_bottom = 2020.0 / scale - 80.0 - 8.0;
        let shell_bottom = 2160.0 / scale - 80.0 - 8.0;
        assert!((preview.top + preview.height - preview_bottom).abs() < 0.01);
        assert!((shell.top + shell.height - shell_bottom).abs() < 0.01);
        assert!((shell.top - preview.top - 80.0).abs() < 0.01);
    }

    #[test]
    fn system_flyout_geometry_matrix_preserves_windows_logical_ratios() {
        let kinds: [(super::SystemFlyoutKind, f32, f32); 4] = [
            (super::SystemFlyoutKind::Input, 360.0, 224.0),
            (super::SystemFlyoutKind::Volume, 360.0, 184.0),
            (super::SystemFlyoutKind::NetworkPower, 360.0, 640.0),
            (super::SystemFlyoutKind::Calendar, 380.0, 520.0),
        ];
        for dpi in [96, 144, 168, 216] {
            let monitor = MonitorRecord {
                device_name: format!("dpi-{dpi}"),
                primary: false,
                bounds: ScreenRect {
                    left: -3840,
                    top: -300,
                    right: 0,
                    bottom: 2160,
                },
                work_area: ScreenRect {
                    left: -3840,
                    top: -300,
                    right: 0,
                    bottom: 2040,
                },
                dpi_x: dpi,
                dpi_y: dpi,
            };
            let scale = dpi as f32 / 96.0;
            let work_left = -3840.0 / scale;
            let work_right = 0.0;
            let work_top = -300.0 / scale;
            for shell in [false, true] {
                for rows in 1..=3 {
                    let taskbar_bottom = if shell { 2160.0 } else { 2040.0 } / scale;
                    let popup_bottom = taskbar_bottom - 40.0 * rows as f32 - 8.0;
                    for (kind, preferred_width, preferred_height) in kinds {
                        let geometry =
                            super::system_flyout_geometry(&monitor, shell, kind, 2, 0, rows);
                        assert_eq!(geometry.width, preferred_width);
                        assert_eq!(
                            geometry.height,
                            preferred_height.min(popup_bottom - work_top)
                        );
                        assert!(geometry.left >= work_left);
                        assert!(geometry.left + geometry.width <= work_right + 0.01);
                        assert!(geometry.top >= work_top);
                        assert!((geometry.top + geometry.height - popup_bottom).abs() < 0.01);
                    }
                }
            }
        }
    }

    #[test]
    fn wifi_actions_map_to_exact_typed_commands_without_claiming_observation() {
        assert_eq!(
            super::system_status_command(taskbar_ui::SystemStatusAction::OpenLanguagePreferences),
            shell_provider_protocol::SystemStatusCommand::OpenLanguagePreferences
        );
        assert_eq!(
            super::system_status_command(taskbar_ui::SystemStatusAction::RefreshWifi),
            shell_provider_protocol::SystemStatusCommand::RefreshWifi
        );
        assert_eq!(
            super::system_status_command(taskbar_ui::SystemStatusAction::ConnectWifi {
                interface_id: "interface-1".into(),
                profile_name: "profile-1".into(),
            }),
            shell_provider_protocol::SystemStatusCommand::ConnectWifi {
                interface_id: "interface-1".into(),
                profile_name: "profile-1".into(),
            }
        );
        assert_eq!(
            super::system_status_command(taskbar_ui::SystemStatusAction::DisconnectWifi {
                interface_id: "interface-2".into(),
            }),
            shell_provider_protocol::SystemStatusCommand::DisconnectWifi {
                interface_id: "interface-2".into(),
            }
        );
    }

    #[test]
    fn taskbar_clock_locale_follows_configured_or_user_language_tag() {
        assert_eq!(
            super::clock_locale_from_tag(Some("zh-TW")),
            taskbar_ui::ClockLocale::ZhTw
        );
        assert_eq!(
            super::clock_locale_from_tag(Some("zh-Hant-TW")),
            taskbar_ui::ClockLocale::ZhTw
        );
        assert_eq!(
            super::clock_locale_from_tag(Some("en-US")),
            taskbar_ui::ClockLocale::En
        );
        assert_eq!(
            super::clock_locale_from_tag(None),
            taskbar_ui::ClockLocale::En
        );
    }

    #[test]
    fn system_flyout_composition_is_explicit_truthful_and_never_delegates() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "system_flyout_presentation()",
            "SystemFlyoutPresentation::new",
            "system_flyout_geometry(",
            "system_flyout_settings.borrow().taskbar.rows",
            "SystemFlyoutView::new",
            "status:owned-flyout-opened",
            "status:flyout-dismissed",
            "status:flyout-open-failed",
            "apply_system_status_action(",
            "NotificationCenterAction::Dismiss",
            "NotificationCenterAction::ClearAll",
            "dismiss_notification(",
            "clear_notifications(",
        ] {
            assert!(production.contains(required), "missing {required}");
        }
        for forbidden in [
            "explorer.exe",
            "Shell_TrayWnd",
            "StartMenuExperienceHost",
            "ShellExperienceHost",
            "QuickSettings",
            "SystemSettings",
            "ms-settings:",
            "ms-settings:",
        ] {
            assert!(!production.contains(forbidden), "delegated {forbidden}");
        }
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
            "TaskbarContextCommand::ToggleLockTaskbar",
            "TaskbarContextCommand::ReturnToDefaultExplorer",
            "recover_explorer_shell()",
            "explorer-return:verified",
            "taskbar:lock-toggled",
            "set_owned_taskbar_resizable",
            "attach_resize_observer",
            "taskbar_physical_geometry",
            "taskbar:resize-appbar-synced",
            "taskbar:appbar-unavailable-owned-shell",
            "taskbar:resize-owned-workarea-synced",
        ] {
            assert!(production.contains(token), "missing {token}");
        }
        assert!(!production.contains("ms-settings:taskbar"));
        assert!(!production.contains("Shell_TrayWnd"));
        for forbidden in [
            "explorer.exe",
            "StartMenuExperienceHost",
            "SearchHost",
            "ShellExperienceHost",
        ] {
            assert!(!production.contains(forbidden));
        }
        assert!(include_str!("../../taskbar-ui/src/view.rs").contains("cx.stop_propagation();"));
    }

    #[test]
    fn owned_auto_hide_source_retries_lifecycle_and_observes_preview_attention() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for token in [
            "taskbar-auto-hide",
            "physical_cursor_position()",
            "owned_taskbar_resize_active()",
            "transition_pending",
            "if transition_ok",
            "lease.remove_appbar()",
            "let _ = lease.remove_appbar();",
            "window.on_window_should_close",
            "taskbar:auto-hide-close-visible",
            "taskbar:preview-shell-hook-owned",
            "taskbar:auto-hide-teardown-visible",
            "superdesktop-auto-hide-reveal",
            "std::thread::sleep(Duration::from_millis(50))",
        ] {
            assert!(production.contains(token), "missing {token}");
        }
        assert!(!production.contains("Shell_TrayWnd"));
        assert!(!production.contains("StartMenuExperienceHost"));
    }

    #[test]
    fn task_preview_open_source_keeps_hover_passive_and_click_keyboard_focused() {
        assert!(!super::PreviewOpenSource::Hover.activates_window());
        assert!(!super::PreviewOpenSource::Hover.assigns_keyboard_focus());
        assert!(super::PreviewOpenSource::Click.activates_window());
        assert!(super::PreviewOpenSource::Click.assigns_keyboard_focus());

        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(production.contains("PreviewOpenSource::Hover"));
        assert!(production.contains("PreviewOpenSource::Click"));
        assert!(production.contains("if source.activates_window()"));
        assert!(production.contains("source.assigns_keyboard_focus()"));
        assert!(production.contains("promote_owned_popup_topmost(destination_hwnd)"));
        assert!(production.contains("task-preview:topmost-established"));
        assert!(production.contains("task-preview:topmost-rejected"));
        assert!(production.contains("if topmost_established.get()"));
        assert!(!production.contains("Shell_TrayWnd"));
        assert!(!production.contains("explorer.exe"));

        let view = include_str!("../../taskbar-ui/src/advanced.rs");
        assert!(view.contains("if self.keyboard_focus"));
        assert!(view.contains("window.focus(&self.focus, cx)"));
    }

    #[test]
    fn alt_tab_uses_owned_snapshot_dwm_previews_and_exact_foreground_action() {
        let source = include_str!("surface_runtime.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        for required in [
            "fn alt_tab_cards()",
            "snapshot_task_windows()",
            "AltTabView::new(",
            "ShellHotkeyAction::AltTabForward",
            "ShellHotkeyAction::AltTabCommit",
            "apply_flyout_action(action)",
            "promote_owned_popup_topmost(destination_hwnd)",
        ] {
            assert!(
                production.contains(required),
                "missing Alt+Tab route: {required}"
            );
        }
    }

    #[test]
    fn task_preview_geometry_anchors_clamps_and_scales_once() {
        for dpi in [96, 144, 168, 216] {
            let monitor = MonitorRecord {
                device_name: format!("dpi-{dpi}"),
                primary: false,
                bounds: ScreenRect {
                    left: -2560,
                    top: -200,
                    right: 0,
                    bottom: 1440,
                },
                work_area: ScreenRect {
                    left: -2560,
                    top: -200,
                    right: 0,
                    bottom: 1380,
                },
                dpi_x: dpi,
                dpi_y: dpi,
            };
            let scale = dpi as f32 / 96.0;
            let work_left = -2560.0 / scale;
            let work_right = 0.0;
            let work_top = -200.0 / scale;
            let explorer_taskbar_bottom = 1380.0 / scale;

            for cards in 1..=4 {
                let interior_anchor = -1280;
                let widths = vec![220; cards];
                let geometry =
                    super::task_flyout_geometry(&monitor, &widths, Some(interior_anchor), false, 1);
                assert!(geometry.left >= work_left + 8.0);
                assert!(geometry.left + geometry.width <= work_right - 8.0 + 0.01);
                assert!(geometry.top >= work_top + 8.0);
                let superdesktop_taskbar_top = explorer_taskbar_bottom - 40.0;
                assert!(
                    geometry.top + geometry.height
                        <= superdesktop_taskbar_top - WindowsGuiMetrics::POPUP_GAP + 0.01
                );
                let anchor_logical = interior_anchor as f32 / scale;
                assert!((geometry.left + geometry.width / 2.0 - anchor_logical).abs() < 0.01);

                let left = super::task_flyout_geometry(&monitor, &widths, Some(-2559), false, 1);
                assert!((left.left - (work_left + 8.0)).abs() < 0.01);
                let right = super::task_flyout_geometry(&monitor, &widths, Some(-1), false, 1);
                assert!((right.left + right.width - (work_right - 8.0)).abs() < 0.01);

                let fallback = super::task_flyout_geometry(&monitor, &widths, None, false, 1);
                assert!((fallback.left + fallback.width / 2.0 - (work_left / 2.0)).abs() < 0.01);
            }
        }

        let compact = MonitorRecord {
            device_name: "compact".into(),
            primary: true,
            bounds: ScreenRect {
                left: 0,
                top: 0,
                right: 200,
                bottom: 120,
            },
            work_area: ScreenRect {
                left: 0,
                top: 0,
                right: 200,
                bottom: 120,
            },
            dpi_x: 96,
            dpi_y: 96,
        };
        let geometry = super::task_flyout_geometry(&compact, &[220; 4], Some(100), false, 1);
        assert_eq!(
            (geometry.left, geometry.top, geometry.width, geometry.height),
            (8.0, 8.0, 184.0, 64.0)
        );
    }

    #[test]
    fn task_preview_clearance_reserves_owned_and_preview_mode_rows() {
        let monitor = MonitorRecord {
            device_name: "negative-mixed-dpi".into(),
            primary: false,
            bounds: ScreenRect {
                left: -3840,
                top: -160,
                right: 0,
                bottom: 2000,
            },
            work_area: ScreenRect {
                left: -3840,
                top: -160,
                right: 0,
                bottom: 1920,
            },
            dpi_x: 144,
            dpi_y: 144,
        };
        let scale = 1.5;
        for owned_shell in [false, true] {
            for rows in 1..=3 {
                let geometry = super::task_flyout_geometry(
                    &monitor,
                    &[320, 280],
                    Some(-1200),
                    owned_shell,
                    rows,
                );
                let taskbar_bottom = if owned_shell {
                    monitor.bounds.bottom
                } else {
                    monitor.work_area.bottom
                } as f32
                    / scale;
                let taskbar_top = taskbar_bottom - WindowsGuiMetrics::taskbar_height(rows);
                assert!(
                    geometry.top + geometry.height
                        <= taskbar_top - WindowsGuiMetrics::POPUP_GAP + 0.01,
                    "mode={owned_shell} rows={rows} geometry={geometry:?} taskbar_top={taskbar_top}"
                );
                assert!(geometry.left >= monitor.bounds.left as f32 / scale);
                assert!(geometry.left + geometry.width <= monitor.bounds.right as f32 / scale);
            }
        }
    }

    #[test]
    fn task_preview_card_width_tracks_live_window_aspect_and_available_width() {
        assert_eq!(super::preview_card_width_for_size(1920, 1080, 420.0), 350);
        assert_eq!(super::preview_card_width_for_size(2560, 1080, 420.0), 420);
        assert_eq!(super::preview_card_width_for_size(1080, 1920, 420.0), 160);
        assert_eq!(super::preview_card_width_for_size(1920, 1080, 264.0), 264);
        assert_eq!(super::preview_card_width_for_size(0, 1080, 420.0), 1);
    }
}
