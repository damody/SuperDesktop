## Why

SuperDesktop owns the complete taskbar behavior, but its flat chrome, oversized labeled buttons and generic system-region typography remain visibly different from Windows 11. The mismatch is amplified on the 175% DPI reference host and multi-row configuration.

## What Changes

- Add shared Windows 11 taskbar chrome tokens for light and high contrast.
- Align row background/border, Start/Search/Task View, fixed/task buttons and long/short indicators.
- Compact notification, network, volume, input-language and clock spacing and typography.
- Localize Search and map provider locale values to Windows-like compact IME labels.
- Preserve multi-row, progress, attention, grouping, typed callbacks and Explorer-free ownership.

## Capabilities

### New Capabilities

- `windows11-taskbar-chrome`: Defines Windows 11 taskbar presentation, geometry, localization, accessibility and Explorer-free constraints.

### Modified Capabilities

None.

## Impact

Changes `taskbar-ui/src/view.rs`, presentation tests, taskbar capture evidence and packaged SuperDesktop. AppBar, tracking, providers and settings schemas remain compatible.
