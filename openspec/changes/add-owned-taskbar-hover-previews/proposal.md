## Why

SuperDesktop can render DWM live-thumbnail cards, but only after clicking a grouped task; standard Windows hover previews for single and grouped applications are missing. This is a prominent interaction and visual gap when Explorer is removed.

## What Changes

- Add stale-safe 400 ms task-hover opening and 250 ms leave grace behavior.
- Open owned DWM previews for both single windows and grouped applications without changing click behavior.
- Keep the popup alive while the pointer crosses into it and close only after both task and popup are left.
- Coordinate auto-hide so an owned preview holds visibility, then the existing hide delay resumes after preview grace closes.
- Align preview card geometry, themes, focus, UIA, close, activation, and unavailable states with Windows.
- Add a real-pointer Explorer-free UTIT case plus automated, traceability, release, and package evidence without archiving.

## Capabilities

### New Capabilities

- `owned-taskbar-hover-previews`: Defines hover timing, identity reconciliation, popup lifetime, live-thumbnail presentation, accessibility, and Explorer-free verification.

### Modified Capabilities

None.

## Impact

Changes `taskbar-ui`, `superdesktop-app`, the existing DWM preview composition, UTIT catalog/capture scripts, OpenSpec evidence, and product packages. No protocol, settings migration, Explorer call, undocumented API, or production privilege is added.
