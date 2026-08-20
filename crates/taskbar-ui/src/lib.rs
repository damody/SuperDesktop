//! GPUI taskbar surface, model, interaction, and truthful status boundary.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod advanced;
mod auto_hide;
mod grouping;
mod interaction;
mod layout;
mod notification_area;
mod notification_overflow;
mod show_desktop;
mod start;
mod status;
mod surface;
mod system_control_context;
mod system_flyout;
mod task_view;
mod taskbar_settings;
mod tracker;
mod view;
mod windows_metrics;

pub use advanced::{
    AdvancedTaskbarPreferences, AltTabDismissAction, AltTabView, FlyoutAction, FlyoutHoverAction,
    FlyoutModel, HOVER_PREVIEW_CLOSE_GRACE_MS, HOVER_PREVIEW_DELAY_MS, HoverPreviewController,
    JumpListGroup, JumpListModel, JumpListView, PreviewCard, ProgressState, TaskFlyoutView,
    TaskOverlay, TaskVisualState,
};
pub use auto_hide::{
    AUTO_HIDE_DELAY_MS, AUTO_HIDE_REVEAL_PIXELS, AutoHideEffect, AutoHideEndpoints, AutoHideInput,
    AutoHideState, PhysicalPoint, PhysicalRect, auto_hide_endpoints, reduce_auto_hide,
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
pub use show_desktop::{
    ShowDesktopObservation, ShowDesktopPlan, ShowDesktopSession, ShowDesktopTarget,
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
pub use system_control_context::{
    SystemControlContextAction, SystemControlContextCommand, SystemControlContextDismiss,
    SystemControlContextKind, SystemControlContextView,
};
pub use system_flyout::{
    NotificationCenterAction, NotificationCenterActionHandler, SystemFlyoutAction,
    SystemFlyoutDismiss, SystemFlyoutPresentation, SystemFlyoutTheme, SystemFlyoutView,
};
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
    NotificationOverflowCallback, SystemControlContextCallback, SystemFlyoutCallback,
    TaskHoverCallback, TaskPrimaryCallback, TaskbarBackgroundContextCallback, TaskbarCallbacks,
    TaskbarResizeCallback, TaskbarView,
};
pub use windows_metrics::WindowsGuiMetrics;

pub const CRATE_ROLE: &str = "taskbar GPUI surface and window-management boundary";
