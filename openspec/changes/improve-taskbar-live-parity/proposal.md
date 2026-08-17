## Why

Self-verification after the label repair found three production mismatches: the current host is configured for never-combine while SuperDesktop groups by default, multi-row rendering leaves lower rows empty, and the visible clock is a fixed test fixture. These defects prevent the taskbar from approaching the active Windows 11 + ExplorerPatcher behavior.

## What Changes

- Default new/partial taskbar settings to never combine while preserving explicit grouping.
- Fill one-to-three configured rows top-to-bottom before wrapping into a new task column.
- Add a Win32 local-date/time adapter and update taskbar status when the minute/date changes.
- Add settings, layout, platform-clock, refresh, headful, and strict validation evidence.
- Keep task labels, actions, grouping capability, AppBar height, and notification-area ownership unchanged.

## Capabilities

### New Capabilities

- `taskbar-live-parity`: Defines host-aligned never-combine defaults, column-major multi-row packing, and live local clock refresh.

### Modified Capabilities

None.

## Impact

- Changes settings-store defaults, taskbar-ui rendering, a new platform-win clock adapter, and superdesktop-app reconciliation.
- No schema version, external write, registry mutation, dependency, or installer contract changes.
