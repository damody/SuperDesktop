## Why

Labeled task buttons are fixed at 160 DIP. After reserving tray/status controls, crowded one-row tasks are clipped instead of shrinking like Windows.

## What Changes

- Compute 44–160 DIP labeled task width from live available width, task count and rows.
- Apply one width to hit target, label, progress, attention and indicator.
- Add pure and UTIT crowded/non-overlap gates; keep unarchived.

## Capabilities

### New Capabilities

- `adaptive-taskbar-task-widths`: Windows-like adaptive labeled task sizing and admission.

### Modified Capabilities

None.

## Impact

Changes taskbar UI/tests and UTIT report only. No Explorer, protocol, persistence or privilege change.
