## Why

The owned clock always renders a two-line minute-only format even when the taskbar has three rows. It therefore does not align with Windows' three-line long-time, weekday, and date presentation or reserve enough common width for centered Traditional Chinese text.

## What Changes

- Extend owned local time with seconds and deterministic localized weekday formatting.
- Format Traditional Chinese and English long time/date values with correct midnight/noon behavior.
- Render time/weekday/date on three rows only when the taskbar has three rows; keep a bounded two-line form for one or two rows.
- Reserve one 112-DIP clock column and explicitly center every non-wrapping line across that width.
- Update UIA text and add format, row, geometry, theme, DPI, second-advance, and headful validation.

## Capabilities

### New Capabilities

- `row-aware-taskbar-clock`: Defines locale-aware clock content, row-dependent presentation, exact alignment, accessibility, and visual evidence.

### Modified Capabilities

None. Existing archived taskbar chrome changes are historical and no active base capability owns row-aware clock formatting.

## Impact

Affected code is limited to `platform-win` local time, `taskbar-ui` status/view models, `superdesktop-app` status composition, fixtures, tests, and evidence. No protocol, persistence, dependency, Explorer route, Settings operation, or system clock mutation is introduced.
