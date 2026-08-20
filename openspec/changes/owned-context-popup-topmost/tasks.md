## 1. Owned-shell runtime resilience

### 1.1 Add a fallible asynchronous application update primitive

**目的：** Prevent GPUI application borrow contention from panicking foreground futures.
**輸入：** Vendored GPUI `AsyncApp`, existing `AppCell` borrow and quit semantics.
**產出：** Backward-compatible `try_update` API and focused tests/source contract.
**依賴：** None.
**Owner／Wave：** Primary runtime owner／Wave 1.
**Gate／Evidence：** `G-ASYNC-NO-PANIC`; `evidence/index.jsonl` tasks 1.1.1–1.1.2.
**完成門檻：** Contention and released/quitting app states return errors; existing `update` remains available.

- [x] 1.1.1 Implement `AsyncApp::try_update` with `try_borrow_mut`, quit-state rejection, and `anyhow::Result`.
- [x] 1.1.2 Add compile/source coverage proving the fallible path exists without removing or weakening the existing API.

### 1.2 Migrate SuperDesktop asynchronous callbacks

**目的：** Remove every panic-prone post-await `AsyncApp::update` call from production SuperDesktop runtime paths.
**輸入：** 1.1 API, preview timers, pointer monitor, transfer, auto-hide, refresh, and shutdown futures.
**產出：** Fallible update handling with contextual console/trace reporting.
**依賴：** 1.1.
**Owner／Wave：** Primary application owner／Wave 1.
**Gate／Evidence：** `G-ASYNC-NO-PANIC`; `evidence/index.jsonl` tasks 1.2.1–1.2.3.
**完成門檻：** Production source contains no post-await infallible AsyncApp update; repeating loops continue after rejection; one-shot callbacks fail without panic.

- [x] 1.2.1 Migrate preview open/close and pointer-monitor timers to `try_update` with contextual rejection traces.
- [x] 1.2.2 Migrate transfer, auto-hide, and refresh loops to skip contended ticks and continue their bounded cadence.
- [x] 1.2.3 Migrate timed shutdown and add source-level regression coverage banning production `AsyncApp::update` at asynchronous sites.

### 1.3 Preserve AppBar-unavailable operation

**目的：** Keep the owned shell alive and interactive when Windows rejects initial AppBar registration.
**輸入：** Controlled shell lease initialization, taskbar geometry fallback, action trace.
**產出：** Explicit recoverable state and unit/source tests.
**依賴：** 1.2.
**Owner／Wave：** Primary shell lifecycle owner／Wave 1.
**Gate／Evidence：** `G-APPBAR-RECOVERY`; `evidence/index.jsonl` tasks 1.3.1–1.3.2.
**完成門檻：** AppBar rejection does not set terminal error or quit; shell hook and taskbar geometry remain operational.

- [x] 1.3.1 Make initial AppBar failure explicitly trace a degraded state while continuing owned taskbar setup.
- [x] 1.3.2 Add lifecycle/source coverage proving AppBar-unavailable does not enter terminal/quit paths and refresh remains installed.

## 2. Context popup z-order parity

### 2.1 Centralize one-time popup promotion

**目的：** Give all independent context popups one fail-closed, non-activating native z-order policy.
**輸入：** Existing `promote_owned_popup_topmost`, GPUI Window HWND adapter, console/trace reporting.
**產出：** Shared helper and failure tests/source assertions.
**依賴：** None; can proceed after OpenSpec artifacts.
**Owner／Wave：** Primary popup owner／Wave 1.
**Gate／Evidence：** `G-POPUP-TOPMOST`; `evidence/index.jsonl` tasks 2.1.1–2.1.2.
**完成門檻：** Helper promotes once, returns success only after native confirmation, and removes/reports failed popups.

- [x] 2.1.1 Implement a shared owned-context promotion helper with per-kind success/failure console and trace output.
- [x] 2.1.2 Add source tests proving the helper uses the platform boundary and contains no polling, sleep, or recurring worker.

### 2.2 Apply promotion to every independent right-click popup

**目的：** Close route gaps across task and system context menus.
**輸入：** 2.1 helper and popup creation closures.
**產出：** Promoted Jump List, taskbar context, and system-control context windows with truthful slot state.
**依賴：** 2.1.
**Owner／Wave：** Primary application owner／Wave 1.
**Gate／Evidence：** `G-POPUP-TOPMOST`; `evidence/index.jsonl` tasks 2.2.1–2.2.3.
**完成門檻：** All three routes promote before constructing/storing views; failure paths do not emit opened traces.

