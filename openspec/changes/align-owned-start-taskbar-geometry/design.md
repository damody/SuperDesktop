## Context

`start_window_geometry` currently ends at `work_area.bottom - 12 DIP`; `taskbar_physical_geometry` places SuperDesktop's preview taskbar above that same boundary. The approved design is `docs/superpowers/specs/2026-08-19-owned-start-taskbar-geometry-design.md`.

## Goals / Non-Goals

**Goals:** Eliminate Start/taskbar overlap, preserve 640×720 DIP proportions across DPI/rows/modes, and make Explorer-free actual geometry blocking.

**Non-Goals:** Change Start content/search semantics, call system Start/Search, pixel-diff dynamic text, or claim unavailable physical mixed-DPI certification.

## Decisions

### Exact taskbar contract reuse

Geometry receives `shell` and rows. Preview anchors to `work_area.bottom`; shell anchors to `bounds.bottom`. Both subtract `40 DIP × rows + 12 DIP` exactly once.

### Hybrid admission

Pure tests cover 96/144/168/216 DPI, 1–3 rows, both modes, negative origins, stale work areas and constrained monitors. UTIT runs `--shell` with Explorer absent, records actual rectangles/DPI/logical sizes/gap/containment, and preserves UIA/screenshot/hash checks.

### Noninterference

System Start/Search/Shell host PID sets must not gain a process after owned Start opens. The recovery watchdog and runner verify Explorer absence and restoration.

### Gates and corrections

Blocking gates are `G-START-GEOMETRY`, `G-DPI-MATRIX`, `G-OWNED-START`, `G-SHELL-NONINTERFERENCE`, `G-UTIT`, `G-TRACE`, and `G-PACKAGE`. A-level mechanics may change. B-level geometry/tolerance corrections reopen design/spec/tasks/evidence. C-level scope, privilege, delegation, weaker gates or external mutation requires approval.

## Risks / Trade-offs

- Stale work area: explicit mode avoids inference.
- DWM shadow: actual gap tolerance is 4–20 DIP.
- Small screens: width/height clamp to at least one logical pixel.
- Explorer suppression failure: watchdog restores and the case fails closed.

## Migration Plan

Land geometry/tests, Explorer-free UTIT, full gates and packages. Rollback is a source revert; no migration exists.

## Open Questions

None.
