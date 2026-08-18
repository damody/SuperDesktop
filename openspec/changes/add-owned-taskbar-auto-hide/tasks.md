# Owned Taskbar Auto-Hide Tasks

## 1. Contracts and deterministic state

### 1.1 Persist authoritative automatic-hide state

**目的：** Add one backward-compatible source of truth for automatic hiding.
**輸入：** Current taskbar settings schema, codec, and atomic store.
**產出：** `auto_hide` field, codec behavior, and settings tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-AUTOHIDE-SETTINGS`; automated evidence.
**完成門檻：** Legacy, valid, invalid, and sibling-isolation cases pass.

- [ ] 1.1.1 Add `auto_hide` with a false legacy default without changing schema version.
- [ ] 1.1.2 Decode, encode, and atomically round-trip the authoritative value.
- [ ] 1.1.3 Add legacy fallback, invalid-field, and independent-sibling tests.

### 1.2 Define pure auto-hide state and geometry

**目的：** Make timing and endpoint decisions deterministic before native integration.
**輸入：** Approved 500 ms delay, two-pixel edge, monitor/taskbar geometry.
**產出：** Reducer states, inputs, effects, and geometry helpers.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-AUTOHIDE-STATE`, `G-AUTOHIDE-GEOMETRY`; unit evidence.
**完成門檻：** Every state transition, threshold, DPI, and negative-origin case is deterministic.

- [ ] 1.2.1 Add `Visible`, `HidePending`, and `Hidden` state with typed effects.
- [ ] 1.2.2 Add exact Preview/Shell visible and two-physical-pixel hidden endpoint geometry.
- [ ] 1.2.3 Add reveal, 499/500 ms, duplicate tick, disabled, DPI, and negative-origin tests.

## 2. Owned UI and runtime integration

### 2.1 Enable the Windows-style settings control

**目的：** Replace the unavailable row with a truthful accessible toggle.
**輸入：** Authoritative setting and existing Windows 11 settings view.
**產出：** Localized row, typed save effect, accessibility state.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-AUTOHIDE-UI`; model/UIA evidence.
**完成門檻：** Pointer, keyboard, UIA, save failure, and refresh reconcile one authoritative value.

- [ ] 2.1.1 Replace the disabled auto-hide row with an enabled localized toggle.
- [ ] 2.1.2 Emit one typed atomic save effect and reconcile success/failure.
- [ ] 2.1.3 Add pointer, keyboard, accessibility, localization, and save-failure tests.

### 2.2 Add fail-closed cursor and HWND adapters

**目的：** Observe the pointer and move only the caller-owned taskbar.
**輸入：** Physical endpoint geometry and current-process HWND identity.
**產出：** Cursor observation and idempotent endpoint movement APIs.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-AUTOHIDE-NATIVE`, `G-SHELL-NONINTERFERENCE`; platform evidence.
**完成門檻：** Owned movement is exact; null, retired, foreign, and duplicate requests are zero-effect.

- [ ] 2.2.1 Add a typed physical cursor observation adapter.
- [ ] 2.2.2 Add current-process HWND validation and exact idempotent endpoint movement.
- [ ] 2.2.3 Add invalid, foreign, retired, duplicate, DPI, and source-contract tests.

### 2.3 Integrate visibility holds and timed reconciliation

**目的：** Drive Windows-like reveal/hide behavior from owned runtime state.
**輸入：** Reducer, cursor/geometry adapters, popup handles, focus, resize, and attention.
**產出：** Bounded 50 ms reconciliation and visibility traces.
**依賴：** 2.1, 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-AUTOHIDE-STATE`, `G-AUTOHIDE-INTERACTION`; app/headful evidence.
**完成門檻：** Edge reveal is immediate, hide delay is continuous, and every declared hold prevents hiding.

- [ ] 2.3.1 Compose pointer, focus, resize, popup, Start, flyout, and attention holds.
- [ ] 2.3.2 Run the reducer on the bounded timer and apply each endpoint exactly once.
- [ ] 2.3.3 Reconcile live setting changes, cursor failure, stale windows, and typed traces.

### 2.4 Reconcile AppBar and lifecycle behavior

**目的：** Avoid work-area reservation while hidden and always restore a visible safe endpoint.
**輸入：** Shell/Preview mode, AppBar lease, setting transitions, teardown paths.
**產出：** Reservation policy, disable recovery, and shutdown restoration.
**依賴：** 2.3.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-AUTOHIDE-LIFECYCLE`, `G-SHELL-NONINTERFERENCE`; lifecycle evidence.
**完成門檻：** Preview is non-mutating; Shell skips reservation while enabled; disable and shutdown restore visibility.

- [ ] 2.4.1 Skip Shell AppBar reservation while automatic hiding is authoritative.
- [ ] 2.4.2 Restore visible placement and ordinary reservation when automatic hiding is disabled.
- [ ] 2.4.3 Restore visible placement before normal teardown and add idempotent lifecycle tests.

## 3. Verification and packaging

### 3.1 Prove Windows behavior on the reference host

**目的：** Verify real pointer, geometry, settings, and Explorer-free behavior at 175% DPI.
**輸入：** Release app, headful harness, Preview Explorer, Shell watchdog.
**產出：** Visible/hidden/reveal/delay screenshots, geometry, traces, and process evidence.
**依賴：** 2.4.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-AUTOHIDE-UI`, `G-AUTOHIDE-INTERACTION`, `G-AUTOHIDE-GEOMETRY`, `G-SHELL-NONINTERFERENCE`; headful evidence.
**完成門檻：** Preview and Explorer-free Shell pass row, timing, edge, settings, and restoration checks.

- [ ] 3.1.1 Run fmt, locked/offline workspace tests, clippy warnings-as-errors, and release build.
- [ ] 3.1.2 Capture Preview one-to-three-row hide, delayed reveal, settings, and hold evidence.
- [ ] 3.1.3 Capture Explorer-free Shell endpoints, process absence, teardown, and Explorer restoration evidence.

### 3.2 Validate traceability and distributable packages

**目的：** Admit one reproducible revision with complete task evidence and installers.
**輸入：** Passing automated/headful gates and committed implementation.
**產出：** Binary/installer hashes, unique evidence index, strict validation.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 5.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence and packaging records.
**完成門檻：** Every leaf has unique evidence, both installers build without launch, strict/detailed validators pass, and the change stays unarchived.

- [ ] 3.2.1 Record admitted release binary hashes and implementation revision.
- [ ] 3.2.2 Build standalone and combined NSIS installers without launch and record hashes.
- [ ] 3.2.3 Create the 24-leaf evidence index and pass detailed plus strict validation without archiving.
