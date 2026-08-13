//! Windows integration boundary. No Shell capability is active in this crate yet.

/// Shared Windows capability adapters used by controlled composition spikes.
pub mod common;

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

/// Marker proving this crate is a workspace foundation only.
pub const CRATE_ROLE: &str = "Windows platform boundary";
