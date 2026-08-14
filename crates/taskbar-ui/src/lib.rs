//! GPUI taskbar surface, model, interaction, and truthful status boundary.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod grouping;
mod interaction;
mod layout;
mod start;
mod status;
mod surface;
mod tracker;
mod view;

pub use grouping::{GroupModel, PinChange, TaskGroup};
pub use interaction::{
    AccessibleTask, FixedEntry, GroupSelection, RepairPrompt, TaskAction, TaskEffect,
    TaskInteraction, TaskSource,
};
pub use layout::{OverflowItem, SlotKind, TaskbarLayout, TaskbarRows, TaskbarSlot};
pub use start::{StartAvailability, StartControl, StartEffect, StartFailure, StartSource};
pub use status::{ClockLocale, CoreStatus, ProviderState, StatusRegion, TestClock};
pub use surface::{
    AppBarEffect, AppBarMode, AppBarRegistry, MonitorBar, MonitorGeometry, SurfaceChange,
};
pub use tracker::{
    Eligibility, OwnedWindowEvent, TaskWindow, TrackerPush, WindowObservation, WindowTracker,
};
pub use view::TaskbarView;

pub const CRATE_ROLE: &str = "taskbar GPUI surface and window-management boundary";
