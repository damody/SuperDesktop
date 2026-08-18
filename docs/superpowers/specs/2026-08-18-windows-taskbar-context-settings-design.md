# Windows 11 Taskbar Context Menu and Settings Design

## Goal

SuperDesktop shall own the taskbar context-menu and taskbar-settings experience so it remains usable after Explorer is removed. The surfaces shall closely follow Windows 11 geometry, typography, spacing, theme, keyboard, and accessibility behavior while remaining truthful about capabilities SuperDesktop does not yet implement.

## User surfaces

### Empty taskbar context menu

Right-clicking taskbar background opens a compact owned menu near the pointer and clamped to the monitor work area. It contains **Task Manager** and **Taskbar settings**, uses an 8 px rounded Windows 11 surface, 32 px rows, 16 px icons, separators, hover/pressed states, shadow, light/dark/high-contrast palettes, and Escape/outside-click dismissal. Task Manager is launched through a validated Windows executable route. Taskbar settings opens the owned settings window and returns focus to the taskbar when dismissed.

### Application context menu

The existing application Jump List remains the data source and command route, but its shell is restyled to Windows 11: 8 px radius, 12 px outer padding, 32 px command rows, 16 px icons, section separators, consistent hover/pressed/focus states, and a distinct destructive close action. Pin/unpin, provider commands, and close-all retain their existing typed behavior.

### Owned taskbar settings

The settings window uses the Windows 11 **Personalization > Taskbar** information hierarchy: breadcrumb, grouped cards, expandable headers, control rows, supporting text, dropdowns, switches, and related-settings rows. It includes:

- Taskbar items: Search visibility, Task View visibility, and truthful disabled rows for Widgets/unsupported inbox surfaces.
- System tray icons: existing notification/system status capabilities plus truthful disabled rows for unsupported platform surfaces.
- Other system tray icons: opens the owned notification overflow surface.
- Taskbar behaviors: alignment, labels, group combination, previews, monitor policy, and one-to-three rows.

Unsupported controls are disabled and explain why; they never report a successful mutation.

## Settings and behavior

`TaskbarSettings` gains bounded enums/booleans for search mode, Task View visibility, and alignment. Existing fields remain authoritative for rows, labels, grouping, previews, monitor policy, and pins. Decoding missing fields preserves Windows-like defaults and future/unknown data remains round-trippable.

Every settings mutation clones the current settings document, validates the proposed taskbar state, saves it atomically through `settings-store`, and only then publishes the saved revision to all taskbar entities. Save failure leaves the previous UI and behavior intact and exposes an accessible error status.

Search modes are `hidden`, `icon`, and `box`. Icon and box activate the owned Start search route. Alignment is `left` or `center`; it changes the task region layout without moving the system-status area. Task View visibility removes both rendering and hit testing when disabled.

## Components and data flow

- `settings-store` owns serialization, validation, migration defaults, and round-trip tests.
- `taskbar-ui` owns pure context-menu/settings models and GPUI rendering. Models emit typed effects and contain no filesystem, process, or registry mutation.
- `superdesktop-app` owns windows, focus lifecycle, Task Manager launch admission, settings persistence, and distribution of saved settings.
- Existing Jump List/provider logic remains the only application-command source.

Pointer/keyboard event → pure UI effect → composition callback → validated external effect or atomic settings save → saved settings snapshot → all taskbar views notify and rerender.

## Failure and lifecycle rules

Only one context menu and one settings window may exist per product process. Opening a replacement dismisses the old surface. Monitor removal clamps/reopens surfaces on a live monitor; application close tears them down before taskbar leases. Process launch, provider, and settings failures are traced and displayed without changing unrelated fields. Callback boundaries remain no-unwind and no raw HWND escapes into UI models.

## Verification

- Pure model tests for command availability, keyboard navigation, dismissal, and unsupported rows.
- Settings decode/encode/migration tests for every new field and invalid values.
- Composition tests for atomic save, failure preservation, focus return, singleton windows, and Preview/Shell parity.
- Headful screenshots at the host DPI plus pure 100–500% geometry matrices, light/dark/high-contrast, long Traditional Chinese/English labels, and UIA names/patterns.
- Release workspace fmt/tests/clippy, strict OpenSpec validation, and standalone/combined NSIS package checks without launch.

## Non-goals

This change does not call Explorer's context menu or Windows Settings, claim unsupported Widgets/pen/touch-keyboard ownership, replace Task Manager, or silently implement auto-hide without a verified AppBar input/reservation design.
