# Resizable persistent Start menu tasks

## 1. Settings and geometry contracts

### 1.1 Persist optional logical Start dimensions

**目的：** Extend Start settings without breaking existing files or future-field preservation.
**輸入：** `StartSettings`, custom JSON decoder/encoder, atomic settings store, and schema tests.
**產出：** Optional width/height fields and codec tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-START-SCHEMA`; `evidence/1.1/*`.
**完成門檻：** Missing, valid, malformed, round-trip, and forward-compatible JSON cases pass.

- [ ] 1.1.1 Add optional logical width and height to StartSettings defaults and public schema.
- [ ] 1.1.2 Decode positive bounded integers and encode absent/value dimensions deterministically.
- [ ] 1.1.3 Add legacy-missing, valid round-trip, malformed, and future-field settings tests.

### 1.2 Normalize Start size and derive aligned geometry

**目的：** Restore any saved size as a fully visible monitor-relative Start window.
**輸入：** Monitor DPI/work area, taskbar rows/mode, alignment, Windows metrics, and saved dimensions.
**產出：** Pure size normalization and updated geometry matrix.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-START-GEOMETRY`; `evidence/1.2/*`.
**完成門檻：** Default/min/max/narrow/DPI/alignment/taskbar matrices preserve containment and gap.

- [ ] 1.2.1 Implement shared default, minimum, maximum, and current-monitor size normalization.
- [ ] 1.2.2 Apply normalized saved size to left/center and preview/shell Start positioning.
- [ ] 1.2.3 Extend geometry tests across 96–192 DPI, rows 1–3, narrow work areas, and invalid sizes.

## 2. Native resize and persistence lifecycle

### 2.1 Enable native Start resize with coalesced observation

**目的：** Resize the titlebar-free popup smoothly without a persistence write storm.
**輸入：** GPUI WindowOptions, window-bound observation, StartView lifecycle, and resize callback.
**產出：** Native resize styles, 250ms generation fence, and view tests.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-START-RESIZE`; `evidence/2.1/*`.
**完成門檻：** Native edges resize; unchanged initial size is ignored; a drag emits one latest callback.

- [ ] 2.1.1 Enable native resizing while retaining titlebar-free popup and non-movable behavior.
- [ ] 2.1.2 Add StartView bounds observation with unchanged suppression and 250ms generation-fenced debounce.
- [ ] 2.1.3 Add initial/no-op, continuous-drag, latest-value, and subscription-lifetime tests.

### 2.2 Flush and atomically save terminal size

**目的：** Preserve the latest size even when Start closes near the end of a drag.
**輸入：** Debounced size, deactivation event, taskbar toggle close, shared settings, and SettingsStore.
**產出：** Flush paths, atomic persistence callback, traces, and failure tests.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-START-PERSIST`; `evidence/2.2/*`.
**完成門檻：** Debounce/deactivation/close save latest size once; failure retains prior settings and traces error.

- [ ] 2.2.1 Flush latest observed size on deactivation and before taskbar-toggle removal.
- [ ] 2.2.2 Persist normalized dimensions atomically and update shared settings only after success.
- [ ] 2.2.3 Add close-race, deactivation, unchanged, save-failure, and revision-reconciliation tests.

## 3. Physical verification and release

### 3.1 Prove resize persistence across reopen and restart

**目的：** Verify production pointer resizing and persisted restoration rather than only model callbacks.
**輸入：** Release candidate, isolated settings, UIA/HWND geometry, native pointer drag, restart watchdog, and DPI metadata.
**產出：** Two clean headful reports and privacy-safe geometry evidence.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PHYSICAL-START-RESIZE`, `G-NO-CRASH`; `evidence/3.1/*`.
**完成門檻：** Resize to two sizes, close/reopen, and process restart restore within rounding tolerance twice.

- [ ] 3.1.1 Add a focused physical script for native resize, close/reopen, settings inspection, and process restart.
- [ ] 3.1.2 Run two clean physical cycles against one candidate hash and inspect window containment/UIA.
- [ ] 3.1.3 Scan reports/logs for write storms, stale settings, panic, borrow errors, and sensitive data.

### 3.2 Pass automated, packaging, and traceability gates

**目的：** Deliver the verified sizing behavior through the normal installer and nested-parent integration.
**輸入：** Passing physical candidate, complete tests, OpenSpec artifacts, and installer build.
**產出：** Automated reports, installer hash match, commits, and 18-record evidence index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-AUTOMATED`, `G-OPENSPEC`, `G-PACKAGE`; `evidence/3.2/*`.
**完成門檻：** Format/tests/Clippy/release/strict validation pass, installer hash matches, tracked worktrees are clean, and change remains unarchived.

- [ ] 3.2.1 Run formatting, focused and locked/offline workspace tests, and Clippy warnings-as-errors.
- [ ] 3.2.2 Run release build, installer without launch, and packaged app hash comparison.
- [ ] 3.2.3 Write 18 unique evidence records, commit nested/parent revisions, and rerun strict/status checks.
