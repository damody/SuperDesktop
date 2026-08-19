## Context

The owned preview surface is complete, but `task_flyout_options` centers every popup and `open_task_preview` unconditionally activates it. The approved source design is `docs/superpowers/specs/2026-08-19-taskbar-preview-anchor-focus-design.md`.

## Goals / Non-Goals

**Goals:** Anchor hover/click previews to their source, clamp across monitor/DPI layouts, preserve foreground focus on hover, preserve keyboard use on click, and prove both through Explorer-free UTIT evidence.

**Non-Goals:** Aero Peek, system taskbar delegation, changing preview timing/cards, changing click semantics, or introducing persistent settings.

## Decisions

### Typed open source

`PreviewOpenSource::{Hover, Click}` owns activation and keyboard-focus policy. Hover does neither; click does both. Call sites must pass a source explicitly.

### Pure anchor geometry

A pure helper converts physical work-area and pointer coordinates using the selected monitor DPI, centers the content-sized popup on the source, then clamps it inside the work area. Missing cursor data falls back to monitor center.

### Conditional view focus

Window options, native activation, and `TaskFlyoutView` focus assignment all follow the same source policy. Pointer/UIA card actions remain available in a hover popup; keyboard traversal is admitted for click-opened popups.

### Headful gate

UTIT captures foreground HWND before hover and after admission, source and popup rectangles, expected clamped center, delta, monitor containment, Explorer absence/recovery, screenshot, and binary hash.

### Gates and correction policy

Blocking gates are `G-ANCHOR`, `G-FOCUS`, `G-UTIT`, `G-SHELL-NONINTERFERENCE`, `G-TRACE`, and `G-PACKAGE`. A-level changes may refine helper wiring; B-level changes to focus or geometry semantics reopen design/spec/tasks/evidence; C-level Explorer delegation, undocumented API, weaker recovery, or external mutation requires approval.

## Risks / Trade-offs

- **[Pointer moves during delay]** -> admission uses the pointer that still satisfies the hover controller.
- **[Edge task clips popup]** -> clamp against the full source-monitor work area.
- **[Mixed DPI shifts placement]** -> keep native inputs physical and convert once through the monitor DPI.
- **[Hover loses keyboard access]** -> click-opened grouped previews retain focus and keyboard behavior.

## Migration Plan

Land pure policy/geometry, composition/view wiring, UTIT evidence, full gates, and packages. Rollback is a source revert; no persisted migration exists.

## Open Questions

None.
