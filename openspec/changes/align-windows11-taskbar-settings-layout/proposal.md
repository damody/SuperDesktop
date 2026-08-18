## Why

The owned taskbar settings window still uses physical-size multiplication and a fixed `900×760` render root, producing a narrow left-aligned page, large dead canvas, clipped controls, and inconsistent Windows 11 proportions at 175% DPI. This is one of the largest remaining visible differences while SuperDesktop replaces Explorer-owned taskbar UI.

## What Changes

- Correct settings-window placement and size to use GPUI logical coordinates exactly once.
- Replace the fixed root with a full-window responsive scroll surface and centered bounded content column.
- Align Windows 11 card, row, switch, typography, focus, light/dark, and high-contrast metrics.
- Keep all expanded sections and bottom controls reachable at 100–225% DPI and small work areas.
- Preserve typed settings behavior, localization, UIA, atomic save errors, and Explorer-free ownership.
- Add headful top/middle/bottom captures, geometry evidence, packaging, and traceability gates.

## Capabilities

### New Capabilities

- `windows11-owned-taskbar-settings-layout`: Defines logical DPI sizing, responsive page geometry, Windows 11 visual metrics, scroll reachability, accessibility, and non-delegation.

### Modified Capabilities

None.

## Impact

- Affects `taskbar-ui::taskbar_settings`, `superdesktop-app::surface_runtime`, headful examples, and evidence scripts.
- Adds no dependency and changes no persisted setting schema.
- Never invokes Explorer, `Shell_TrayWnd`, `ms-settings`, or the inbox Settings app.
