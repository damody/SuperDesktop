## Context

`jump_list_options` is fixed and centered. Source design: `docs/superpowers/specs/2026-08-19-taskbar-jump-list-geometry-design.md`.

## Decisions

- Pure geometry receives shell, rows, physical anchor, entry and group counts.
- Width is 360 DIP; height derives from 8 DIP padding, 32 DIP rows, 2 DIP gaps and separators, capped at 480 DIP.
- Horizontal placement follows source and clamps; missing cursor centers.
- Bottom follows the exact owned taskbar anchor minus 8 DIP.
- UTIT verifies actual source/popup/taskbar rectangles plus Menu/MenuItem UIA and screenshots.
- Blocking gates: `G-JUMP-GEOMETRY`, `G-DPI-MATRIX`, `G-UTIT`, `G-SHELL-NONINTERFERENCE`, `G-TRACE`.

## Risks / Rollback

Destroyed tasks yield no model/open. Pointer failure centers deterministically. Small monitors clamp. Rollback is a source revert; no migration. A-level mechanics may change; geometry/tolerance corrections reopen artifacts; delegation or weaker gates requires approval.

## Open Questions

None.
