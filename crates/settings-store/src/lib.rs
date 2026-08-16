//! Versioned, platform-neutral settings schema and persistence boundary.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod json;
mod schema;
mod store;

pub use schema::{
    AccessibilitySettings, DecodeOutcome, DesktopPosition, ExecutionPreference, RuntimeMode,
    SettingsCorrection, SettingsError, SettingsV1, StartSettings, TaskbarSettings, ThemePreference,
    WallpaperMode, WallpaperSettings,
};
pub use store::{
    AtomicSettingsFileSystem, FixtureRootGuard, LoadOutcome, SettingsStore, StoreError,
};

pub const CRATE_ROLE: &str = "settings persistence boundary";
