## Context

Rows already persist and the refresh loop resizes GPUI taskbar windows, but `WindowKind::PopUp` omits `WS_THICKFRAME`, there is no lock state or resize observer, AppBar reservation does not follow runtime row changes, and the renderer inserts one-pixel horizontal lines between rows. The approved source is `docs/superpowers/specs/2026-08-19-windows-taskbar-resize-lock-design.md`.

## Goals / Non-Goals

**Goals:**

- Preserve distinct Preview-above-work-area and Shell-at-monitor-bottom placement.
- Add backward-compatible lock persistence and owned context/settings controls.
- Use native top-edge sizing while unlocked and snap to 1–3 rows.
- Synchronize exact HWND geometry and Shell AppBar thickness.
- Render multi-row taskbar as one continuous panel.

**Non-Goals:**

- Do not move the taskbar to left, right, or top monitor edges.
- Do not exceed three rows, implement auto-hide, or resize Explorer’s taskbar.
- Do not discover or mutate `Shell_TrayWnd` or any non-owned HWND.

## Decisions

`TaskbarSettings.locked` defaults to true and is encoded with the current settings schema. `ToggleLockTaskbar` is the first context-menu command and the same value is exposed in the owned settings behavior section.

`platform-win` adds a safe `set_owned_taskbar_resizable` adapter. It verifies the HWND belongs to the current process, toggles only `WS_THICKFRAME`, and issues `SWP_FRAMECHANGED`. This activates the GPUI Windows backend’s existing `HTTOP` handling without custom mouse capture.

`TaskbarView` owns a top resize strip only while unlocked and stores a window-bounds subscription. The observer quantizes logical height to one, two, or three 40px rows and emits a typed callback only when the row differs.

The app callback saves settings before applying exact geometry. In Shell mode it finds the same-thread controlled lease by owned HWND and calls `reserve_bottom` with the new physical height. Preview never registers or updates an AppBar.

The renderer removes all row-separator elements and retains one outer top border.

## Blocking Gates

- `G-TASKBAR-PLACEMENT`: Preview and Shell bottom anchors.
- `G-TASKBAR-RESIZE`: unlocked native sizing, quantization, persistence, DPI/topology.
- `G-TASKBAR-LOCK`: context/settings state, native style, save failure.
- `G-TASKBAR-CHROME`: no horizontal row separators.
- `G-SHELL-NONINTERFERENCE`: owned HWND validation and no Explorer lookup.
- `G-TRACE`, `G-PACKAGE`: unique evidence, validators, release/installers.

## Adjustment Policy

A-level refinements may change helpers, tests, task order, or evidence paths. B-level corrections to in-scope native-style or geometry assumptions update design/spec/tasks and stale affected evidence. C-level changes to public row bounds, platform/permission boundary, non-owned HWND mutation, or blocking gates require approval.

## Risks / Trade-offs

- **[Native style adds an unintended frame]** → Keep borderless titlebar/DWM policy and headfully inspect every theme.
- **[Resize observer loops while snapping]** → Persist/apply only when the quantized row differs from authoritative state.
- **[AppBar and HWND temporarily disagree]** → Save, exact-position, and reserve on the same UI thread; failed lease update remains a failed gate.
- **[Legacy settings omit lock]** → Default to locked without schema migration.

## Migration Plan

Land settings and platform adapters, wire context/settings and resize observer, remove separators, then run automated/headful/Explorer-free/package gates. Rollback ignores the additive unknown lock field and restores settings-only row changes.

## Open Questions

None.
