//! Deterministic fakes and fixtures shared by SuperDesktop work packages.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

/// Marker proving this crate is a workspace foundation only.
pub const CRATE_ROLE: &str = "test support boundary";

mod completion_rollup;
mod shell_fixture;
mod verification;

pub use completion_rollup::{
    CapabilityLimitation, CompletionRollup, EvidenceSource, ExternalEvidenceSource,
    GateDisposition, REQUIRED_COMPLETION_CHILDREN, REQUIRED_COMPLETION_GATES, RollupDecision,
};
pub use shell_fixture::{FakeEffectAdapter, ShellFixtureBuilder};
pub use verification::{
    CandidateGeometry, ImeFixture, LocaleFixture, PerformanceSamples, ResourceSeries,
    VisualContract,
};
