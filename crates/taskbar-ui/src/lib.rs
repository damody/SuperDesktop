//! GPUI taskbar surface, model, interaction, and truthful status boundary.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod advanced;
mod grouping;
mod interaction;
mod layout;
mod notification_area;
mod notification_overflow;
mod start;
mod status;
mod surface;
mod system_flyout;
mod task_view;
mod taskbar_settings;
mod tracker;
mod view;

pub use advanced::{
    AdvancedTaskbarPreferences, FlyoutAction, FlyoutModel, JumpListGroup, JumpListModel,
    JumpListView, PreviewCard, ProgressState, TaskFlyoutView, TaskOverlay, TaskVisualState,
};
pub use grouping::{GroupModel, PinChange, TaskGroup};
pub use interaction::{
    AccessibleTask, FixedEntry, GroupSelection, RepairPrompt, TaskAction, TaskEffect,
    TaskInteraction, TaskSource,
};
pub use layout::{OverflowItem, SlotKind, TaskbarLayout, TaskbarRows, TaskbarSlot};
pub use notification_area::{
    NotificationAccessibleNode, NotificationAreaModel, NotificationPlacement,
};
pub use notification_overflow::{
    NotificationOverflowAction, NotificationOverflowDismiss, NotificationOverflowView,
};
pub use start::{
    StartAccessibilityNode, StartActions, StartAvailability, StartControl, StartEffect,
    StartFailure, StartModel, StartPage, StartPowerAction, StartSnapshot, StartSource, StartView,
};
pub use status::{
    ClockLocale, CoreStatus, ProviderState, StatusRegion, SystemFlyoutKind, SystemStatusAction,
    TestClock,
};
pub use surface::{
    AppBarEffect, AppBarMode, AppBarRegistry, MonitorBar, MonitorGeometry, SurfaceChange,
};
pub use system_flyout::{SystemFlyoutAction, SystemFlyoutDismiss, SystemFlyoutView};
pub use task_view::{
    DesktopCard, TaskViewAccessibleNode, TaskViewEffect, TaskViewModel, TaskViewSurface,
    VirtualDesktopSnapshot,
};
pub use taskbar_settings::{
    TaskbarContextAction, TaskbarContextCommand, TaskbarContextEffect, TaskbarContextModel,
    TaskbarContextView, TaskbarSettingId, TaskbarSettingRow, TaskbarSettingsAction,
    TaskbarSettingsEffect, TaskbarSettingsModel, TaskbarSettingsSection, TaskbarSettingsView,
    TaskbarSurfaceDismiss,
};
pub use tracker::{
    Eligibility, OwnedWindowEvent, TaskWindow, TrackerPush, WindowObservation, WindowTracker,
};
pub use view::{
    NotificationOverflowCallback, SystemFlyoutCallback, TaskbarBackgroundContextCallback,
    TaskbarCallbacks, TaskbarView,
};

pub const CRATE_ROLE: &str = "taskbar GPUI surface and window-management boundary";
