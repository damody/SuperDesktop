## Why

Owned Start has Windows-like logical size but its bottom ignores SuperDesktop's own taskbar. It overlaps the preview taskbar and depends on stale Explorer work-area reservation in shell mode.

## What Changes

- Add explicit preview/shell and taskbar-row inputs to Start geometry.
- Anchor Start above the owned taskbar with a 12 DIP gap and constrained Windows 640×720 DIP proportions.
- Add DPI/mode/row/origin boundary tests.
- Upgrade `gui-start` to Explorer-free watchdog execution with authoritative HWND/UIA geometry and no-system-host assertions.
- Run full traceability, quality, release and no-launch package gates without archiving.

## Capabilities

### New Capabilities

- `owned-start-taskbar-geometry`: Defines Start/taskbar non-overlap, mode-aware anchoring, logical proportions and Explorer-free admission.

### Modified Capabilities

None.

## Impact

Changes `superdesktop-app`, Start UTIT capture/catalog, evidence and packages. No protocol, persistence, privilege, undocumented API, or Explorer/system Start dependency is added.
