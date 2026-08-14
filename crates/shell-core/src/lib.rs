//! Platform-neutral state, command, and reconciliation contracts for SuperDesktop.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod bridge;
mod identity;
mod model;
mod queue;
mod reducer;

pub use bridge::{
    BridgeLaunchRequest, BridgeLaunchSource, BridgeRepair, BridgeTerminal, MessageKey,
};
pub use identity::{
    ApplicationId, CorrelationId, Generation, IdentityError, MonitorId, RequestId, ShellItemId,
    WindowId,
};
pub use model::{
    ActiveRequest, ApplicationState, Diagnostic, DiagnosticKind, ExecutionMode, LifecyclePhase,
    MonitorState, RecoveryPhase, RequestKind, SelectionState, ShellCommand, ShellEffect,
    ShellEvent, ShellState, TerminalKind, WindowState,
};
pub use queue::{BoundedEventQueue, DEFAULT_EVENT_QUEUE_CAPACITY, QueueDomain, QueuePush};
pub use reducer::{Transition, reduce, stable_state_hash};

/// Stable description used by architecture and handoff manifests.
pub const CRATE_ROLE: &str = "platform-neutral shell state contracts";
