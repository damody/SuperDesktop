//! Platform-neutral state and command contracts for SuperDesktop.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

/// Marker proving this crate is a workspace foundation only.
pub const CRATE_ROLE: &str = "platform-neutral shell state contracts";