- [x] 2.2.1 Apply promotion and fail-closed handle storage to task application Jump Lists.
- [x] 2.2.2 Apply promotion and fail-closed handle storage to taskbar background context menus.
- [x] 2.2.3 Apply promotion and fail-closed handle storage to input/volume system-control context menus.

### 2.3 Prove native z-order, dismissal, and crash survival headfully

**目的：** Verify real Windows behavior rather than only source composition.
**輸入：** Release app/fixtures, physical pointer input, UI Automation, native style probes, redirected stderr.
**產出：** Focused UTIT reports for each popup kind and AppBar/async stress interval.
**依賴：** 1.1–1.3, 2.1–2.2.
**Owner／Wave：** Primary verification owner／Wave 2.
**Gate／Evidence：** `G-POPUP-TOPMOST`, `G-APPBAR-RECOVERY`, `G-ASYNC-NO-PANIC`; focused reports and `evidence/index.jsonl` tasks 2.3.1–2.3.4.
**完成門檻：** Native topmost and focus-loss dismissal pass for every route; process survives bounded degraded/stress run; stderr has no borrow panic; two consecutive runs validate.

- [x] 2.3.1 Extend physical-pointer UTIT to verify native topmost state for task Jump List and taskbar background menu.
- [x] 2.3.2 Extend physical-pointer UTIT to verify native topmost state for input and volume context menus.
- [x] 2.3.3 Verify focus-loss dismissal and bounded AppBar-unavailable/refresh/popup stress with no `RefCell already borrowed` output.
- [x] 2.3.4 Execute and validate two consecutive focused UTIT runs with zero failed or blocked cases.

## 3. Quality, release, and integration

### 3.1 Run code quality gates

**目的：** Detect regressions in GPUI, platform, UI, application, and UTIT code.
**輸入：** Completed Wave 1 implementation and Wave 2 scripts.
**產出：** Formatting, tests, and warnings-denied Clippy evidence.
**依賴：** 2.3 implementation complete.
**Owner／Wave：** Primary verifier／Wave 2.
**Gate／Evidence：** `G-RUST-QUALITY`; `evidence/index.jsonl` tasks 3.1.1–3.1.3.
**完成門檻：** Every command exits zero and no affected gate is skipped.

- [x] 3.1.1 Run `cargo fmt --all -- --check`, PowerShell parse, and `git diff --check`.
- [x] 3.1.2 Run affected GPUI/workspace package tests including platform-win, taskbar-ui, superdesktop-app, and superdesktop-utit.
- [x] 3.1.3 Run Clippy with `-D warnings` for every affected package/target.

### 3.2 Build complete release artifacts

**目的：** Prove runtime and popup fixes are present in distributable binaries.
**輸入：** Passing quality and headful gates.
**產出：** Complete release workspace and combined installer with hashes.
**依賴：** 2.3, 3.1.
**Owner／Wave：** Primary integrator／Wave 3.
**Gate／Evidence：** `G-RELEASE`; `evidence/index.jsonl` tasks 3.2.1–3.2.2.
**完成門檻：** Release workspace and installer builder exit zero; hashes and sizes are recorded.

- [x] 3.2.1 Build the full SuperDesktop release workspace with locked offline dependencies and hash required binaries.
- [x] 3.2.2 Build the combined installer without launching it and record path, size, and SHA-256.

### 3.3 Finalize evidence and scoped integration

**目的：** Complete the OpenSpec audit trail and preserve unrelated workspace changes.
**輸入：** All gate results and dirty-worktree inventory.
**產出：** Evidence index, completed tasks, strict validation, scoped nested/outer commits.
**依賴：** 3.2.
**Owner／Wave：** Primary integrator／Wave 3 exit.
**Gate／Evidence：** `G-INTEGRATION`; `evidence/index.jsonl` tasks 3.3.1–3.3.3.
**完成門檻：** Every leaf is evidence-backed; strict validation passes; only scoped paths/gitlink are committed.

- [x] 3.3.1 Write unique evidence records with commands, expected/actual results, exit status, hashes, gates, and timestamps.
- [x] 3.3.2 Complete tasks and run detailed-task, strict OpenSpec, placeholder, and contradiction checks.
- [x] 3.3.3 Commit scoped SuperDesktop changes and synchronize the outer repository gitlink without staging unrelated files.
