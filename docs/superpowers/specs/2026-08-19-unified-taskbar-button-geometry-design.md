# Unified Taskbar Button Geometry Design

## Problem

The owned `SuperExplorer` taskbar entry is fixed at 160 DIP while ordinary labeled task buttons shrink under crowding. This produces an uneven Windows 11 task-button rhythm and a mismatched running indicator. The live UTIT case currently proves only that the fixed entry exists, so it cannot detect this regression. One source-contract assertion is also self-referential and can pass by matching its own obsolete source string.

## Decision

Treat the fixed entry as one labeled task slot for width allocation without changing its distinct command routing. Fixed controls (Start, Search, and Task View) remain reserved on the left. The fixed entry and ordinary labeled tasks share one adaptive width in the 44-160 DIP range. The fixed running indicator uses that width minus 16 DIP, preserving an 8 DIP inset on both sides and the Windows-style long underline.

The fixed entry remains a separately owned control: it does not become a window task, does not inherit grouping state, and does not delegate to `explorer.exe`.

## Components and data flow

- `taskbar-ui` computes available task width from the live window width, fixed controls, notification/system controls, row count, and the fixed entry plus ordinary task slots.
- The fixed entry consumes the same computed width for its hit target, label container, and indicator.
- Ordinary task state layers continue consuming the same computed width.
- `capture-taskbar-live-production.ps1` records the fixed entry and ordered ordinary task measurements in physical pixels and logical DIP, then rejects width mismatch or right-control overlap.
- Source-contract tests assert current behavior through non-self-referential structural markers and negative checks for obsolete hard-coded geometry.

## Failure handling

Width allocation remains clamped to 44-160 DIP. A zero-sized or severely constrained surface therefore remains bounded and cannot create negative geometry. UTIT fails closed if the fixed entry is absent, bounds are unavailable, any labeled width leaves the clamp, fixed/task widths differ by more than one physical pixel, task order is unstable, or any task crosses the reserved right-control boundary.

## Verification

1. Pure matrix tests cover spacious, crowded, minimum-width, and multi-row allocations including the fixed slot.
2. Focused live UTIT forces a crowded one-row profile and records fixed/task widths, order, indicator contract, and right boundary.
3. Full shell-parity validates all GUI, Explorer-free, recovery, build, and static cases.
4. Workspace tests, Clippy with warnings denied, release build, dependency architecture, source boundary, and strict OpenSpec validation remain mandatory.

## Non-goals

This change does not alter task grouping, fixed-entry command semantics, notification-area contents, or Start/IME flyout behavior.
