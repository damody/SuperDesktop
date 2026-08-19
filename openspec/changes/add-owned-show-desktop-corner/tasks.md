# Owned Show Desktop Corner Tasks

## 1. Reversible model and native boundary

### 1.1 Implement the pure exact-identity session reducer

**目的：** Define deterministic first-click admission and second-click restore policy.
**輸入：** Approved design, native window snapshot fields, existing task eligibility rules.
**產出：** Pure target/session/effect model and focused unit tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-SHOW-DESKTOP-MODEL`; reducer test report.
**完成門檻：** Eligible targets are exact, ordered, bounded, reversible, and stale-safe.

- [ ] 1.1.1 Add exact target identity, session state, activation effect, and success reconciliation types.
- [ ] 1.1.2 Implement first-click admission and active-session fresh-snapshot intersection.
- [ ] 1.1.3 Test empty, pre-minimized, partial failure, stale/reused identity, new-window, and repeated cycles.

### 1.2 Add validated non-activating native restore

**目的：** Apply reducer effects without focus theft or shell delegation.
**輸入：** Reducer effects, current documented Win32 action boundary.
**產出：** Non-activating restore action, live identity checks, source tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-SHOW-DESKTOP-NATIVE`, `G-SHELL-NONINTERFERENCE`; native report.
**完成門檻：** Minimize/restore mutate only validated live targets and never synthesize shell input.

- [ ] 1.2.1 Add documented `Restore` action while retaining activating task-button behavior.
- [ ] 1.2.2 Wire authoritative snapshot matching and record only successfully minimized targets.
- [ ] 1.2.3 Test invalid HWND, PID/identity mismatch, destroyed target, partial action failure, and no forbidden API path.

## 2. Windows 11 taskbar surface and interaction

### 2.1 Render the far-edge corner across taskbar layouts

**目的：** Match the Windows 11 right-edge target and visual states.
**輸入：** Existing status-region layout, taskbar rows, theme and contrast tokens.
**產出：** Full-height 8-DIP control with responsive Windows-style states.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SHOW-DESKTOP-UI`; layout/theme report.
**完成門檻：** The control is flush right at 100–225% DPI and continuous across one to three rows.

- [ ] 2.1.1 Add the final 8-DIP full-height button after the clock and reserve exact status width.
- [ ] 2.1.2 Add light, dark, high-contrast, hover, pressed, and focus edge treatments.
- [ ] 2.1.3 Test 100–225% DPI, negative origin, one-to-three rows, no right gap, and no row separators.

### 2.2 Wire equivalent accessible typed activation

**目的：** Make pointer, keyboard, and UIA use one owned action path.
**輸入：** GPUI control, app session callback, localization conventions.
**產出：** Typed callback, Button semantics, localized name, Enter/Space behavior.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SHOW-DESKTOP-A11Y`, `G-SHELL-NONINTERFERENCE`; accessibility report.
**完成門檻：** All supported modalities invoke exactly one owned cycle and unsupported input is zero-effect.

- [ ] 2.2.1 Add the app-owned callback and reducer/native execution bridge.
- [ ] 2.2.2 Add stable Button role/name/id, Traditional Chinese label, focus, Enter, and Space.
- [ ] 2.2.3 Test callback count, localization, UIA source contract, unsupported keys, and zero delegation.

## 3. Explorer-absent admission and packaging

### 3.1 Verify a real reversible production cycle

**目的：** Prove GUI geometry and function on the Windows 11 175% host without Explorer.
**輸入：** Release app, controlled three-window fixture, watchdog capture harness.
**產出：** Screenshots, UIA/action trace, exact window-state and process evidence.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-SHOW-DESKTOP-UI`, `G-SHOW-DESKTOP-A11Y`, `G-SHELL-NONINTERFERENCE`; `evidence/headful/*`.
**完成門檻：** First click minimizes only two visible fixtures, second restores only those two, and Explorer is safely restored after capture.

- [ ] 3.1.1 Add controlled visible/pre-minimized fixture windows and watchdog capture harness.
- [ ] 3.1.2 Capture light, dark, high contrast, one/multi-row, hover/focus, far-edge, and 175% bounds.
- [ ] 3.1.3 Prove UIA and pointer cycles, pre-minimized preservation, stale-target safety, and forbidden-process absence.

### 3.2 Run full gates and admit packages

**目的：** Produce distributable, traceable artifacts without archive mutation.
**輸入：** Passing implementation/headful capture and committed revision.
**產出：** Full gate report, release/package hashes, both NSIS installers, 18-leaf evidence index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence index and package report.
**完成門檻：** 18/18 unique passed leaves, strict/detailed validation, both installers, unarchived change.

- [ ] 3.2.1 Run fmt, locked/offline workspace tests, Clippy warnings-as-errors, and release build.
- [ ] 3.2.2 Commit implementation and build standalone plus combined NSIS installers without launch.
- [ ] 3.2.3 Record revision/binary/package hashes, create 18 unique evidence records, and pass detailed/strict validation.
