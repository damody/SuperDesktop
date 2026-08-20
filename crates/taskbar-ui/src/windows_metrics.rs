//! Canonical Windows shell geometry in device-independent pixels.

/// Shared first-wave geometry used by production composition and parity tests.
pub struct WindowsGuiMetrics;

impl WindowsGuiMetrics {
    pub const TASKBAR_ROW_HEIGHT: f32 = 40.0;
    pub const TASK_ICON_EDGE: f32 = 24.0;
    pub const PRIMARY_TARGET_WIDTH: f32 = 44.0;
    pub const STATUS_TARGET_SIZE: f32 = 36.0;
    pub const POPUP_GAP: f32 = 8.0;
    pub const POPUP_EDGE_MARGIN: f32 = 8.0;

    pub const START_WIDTH: f32 = 640.0;
    pub const START_MAX_HEIGHT: f32 = 720.0;
    pub const START_HORIZONTAL_MARGIN: f32 = 12.0;
    pub const START_TASKBAR_GAP: f32 = 12.0;

    pub const SYSTEM_FLYOUT_WIDTH: f32 = 360.0;
    pub const CALENDAR_FLYOUT_WIDTH: f32 = 380.0;
    pub const NOTIFICATION_OVERFLOW_WIDTH: f32 = 344.0;
    pub const NOTIFICATION_OVERFLOW_CELL: f32 = 48.0;
    pub const NOTIFICATION_OVERFLOW_PADDING: f32 = 12.0;
    pub const NOTIFICATION_OVERFLOW_COLUMNS: usize = 6;

    pub const TASKBAR_CONTEXT_WIDTH: f32 = 220.0;
    pub const SYSTEM_CONTEXT_WIDTH: f32 = 240.0;
    pub const CONTEXT_ROW_HEIGHT: f32 = 44.0;
    pub const CONTEXT_PADDING: f32 = 4.0;
    pub const POPUP_RADIUS: f32 = 8.0;

    pub const PREVIEW_WIDTH: f32 = 360.0;

    #[must_use]
    pub fn taskbar_height(rows: u8) -> f32 {
        Self::TASKBAR_ROW_HEIGHT * f32::from(rows.clamp(1, 3))
    }

    #[must_use]
    pub fn overflow_rows(item_count: usize) -> usize {
        item_count
            .max(1)
            .div_ceil(Self::NOTIFICATION_OVERFLOW_COLUMNS)
    }

    #[must_use]
    pub fn overflow_height(item_count: usize) -> f32 {
        Self::NOTIFICATION_OVERFLOW_PADDING * 2.0
            + Self::NOTIFICATION_OVERFLOW_CELL * Self::overflow_rows(item_count) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::WindowsGuiMetrics as M;

    #[test]
    fn geometry_matrix_is_stable_for_supported_dpi_rows_and_negative_origins() {
        for dpi in [96_u32, 120, 144, 168, 192] {
            let scale = dpi as f32 / 96.0;
            for rows in 1..=3 {
                let physical_height = (M::taskbar_height(rows) * scale).round();
                assert!((physical_height / scale - M::taskbar_height(rows)).abs() <= 0.5);
                let monitor_left = -3840.0 / scale;
                let popup_left = monitor_left + M::POPUP_EDGE_MARGIN;
                assert!(popup_left >= monitor_left);
                assert!(popup_left + M::NOTIFICATION_OVERFLOW_WIDTH <= 0.0);
            }
        }
        assert_eq!(M::overflow_rows(1), 1);
        assert_eq!(M::overflow_rows(6), 1);
        assert_eq!(M::overflow_rows(7), 2);
        assert_eq!(M::overflow_height(7), 120.0);
    }
}
