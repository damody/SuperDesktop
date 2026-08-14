//! Safe external-process bridge for the separately built SuperExplorer product.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod admission;
mod command;
mod launcher;
mod repair;
mod resolver;

pub use admission::{AdmissionDiagnostic, AdmissionDispatcher, AdmissionTerminal, MonotonicMillis};
pub use command::{LaunchSpec, build_default_launch, build_folder_launch};
pub use launcher::{LaunchOutcome, ProcessLauncher, invoke_completion_callback};
pub use repair::{Locale, RepairAction, RepairModel, redacted_diagnostic, repair_model};
pub use resolver::{ExecutableCandidate, ExecutableResolver, ResolvedExecutable, ResolverTrace};

pub const CRATE_ROLE: &str = "safe external SuperExplorer process bridge";
