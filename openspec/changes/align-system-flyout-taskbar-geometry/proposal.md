## Why

Owned input, network/power, volume, and calendar flyouts sit approximately one extra taskbar row above the Windows 11 position in replacement-shell mode. Their geometry always subtracts taskbar height from `work_area.bottom`, even though the shell taskbar itself is anchored to `bounds.bottom` and the reported work area can already contain an Explorer/AppBar reservation.

## What Changes

- Make system-flyout placement explicitly aware of preview versus replacement-shell taskbar anchoring.
- Preserve Windows 11 preferred widths/heights while eliminating double taskbar reservation.
- Add a 96–216 DPI, one-to-three-row, negative-origin and constrained-monitor pure geometry matrix.
- Extend the existing UTIT system-status case with actual popup/taskbar rectangles, logical dimensions, gap, containment, replacement, Explorer absence, and recovery assertions.
- Run full local, Explorer-free, traceability, release, and no-launch package gates without archiving.

## Capabilities

### New Capabilities

- `system-flyout-taskbar-geometry`: Defines mode-aware taskbar anchoring and authoritative runtime geometry admission for owned system flyouts.

### Modified Capabilities

None.

## Impact

Changes `superdesktop-app`, the system-status UTIT capture script and evidence/packages. No protocol, settings, privilege, persistence, undocumented API, or Explorer dependency is added.
