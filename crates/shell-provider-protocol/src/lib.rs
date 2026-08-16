//! Versioned, platform-neutral contracts for isolated SuperDesktop providers.

mod context_menu;
mod dto;
mod envelope;
mod search;
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
pub use search::{
    SearchBatch, SearchProvider, SearchProviderState, SearchQuery, rank_search_results,
};
pub use validation::{
    MAX_COLLECTION_ITEMS, MAX_FRAME_BYTES, MAX_ICON_BYTES, MAX_TEXT_BYTES, Validate,
    ValidationError, validate_frame_size,
};

/// Stable description used by architecture and handoff manifests.
pub const CRATE_ROLE: &str = "platform-neutral isolated shell provider protocol";
