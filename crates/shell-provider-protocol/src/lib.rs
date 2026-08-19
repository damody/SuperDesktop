//! Versioned, platform-neutral contracts for isolated SuperDesktop providers.

mod context_menu;
mod dto;
mod envelope;
mod jump_list;
mod notification;
mod search;
mod system_status;
mod taskbar_state;
mod validation;

pub use context_menu::{
    MAX_MENU_DEPTH, MenuContext, MenuEnumeration, MenuInvocation, MenuInvocationResult,
    validate_command_tree,
};
pub use dto::{
    CommandDescriptor, CommandId, CommandRisk, IconData, NotificationIcon, ProviderCapability,
    SearchCategory, SearchResult, ShellItem, ShellItemKind, TaskPreview, VirtualDesktop,
};
pub use envelope::{
    CURRENT_PROTOCOL, Envelope, Handshake, HostHealth, ProtocolVersion, ProviderRequest,
    ProviderResponse, ResponseBody, TerminalKind, contract_manifest,
};
pub use jump_list::{JumpListRequest, JumpListResponse};
pub use notification::{
    IconKey, NotificationEvent, NotificationEventKind, NotificationHostHealth,
    NotificationHostResponse, NotificationMutation, NotificationSeverity, NotificationSnapshot,
    NotifyIconCallbackRoute, NotifyIconClientIdentity, NotifyIconCompatibilityOperation,
    NotifyIconCompatibilityRequest, NotifyIconCompatibilityTerminal, NotifyIconIdentity,
    NotifyIconLayoutVersion, NotifyIconTerminalKind, OwnedNotification, OwnedNotificationContent,
    OwnedNotifyIcon, RegisteredIcon, WindowsNotificationAccess, WindowsNotificationChange,
    WindowsNotificationEventStatus,
};
pub use search::{
    SearchBatch, SearchProvider, SearchProviderState, SearchQuery, rank_search_results,
};
pub use system_status::{
    AudioStatus, ClockCalendarStatus, InputProfile, InputProfileKind, InputStatus,
    MAX_INPUT_PROFILES, MAX_WIFI_NETWORKS, NetworkStatus, PowerStatus, StatusAvailability,
    SystemStatusCommand, SystemStatusCommandRequest, SystemStatusCommandTerminal,
    SystemStatusHostHealth, SystemStatusHostRequest, SystemStatusHostResponse,
    SystemStatusSnapshot, SystemStatusTerminalKind, WifiNetwork, WifiStatus,
};
pub use taskbar_state::{
    TaskbarAttentionMode, TaskbarAttentionState, TaskbarProgressKind, TaskbarProgressState,
    TaskbarStateHostRequest, TaskbarStateHostResponse, TaskbarStateSnapshot, TaskbarStateTerminal,
    TaskbarStateTerminalKind, TaskbarWindowIdentity, TaskbarWindowState, reduce_group_progress,
};
pub use validation::{
    MAX_COLLECTION_ITEMS, MAX_FRAME_BYTES, MAX_ICON_BYTES, MAX_TEXT_BYTES, Validate,
    ValidationError, validate_frame_size,
};

/// Stable description used by architecture and handoff manifests.
pub const CRATE_ROLE: &str = "platform-neutral isolated shell provider protocol";
