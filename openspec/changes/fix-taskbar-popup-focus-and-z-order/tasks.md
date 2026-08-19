## 1. Popup contracts and platform adapter

### 1.1 Non-activating topmost HWND promotion

**目的：** Provide one bounded Windows adapter that establishes preview z-order without changing focus.
**輸入：** Approved design and Windows taskbar adapter.
**產出：** Owned-popup promotion helper and focused platform tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** G-POPUP-HWND-OWNERSHIP and G-PREVIEW-NOACTIVATE.
**完成門檻：** All focused platform gates pass.

- [x] 1.1.1 Add current-process HWND validation and non-moving, non-sizing, non-activating `HWND_TOPMOST` promotion to the Windows taskbar adapter.
- [x] 1.1.2 Add focused adapter tests or source-contract checks for liveness, ownership, `HWND_TOPMOST`, and `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`.
- [x] 1.1.3 Run the focused `platform-win` test command and record exit status plus output hash in `evidence/unit/platform-popup-topmost.json`.

### 1.2 Context-menu activation lifecycle

**目的：** Dismiss the context popup at the window-activation boundary.
**輸入：** Existing context view and GPUI activation observer pattern.
**產出：** Retained activation subscription and lifecycle tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** G-CONTEXT-DEACTIVATE.
**完成門檻：** Focused taskbar UI gates pass.

- [x] 1.2.1 Retain a GPUI window-activation subscription in `TaskbarContextView` and invoke its dismiss callback when the popup becomes inactive.
- [x] 1.2.2 Add regression coverage for retained activation observation, deactivation dismissal, and descendant-focus safety.
- [x] 1.2.3 Run the focused `taskbar-ui` test command and record exit status plus output hash in `evidence/unit/context-deactivation.json`.

## 2. Preview composition integration

### 2.1 Fail-closed topmost preview admission

**目的：** Establish topmost stacking before exposing previews.
**輸入：** Work package 1.1 and preview composition.
**產出：** Integrated promotion, traces, and regression coverage.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** G-PREVIEW-TOPMOST and G-PREVIEW-FAIL-CLOSED.
**完成門檻：** Both preview sources pass composition gates.

- [x] 2.1.1 Integrate owned-popup promotion immediately after resolving the preview destination HWND and before constructing `TaskFlyoutView`.
- [x] 2.1.2 Add success/rejection traces and fail-closed cleanup for unresolved or unpromotable preview HWNDs.
- [x] 2.1.3 Extend preview composition tests for topmost admission, hover no-activation, click activation, cleanup, and Explorer-free source guards.
- [x] 2.1.4 Run the focused `superdesktop-app` preview tests and record exit status plus output hash in `evidence/unit/preview-composition.json`.

## 3. Integrated validation and evidence

### 3.1 Static and crate quality gates

**目的：** Prove formatting, compilation, and focused regressions.
**輸入：** Completed implementation and locked toolchain.
**產出：** Quality logs and indexed hashes.
**依賴：** 1.2 and 2.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** G-FORMAT, G-CHECK, and G-FOCUSED-TESTS.
**完成門檻：** Every static and crate quality gate passes.

- [x] 3.1.1 Run `cargo fmt --all -- --check` and record the result as task `3.1.1` in `evidence/quality/evidence-index.json`.
- [x] 3.1.2 Run locked compilation for `platform-win`, `taskbar-ui`, and `superdesktop-app` and record the result as task `3.1.2` in `evidence/quality/evidence-index.json`.
- [x] 3.1.3 Run the combined focused regression filter and record the result as task `3.1.3` in `evidence/quality/evidence-index.json`.

### 3.2 Headful behavior proof

**目的：** Verify both user-visible behaviors on Windows.
**輸入：** Built binary and interactive Windows session.
**產出：** Screenshots, reports, traces, and evidence index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** G-CONTEXT-HEADFUL and G-PREVIEW-HEADFUL.
**完成門檻：** Both headful scenarios pass with hashed evidence.

- [x] 3.2.1 Run the context-menu deactivation scenario and save its report, trace, and screenshot under `evidence/headful/context-deactivation/`.
- [x] 3.2.2 Run the overlapping hover-preview scenario and save z-order plus foreground-before/after observations under `evidence/headful/preview-topmost/`.
- [x] 3.2.3 Hash and index both headful scenarios with expected/actual results and reviewer disposition in `evidence/headful/evidence-index.json`.

### 3.3 Final traceability and completion review

**目的：** Close the change only with complete traceability.
**輸入：** All artifacts, tasks, and evidence.
**產出：** Strict validation and final review report.
**依賴：** 3.2.
**Owner／Wave：** Primary integrator / wave 5.
**Gate／Evidence：** G-OPENSPEC-STRICT and G-TRACEABILITY.
**完成門檻：** No failed, blocked, stale, P0, or P1 item remains.

- [x] 3.3.1 Run strict OpenSpec validation and record its exit status plus output hash in `evidence/final-review.json`.
- [x] 3.3.2 Review proposal-to-design-to-spec-to-task traceability and record every scenario mapping in `evidence/final-review.json`.
- [x] 3.3.3 Confirm all evidence records are current, no task is failed/blocked/stale, and the change is ready for archive review.
