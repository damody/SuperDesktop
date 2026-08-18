# Windows Taskbar Resize and Lock Tasks

## 1. Contracts and Native Boundary

### 1.1 Persist authoritative lock state

**目的：** Add one backward-compatible source of truth for taskbar locking.
**輸入：** Current settings schema, decoder, encoder, correction tests.
**產出：** `TaskbarSettings.locked` and round-trip/fallback tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-LOCK`; automated record.
**完成門檻：** Legacy files default locked, valid values round-trip, invalid siblings stay isolated.

- [x] 1.1.1 Add `locked` with a true default to taskbar settings.
- [x] 1.1.2 Decode and encode lock state without changing schema version.
- [x] 1.1.3 Add legacy fallback, round-trip, and independent-field tests.

### 1.2 Add an owned-HWND resize-style adapter

**目的：** Enable Windows native top-edge sizing without touching foreign windows.
**輸入：** Caller-owned HWND, current process identity, Windows style APIs.
**產出：** Safe `set_owned_taskbar_resizable` adapter and negative tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-RESIZE`, `G-SHELL-NONINTERFERENCE`; platform record.
**完成門檻：** Owned style toggles only `WS_THICKFRAME`; invalid/foreign HWNDs are zero-effect.

- [x] 1.2.1 Validate HWND liveness and current-process ownership.
- [x] 1.2.2 Toggle `WS_THICKFRAME` with frame refresh and idempotence.
- [x] 1.2.3 Add owned, invalid, foreign, and repeated-toggle tests.

## 2. Owned Taskbar Interaction

### 2.1 Expose lock in context menu and settings

**目的：** Give pointer, keyboard, and UIA users one truthful lock control.
**輸入：** Lock setting, context model/view, taskbar settings model/view.
**產出：** Checked localized menu item, settings behavior row, typed save effects.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-LOCK`; model/UIA/headful records.
**完成門檻：** Both surfaces show the authoritative state and emit one atomic toggle.

- [x] 2.1.1 Add first-position `ToggleLockTaskbar` context command with checked UIA state.
- [x] 2.1.2 Add owned settings behavior row and typed lock mutation.
- [x] 2.1.3 Add pointer, keyboard, localization, save-failure, and reconciliation tests.

### 2.2 Quantize native resize to rows

**目的：** Convert native top-edge height changes into exact persistent rows.
**輸入：** Unlocked view state, window bounds, 40px row contract.
**產出：** Resize strip, bounds subscription, row quantizer, typed callback.
**依賴：** 1.2, 2.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-RESIZE`, `G-TASKBAR-LOCK`; deterministic/headful records.
**完成門檻：** Unlocked heights map to 1–3 rows once; locked state exposes no resize route.

- [x] 2.2.1 Add unlocked top-edge resize strip and native vertical-resize cursor.
- [x] 2.2.2 Observe bounds and quantize logical height to one, two, or three rows.
- [x] 2.2.3 Add threshold, duplicate, locked, DPI, and negative-origin tests.

### 2.3 Reconcile placement, HWND, settings, and AppBar

**目的：** Apply each accepted row atomically without bottom-edge or work-area drift.
**輸入：** Resize callback, settings store, monitor geometry, controlled AppBar lease.
**產出：** Saved row, snapped HWND, updated Shell reservation, Preview/Shell tests.
**依賴：** 2.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-PLACEMENT`, `G-TASKBAR-RESIZE`; app/platform records.
**完成門檻：** Preview anchors to work area, Shell anchors to monitor and reserves exact thickness.

- [x] 2.3.1 Preserve Preview work-bottom and Shell monitor-bottom anchors.
- [x] 2.3.2 Save changed rows before snapping exact DPI-scaled HWND geometry.
- [x] 2.3.3 Update the matching Shell AppBar lease when available, preserve explicit Explorer-free owned geometry when the broker is absent, and reject other save/lease failure.

### 2.4 Remove row separators without losing indicators

**目的：** Render multiple rows as one continuous Windows taskbar panel.
**輸入：** Current row renderer, outer border, running/progress indicators.
**產出：** Separator-free renderer and source/visual tests.
**依賴：** 2.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-CHROME`; source and theme screenshots.
**完成門檻：** One/two/three rows have no full-width internal line and retain outer border/indicators.

- [x] 2.4.1 Remove generated full-width row separator elements.
- [x] 2.4.2 Preserve outer top border and per-task indicator/progress geometry.
- [x] 2.4.3 Add one/two/three-row light/dark/high-contrast source and geometry tests.

## 3. Verification and Packaging

### 3.1 Prove resize and lock on the reference host

**目的：** Verify real pointer/UIA behavior with and without Explorer at 175% DPI.
**輸入：** Release app, owned context menu, resize harness, watchdog.
**產出：** Preview/Shell screenshots, drag/lock traces, process/AppBar evidence.
**依賴：** 2.3, 2.4.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-TASKBAR-PLACEMENT`, `G-TASKBAR-RESIZE`, `G-TASKBAR-LOCK`, `G-TASKBAR-CHROME`, `G-SHELL-NONINTERFERENCE`; headful records.
**完成門檻：** All rows, lock states, anchors, themes, UIA, AppBar and Explorer absence pass.

- [x] 3.1.1 Run fmt, locked/offline workspace tests, and clippy warnings-as-errors.
- [x] 3.1.2 Capture Preview drag/lock and separator-free 1–3 row matrices.
- [x] 3.1.3 Capture Explorer-free Shell drag/AppBar/UIA/process evidence and restore Explorer.

### 3.2 Validate traceability, release, and installers

**目的：** Produce reproducible completion evidence and distributable packages.
**輸入：** Passing automated/headful gates and admitted revision.
**產出：** Release hashes, unique evidence index, standalone and combined NSIS installers.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence/package records.
**完成門檻：** Every leaf has unique evidence; detailed/strict validation and both installers pass without launch.

- [x] 3.2.1 Build release binaries and record hashes.
- [x] 3.2.2 Build standalone and combined NSIS installers without launch and record hashes.
- [x] 3.2.3 Create unique evidence index and pass detailed and strict validation.
