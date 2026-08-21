# Resizable persistent Start menu

## Goal

The owned SuperDesktop Start menu shall resize from native window edges and reopen using the user's previous logical width and height.

## Data model

`StartSettings` gains optional `width_dip` and `height_dip` values. Missing values preserve the existing 640×720 DIP default. Values are encoded inside the existing schema version so older files remain valid and unknown future fields retain their existing behavior.

Persisted values are logical device-independent pixels, not physical pixels. A 640 DIP menu therefore remains visually consistent after DPI or monitor changes. Restore always clamps the requested size to the current monitor before creating the window.

## Geometry

The minimum usable size is 420×360 DIP. The maximum is the current work area minus the existing horizontal margin, taskbar height, and Start-to-taskbar gap. On a smaller monitor the effective minimum collapses to the available size, ensuring the menu remains fully contained.

Only size is persisted. Position continues to derive from the taskbar alignment: left alignment anchors to the work-area margin and center alignment centers the resized menu. The bottom edge remains above the matching preview or owned-shell taskbar.

## Resize and persistence lifecycle

The GPUI Start window uses native resizable window styles while remaining a titlebar-free popup. `StartView` observes window bounds and records only real size changes. It debounces the persistence callback for 250ms so a drag does not create a settings write storm.

The latest size is flushed when the debounce expires, when the window loses activation, and before the taskbar toggle closes an existing Start window. The app callback clamps and rounds the size, clones the latest settings revision, and saves through the existing atomic `SettingsStore`. Successful and failed persistence have distinct trace markers.

## Failure behavior

Malformed, zero, stale, oversized, or undersized settings never prevent Start from opening. Geometry normalization falls back or clamps to a fully visible size. A persistence failure leaves the last in-memory and on-disk valid settings intact and remains visible in the action trace.

## Verification

- Settings tests cover missing fields, round-trip values, malformed numbers, and forward-compatible JSON.
- Geometry tests cover default, minimum, maximum, narrow work areas, left/center alignment, shell/preview taskbars, and 96–192 DPI.
- View tests cover native resizing, debounce coalescing, activation flush, close flush, and no-op unchanged size.
- Physical UTIT resizes Start to two non-default sizes, closes/reopens it, restarts the release candidate, and verifies the same logical size within DPI rounding tolerance.
- Workspace tests, Clippy with warnings denied, release build, strict OpenSpec validation, and installer hash comparison remain blocking.

## Non-goals

This change does not persist Start position, synchronize size between Windows users, add resize settings UI, or allow the menu outside the active monitor work area.
