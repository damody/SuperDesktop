## Why

The owned notification overflow uses logical GPUI dimensions inside a native popup whose bounds are not converted to physical pixels. At 175% DPI the result is a narrow surface that overlaps the taskbar instead of floating above it like Windows 11.

## What Changes

- Make overflow native bounds explicitly DPI-scaled, work-area-clamped and separated from the taskbar edge.
- Align panel width, grid density, corner radius, border, shadow, icon sizing, hover, focus and pressed states with Windows 11.
- Preserve owned pointer, context, keyboard, UIA, dismissal and Explorer-free behavior.
- Add deterministic DPI geometry tests and current-host headful evidence.

## Capabilities

### New Capabilities

- `windows11-notification-overflow`: Defines Explorer-free overflow geometry, visual states, accessibility and DPI behavior.

### Modified Capabilities

None.

## Impact

Changes `taskbar-ui::NotificationOverflowView`, `superdesktop-app` popup placement, related tests, evidence and packaged SuperDesktop binaries. No new provider, registry value or Explorer dependency is introduced.
