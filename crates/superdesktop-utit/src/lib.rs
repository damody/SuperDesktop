//! SuperDesktop UI Test Integration Tool.

#[cfg(not(windows))]
compile_error!("SuperDesktop UTIT is supported only on Windows targets.");

mod catalog;
mod cli;
mod executor;
mod model;
mod preflight;
mod report;

pub use catalog::{catalog, select_cases, validate_catalog};
pub use cli::{CommandLine, UtitCommand, parse_args};
pub use executor::{ExecutionOptions, execute_case, execute_run};
pub use model::*;
pub use preflight::{evaluate_prerequisite, observe_host};
pub use report::{hash_file, validate_report, write_report_bundle};

pub const CRATE_ROLE: &str = "test-only typed Windows shell-parity runner";
