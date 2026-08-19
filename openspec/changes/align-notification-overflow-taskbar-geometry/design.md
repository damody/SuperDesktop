## Context

`notification_overflow_bounds` always uses `work_area.bottom`; the shell taskbar uses `bounds.bottom`. The approved design is `docs/superpowers/specs/2026-08-19-notification-overflow-taskbar-geometry-design.md`.

## Goals / Non-Goals

**Goals:** Correct both runtime modes, preserve 344 DIP/six-column proportions, and add forced Explorer-free HWND admission.

**Non-Goals:** Undocumented tray APIs, system overflow delegation, changing NotifyIcon protocol, or unavailable physical mixed-DPI certification.

## Decisions

- Pass `shell` through bounds/options/composition; select the same bottom anchor as taskbar geometry.
- Keep 8 DIP gap, 24 DIP outer vertical padding, 48 DIP cells, six columns and six-row cap.
- Run the documented fixture with 20 icons, force the owned overflow open, and record actual geometry/UIA/callback/recovery.
- Gates: `G-OVERFLOW-GEOMETRY`, `G-DPI-MATRIX`, `G-NOTIFYICON`, `G-SHELL-NONINTERFERENCE`, `G-UTIT`, `G-TRACE`, `G-PACKAGE`.
- A-level test mechanics may change; B-level geometry/tolerance corrections reopen artifacts/evidence; C-level scope, privilege, delegation or weaker gates require approval.

## Risks / Failure handling

Stale work areas are ignored through explicit mode. DWM frame uses width/gap tolerance. Small monitors clamp. Watchdog restores Explorer. Missing fixture, panel, callbacks or host recovery fails closed.

## Migration Plan

Land geometry/tests, UTIT catalog/capture, full gates and packages. Rollback is a source revert; no migration exists.

## Open Questions

None.
