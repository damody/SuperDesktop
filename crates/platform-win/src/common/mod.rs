//! Private-to-platform, explicit Win32 ownership boundaries used by capability spikes.

pub mod admission;
pub mod appbar_shell_hook;
pub mod desktop;
pub mod desktop_operations;
pub mod elevation;
pub mod explorer_recovery;
pub mod ffi_boundary;
pub mod guardian_lease;
pub mod icon;
pub mod jump_list;
pub mod monitor_dpi_start;
pub mod native_window;
pub mod notify_icon_compat;
pub mod owner_lease;
pub mod power;
pub mod settings_file;
pub mod shell_hotkey;
pub mod start_search;
pub mod system_status;
pub mod taskbar;
pub mod taskbar_preview;
pub mod taskbar_status;
pub mod virtual_desktop;
pub mod windows_notification_events;
