//! External-process bridge boundary for the separately built SuperExplorer product.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

/// Marker proving this crate is a workspace foundation only.
pub const CRATE_ROLE: &str = "external Explorer process bridge";
