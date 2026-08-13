//! Test-support boundary. Fakes and fixtures are introduced by their owning waves.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

/// Marker proving this crate is a workspace foundation only.
pub const CRATE_ROLE: &str = "test support boundary";
