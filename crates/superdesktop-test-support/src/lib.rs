//! Deterministic fakes and fixtures shared by SuperDesktop work packages.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

/// Marker proving this crate is a workspace foundation only.
pub const CRATE_ROLE: &str = "test support boundary";

mod shell_fixture;
mod verification;

pub use shell_fixture::{FakeEffectAdapter, ShellFixtureBuilder};
pub use verification::{
    CandidateGeometry, ImeFixture, LocaleFixture, PerformanceSamples, ResourceSeries,
    VisualContract,
};
