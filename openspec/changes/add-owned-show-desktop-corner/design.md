## Context

SuperDesktop already enumerates eligible top-level task windows and applies validated HWND actions, while `taskbar-ui` owns the Windows 11 taskbar surface. The missing Show desktop corner crosses the pure task model, native window action boundary, GPUI rendering, headful verification, and packaging. The approved source design is `docs/superpowers/specs/2026-08-19-owned-show-desktop-corner-design.md`.

## Goals / Non-Goals

**Goals:**

- Reproduce Windows' far-right full-height Show desktop target and reversible click behavior.
- Preserve pre-minimized windows and reject stale or reused HWNDs with complete identity validation.
- Support pointer, keyboard, UIA, localization, themes, multi-row taskbars, and 175% DPI.
- Operate with Explorer and system shell UI absent.

**Non-Goals:**

- Aero Peek preview, per-monitor independent desktop sessions, desktop animations, or Settings UI.
- Win+D injection, `Shell_TrayWnd`, COM shell automation, undocumented APIs, or system UI delegation.
- Persisting the active session across SuperDesktop restart.

## Decisions

### Pure reversible session reducer

`taskbar-ui` owns a deterministic `ShowDesktopSession` and exact `ShowDesktopTarget` values. An inactive activation selects eligible, visible, non-minimized targets; successful actions become the active restore set. An active activation intersects the restore set with a fresh authoritative snapshot and then clears. This keeps policy testable and makes platform failures explicit. A stateless minimize-only implementation was rejected because it cannot reproduce second-click restore.

### Complete identity matching

Targets retain HWND, PID, and stable window identity. Restore requires all three fields to match a new native snapshot. HWND-only retention was rejected because Windows can reuse handles after destruction.

### Non-activating restore

`platform-win` adds a documented `Restore` action separate from `RestoreAndActivate`. Show desktop restores every validated target without stealing focus for each window. Existing task-button activation keeps the current activating behavior.

### Far-edge GPUI control

The system status width includes a final 8-DIP, full taskbar-height Button after the clock. It is flush right and remains continuous across one to three rows. Hover, pressed, focus, dark, and high-contrast states are conveyed with both fill and edge/border geometry. This uses the existing GPUI surface rather than a second native window, avoiding seams and appbar ownership changes.

### Correction policy and blocking gates

- **A — task refinement:** helper extraction, task order, test fixture values, or evidence paths may change without scope expansion.
- **B — design/spec correction:** an unsafe identity or geometry assumption requires updating design/spec/tasks and replacing stale evidence.
- **C — material change:** shell delegation, undocumented APIs, persistence, new external writes, weaker gates, or broader platform claims require user approval.

Blocking gates are `G-SHOW-DESKTOP-MODEL`, `G-SHOW-DESKTOP-NATIVE`, `G-SHOW-DESKTOP-UI`, `G-SHOW-DESKTOP-A11Y`, `G-SHELL-NONINTERFERENCE`, `G-TRACE`, and `G-PACKAGE`.

## Risks / Trade-offs

- **[A target closes and its HWND is reused]** → Require complete fresh identity match before restore.
- **[A minimize action fails partway]** → Store only successfully minimized identities and restore only that subset.
- **[A new window appears while desktop is shown]** → Do not mutate it; the session is intentionally limited to the admitted first-click set.
- **[Tiny edge control is hard to reach by keyboard]** → Keep the Windows pointer width while exposing a named focusable Button with Enter/Space.
- **[Theme color alone is ambiguous]** → Pair fills with a visible edge/focus border and high-contrast tokens.

## Migration Plan

1. Land reducer and native non-activating restore tests.
2. Wire the app callback and far-edge GPUI control.
3. Run source, UIA, headful Explorer-absent, and full workspace gates.
4. Build and hash release plus both NSIS installers.

Rollback is a source revert. There is no registry, protocol, settings, or persisted-data migration.

## Open Questions

None.
