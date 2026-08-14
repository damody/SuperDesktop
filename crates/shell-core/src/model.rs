use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ApplicationId, BridgeLaunchRequest, BridgeTerminal, CorrelationId, Generation, MessageKey,
    MonitorId, RequestId, ShellItemId, WindowId,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExecutionMode {
    #[default]
    Preview,
    Shell,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LifecyclePhase {
    #[default]
    Stopped,
    Preview,
    StartingShell,
    RunningShell,
    ShuttingDown,
    Recovering,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RecoveryPhase {
    #[default]
    Idle,
    Requested,
    RestoringWorkArea,
    RestoringExplorer,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorState {
    pub id: MonitorId,
    pub dpi_x: u32,
    pub dpi_y: u32,
    pub primary: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowState {
    pub id: WindowId,
    pub application_id: ApplicationId,
    pub title: String,
    pub order: u64,
    pub active: bool,
    pub minimized: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationState {
    pub id: ApplicationId,
    pub window_ids: Vec<WindowId>,
    pub pinned: bool,
    pub order: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionState {
    pub selected: BTreeSet<ShellItemId>,
    pub focused: Option<ShellItemId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalKind {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticKind {
    RejectedTransition,
    StaleResult,
    CancelledResult,
    DuplicateTerminal,
    LateDelta,
    QueueOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestKind {
    DesktopRefresh,
    WindowRefresh,
    Bridge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub request_id: Option<RequestId>,
    pub correlation_id: Option<CorrelationId>,
    pub message_key: Option<MessageKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveRequest {
    pub request_id: RequestId,
    pub generation: Generation,
    pub kind: RequestKind,
    pub cancelled: bool,
    pub terminal: Option<TerminalKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellState {
    pub mode: ExecutionMode,
    pub lifecycle: LifecyclePhase,
    pub recovery: RecoveryPhase,
    pub generation: Generation,
    pub settings_revision: u64,
    pub next_request_id: u64,
    pub monitors: BTreeMap<MonitorId, MonitorState>,
    pub desktop_items: BTreeSet<ShellItemId>,
    pub windows: BTreeMap<WindowId, WindowState>,
    pub applications: BTreeMap<ApplicationId, ApplicationState>,
    pub selection: SelectionState,
    pub requests: BTreeMap<RequestId, ActiveRequest>,
    pub terminals: BTreeMap<CorrelationId, TerminalKind>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Preview,
            lifecycle: LifecyclePhase::Stopped,
            recovery: RecoveryPhase::Idle,
            generation: Generation(0),
            settings_revision: 0,
            next_request_id: 1,
            monitors: BTreeMap::new(),
            desktop_items: BTreeSet::new(),
            windows: BTreeMap::new(),
            applications: BTreeMap::new(),
            selection: SelectionState::default(),
            requests: BTreeMap::new(),
            terminals: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellCommand {
    StartPreview,
    StartShell { explicit_opt_in: bool },
    Stop,
    CancelRequest(RequestId),
    LaunchBridge(BridgeLaunchRequest),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellEvent {
    Command(ShellCommand),
    RequestStarted {
        request_id: RequestId,
        generation: Generation,
    },
    RequestTerminal {
        request_id: RequestId,
        correlation_id: CorrelationId,
        generation: Generation,
        terminal: TerminalKind,
    },
    BridgeTerminal {
        request_id: RequestId,
        correlation_id: CorrelationId,
        generation: Generation,
        terminal: BridgeTerminal,
    },
    DesktopItemsChanged {
        request_id: RequestId,
        generation: Generation,
        items: BTreeSet<ShellItemId>,
    },
    WindowsChanged {
        request_id: RequestId,
        generation: Generation,
        windows: BTreeMap<WindowId, WindowState>,
    },
    DesktopOverflow,
    WindowOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellEffect {
    PreviewReady,
    ProbeShellPrerequisites,
    BeginShutdown,
    CancelWorker(RequestId),
    LaunchBridge(BridgeLaunchRequest),
    RequestDesktopSnapshot {
        request_id: RequestId,
        generation: Generation,
    },
    RequestWindowSnapshot {
        request_id: RequestId,
        generation: Generation,
    },
    Rejected(DiagnosticKind),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_safe_preview_authority() {
        let state = ShellState::default();
        assert_eq!(state.mode, ExecutionMode::Preview);
        assert_eq!(state.lifecycle, LifecyclePhase::Stopped);
        assert!(state.monitors.is_empty());
        assert!(state.requests.is_empty());
    }

    #[test]
    fn state_contract_is_owned_and_cloneable() {
        fn assert_owned<T: Clone + Send + Sync + 'static>() {}
        assert_owned::<ShellState>();
        assert_owned::<ShellEvent>();
        assert_owned::<ShellCommand>();
        assert_owned::<ShellEffect>();
    }
}
