//! GPUI desktop surface models and rendering boundary.

#[cfg(not(windows))]
compile_error!("SuperDesktop is supported only on Windows targets.");

mod geometry;
mod interaction;
mod layout;
mod namespace;
mod view;
mod wallpaper;
mod watcher;

pub use geometry::{
    DesktopSurfaceRegistry, Dpi, LogicalPoint, LogicalRect, MonitorDescriptor, SurfaceChange,
    SurfaceState,
};
pub use interaction::{
    AccessibleAction, AccessibleNode, ActivationController, ActivationEffect, ActivationSource,
    AssociationRequest, DeferredAction, RepairState, TerminalResult,
};
pub use layout::{DesktopLayout, GridMetrics, PersistedPosition, SelectionGesture, SelectionModel};
pub use namespace::{
    DesktopItem, DesktopOrigin, IconDescriptor, ItemCapabilities, merge_desktop_items,
};
pub use view::DesktopView;
pub use wallpaper::{
    BoundedWallpaperCache, ImageSize, Placement, WallpaperError, WallpaperMode, wallpaper_placement,
};
pub use watcher::{DesktopWatcherQueue, WatcherDelta, WatcherKind, WatcherPush};

pub const CRATE_ROLE: &str = "desktop GPUI surface and interaction boundary";
