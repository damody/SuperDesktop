# Windows taskbar placement, resize, and lock

## Outcome

SuperDesktop shall behave like the classic resizable Windows taskbar while preserving its Explorer-free shell ownership. When Explorer is present in Preview mode, the owned taskbar remains immediately above the existing Explorer work area. When Explorer is absent in Shell mode, it occupies the monitor bottom edge formerly owned by Explorer. Users can unlock the taskbar from its owned context menu, drag the top edge to choose one, two, or three rows, and lock it again.

## Selected approach

The Windows GPUI backend already maps the top non-client edge to `HTTOP` for borderless windows, but popup windows are created without `WS_THICKFRAME`. A documented platform adapter will add or remove only `WS_THICKFRAME` on the caller-owned taskbar HWND according to the persisted lock setting. Native resizing therefore owns capture, cancellation, DPI, and negative-origin behavior.

A GPUI window-bounds observer converts the live logical height to the nearest 40px row count, clamps it to 1–3, and emits a typed resize callback. The app atomically persists the new row count, snaps the HWND to an exact physical height, and updates the same-thread AppBar lease in Shell mode.

## Settings and context menu

`TaskbarSettings` gains `locked: bool`, defaulting to `true` like the traditional Windows taskbar. Decoding an older settings file supplies the default without changing schema version; encoding always writes the field. The owned context menu gains a first `ToggleLockTaskbar` item with a checkmark and localized accessible state. The owned taskbar settings surface exposes the same value under behaviors.

Toggling lock is an atomic settings save. A failed save leaves both the authoritative setting and native style unchanged. The refresh loop reconciles every taskbar HWND to the authoritative lock state.

## Placement and row geometry

Each row remains 40 logical pixels. Preview mode uses `monitor.work_area.bottom`, so Explorer and SuperDesktop do not overlap. Shell mode uses `monitor.bounds.bottom` and reserves exactly `rows × 40 × DPI / 96` physical pixels through the existing controlled AppBar capability.

The top-edge observer ignores bounds changes while locked. While unlocked, it maps heights below 60px to one row, 60–99px to two rows, and 100px or more to three rows, then snaps to the exact row height. The window width and bottom edge never drift during a row change.

## Visual behavior

Multi-row rendering uses one continuous taskbar panel. It MUST NOT draw horizontal row separators. The outer top border remains, and per-task running/progress indicators remain independent of row count.

An unlocked taskbar exposes a narrow top resize hit strip with the vertical-resize cursor and accessible name. The strip is absent while locked, so accidental resize is impossible.

## Explorer independence and safety

All resizing applies only to the HWND supplied by the owned GPUI window. The platform adapter verifies that the HWND belongs to the current process before changing style. It never finds or modifies `Shell_TrayWnd`, Explorer windows, or another process. Preview performs no AppBar registration; Shell uses only the existing controlled lease.

## Verification

Blocking gates:

- `G-TASKBAR-PLACEMENT`: Preview above Explorer work area and Shell at the physical monitor bottom.
- `G-TASKBAR-RESIZE`: native unlocked resize, 1–3 row snapping, persistence, DPI, and negative-origin behavior.
- `G-TASKBAR-LOCK`: context/settings lock state, checked UIA state, save failure, and native style reconciliation.
- `G-TASKBAR-CHROME`: no horizontal row separators in one-, two-, or three-row captures.
- `G-SHELL-NONINTERFERENCE`: no Explorer HWND lookup or delegated shell surface.
- `G-TRACE` and `G-PACKAGE`: unique evidence, strict validation, release and both NSIS installers.

Automated gates cover schema fallback/round-trip, row quantization, context menu actions, HWND ownership validation, AppBar thickness, and source guards. Headful gates capture Preview with Explorer and Shell without Explorer at 175% DPI, drag through all row counts, verify lock prevents resize, inspect UIA, and restore Explorer with the watchdog. Packaging builds without launch.

## Adjustment policy

- **A — refinement:** helper extraction, task order, exact test fixtures, and evidence paths may change without changing scope or gates.
- **B — correction:** an unreasonable native-style, threshold, or geometry assumption inside this scope requires design/spec/task correction and stale-evidence replacement.
- **C — material change:** undocumented APIs, changing the 1–3 row public contract, touching non-owned HWNDs, weakening gates, or adding external mutation requires explicit approval.

## Rollback

Reverting this batch restores settings-driven rows without native dragging. Existing settings containing `locked` remain forward-compatible unknown data to older binaries. Rollback MUST NOT start Explorer except through the existing guardian recovery contract.
