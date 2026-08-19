## Context

The owned taskbar already has DWM thumbnail admission and a `TaskFlyoutView`, but application composition opens it only from grouped-task clicks. The approved source design is `docs/superpowers/specs/2026-08-19-owned-taskbar-hover-previews-design.md`.

## Goals / Non-Goals

**Goals:** Implement Windows-like delayed hover previews for single/grouped tasks, stale-safe switching, popup crossing, close grace, theme/accessibility alignment, and Explorer-free UTIT evidence.

**Non-Goals:** Aero Peek desktop transparency, system taskbar delegation, persistent preview state, undocumented interfaces, or changing task-button click semantics.

## Decisions

### Pure generation controller

`HoverPreviewController` owns task/popup hover and generation. Enter produces a token; open after 400 ms requires an exact task/token match. Leave produces a token; close after 250 ms requires neither surface hovered and exact generation. This rejects stale timers without native state.

### Typed GPUI callbacks and owned scheduling

`TaskbarCallbacks` gains task-hover and the flyout gains popup-hover callbacks. The app uses GPUI executors/timers and its existing popup slot. A shared open function resolves fresh single/group identities and DWM admission. No input injection or Explorer HWND is used.

### Owned presentation

The popup uses content-sized Windows card geometry, light/dark/high-contrast tokens, real live thumbnails, title, close button, focus border, UIA Button semantics, Enter/Delete/Escape, and truthful unavailable fallback.

### Gates and correction policy

Blocking gates are `G-HOVER-MODEL`, `G-HOVER-COMPOSITION`, `G-HOVER-UI`, `G-HOVER-A11Y`, `G-SHELL-NONINTERFERENCE`, `G-UTIT`, `G-TRACE`, and `G-PACKAGE`. A-level refinements may change helper/timer wiring; B-level unsafe identity/geometry corrections reopen design/spec/tasks/evidence; C-level delegation, undocumented API, weaker timing/recovery, or external mutation requires approval.

## Risks / Trade-offs

- **[Rapid task crossing opens stale content]** → generation-token exact match.
- **[Pointer cannot cross popup gap]** → popup hover plus 250 ms close grace.
- **[Window closes during delay]** → fresh authoritative snapshot before opening.
- **[DWM registration fails]** → truthful unavailable card and RAII cleanup.

## Migration Plan

Land controller/tests, callbacks/scheduling, presentation, UTIT case, full gates, and packages. Rollback is a source revert; no persisted migration exists.

## Open Questions

None.
