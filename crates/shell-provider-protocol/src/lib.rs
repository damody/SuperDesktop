//! Versioned, platform-neutral contracts for isolated SuperDesktop providers.

mod dto;
mod envelope;
mod validation;

pub use dto::{
    CommandDescriptor, CommandId, CommandRisk, IconData, NotificationIcon, ProviderCapability,
    SearchCategory, SearchResult, ShellItem, ShellItemKind, TaskPreview, VirtualDesktop,
};
pub use envelope::{
    CURRENT_PROTOCOL, Envelope, Handshake, HostHealth, ProtocolVersion, ProviderRequest,
    ProviderResponse, ResponseBody, TerminalKind, contract_manifest,
};
pub use validation::{
    MAX_COLLECTION_ITEMS, MAX_FRAME_BYTES, MAX_ICON_BYTES, MAX_TEXT_BYTES, Validate,
    ValidationError, validate_frame_size,
};

/// Stable description used by architecture and handoff manifests.
pub const CRATE_ROLE: &str = "platform-neutral isolated shell provider protocol";
