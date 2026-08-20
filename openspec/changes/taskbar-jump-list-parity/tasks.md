## 1. Popup and Jump List implementation

### 1.1 Make task preview cancellation explicit

**目的：** Ensure a task context request invalidates every pending or visible preview before any Jump List resolution path.
**輸入：** Approved design, `HoverPreviewController`, preview slot/HWND state, task-context callback.
**產出：** Controller cancellation API, coordinated popup transition, deterministic unit coverage.
**依賴：** None.
**Owner／Wave：** Primary implementation owner／Wave 1.
**Gate／Evidence：** `G-POPUP-EXCLUSIVE`; `evidence/index.jsonl` records for tasks 1.1.1–1.1.3.
**完成門檻：** Pending timers cannot open after cancellation, visible previews are removed, and every task-context early return occurs after cancellation.

- [x] 1.1.1 Add `HoverPreviewController::cancel` and unit-test task, popup-hover, and stale-generation reset behavior.
- [x] 1.1.2 Capture the preview controller, preview window slot, and active preview HWND in the task-context callback and cancel/remove/clear them before Jump List toggle or lookup.
- [x] 1.1.3 Add source-level regression coverage proving preview cancellation precedes task snapshot/provider early-return paths.

### 1.2 Make destination data truthful and application-scoped

**目的：** Remove unrelated global Recent/Frequent items while preserving verified executable tasks and local fallback actions.
**輸入：** `platform-win` Jump List provider, provider protocol response, application identity.
**產出：** Fail-closed provider enumeration and provider/dispatcher tests.
**依賴：** Proposal ownership decision.
**Owner／Wave：** Primary platform owner／Wave 1.
**Gate／Evidence：** `G-DESTINATION-OWNERSHIP`; `evidence/index.jsonl` records for tasks 1.2.1–1.2.2.
**完成門檻：** Invalid or valid executable requests never emit unverified global Recent/Frequent items, and valid executable tasks remain actionable.

- [x] 1.2.1 Remove global `FOLDERID_Recent` enumeration and return empty Recent/Frequent groups unless ownership is proven.
- [x] 1.2.2 Update platform/provider tests to reject unrelated destination labels while retaining the canonical executable `Open new window` task.

### 1.3 Align required bottom commands with File Explorer

**目的：** Present a stable bottom command area with one pin state and one applicable close action.
**輸入：** Captured application windows, persisted taskbar pins, `JumpListModel` and `JumpListView`.
**產出：** Correct command composition, ordering, labels, invocation, and rendering.
**依賴：** 1.1, 1.2.
**Owner／Wave：** Primary application/UI owner／Wave 1.
**Gate／Evidence：** `G-EXPLORER-COMMANDS`; `evidence/index.jsonl` records for tasks 1.3.1–1.3.4.
**完成門檻：** Single-window lists contain pin/unpin then `Close window`; grouped lists contain pin/unpin then `Close all windows`; no duplicate close or `Actions` heading exists.

- [x] 1.3.1 Recompose local commands so optional minimize/maximize precede pin/unpin and exactly one final close command.
- [x] 1.3.2 Route the single and grouped close commands through captured exact identities and report partial/rejected execution truthfully.
- [x] 1.3.3 Render the local group without a synthetic heading while preserving accessible menu semantics and separator spacing.
- [x] 1.3.4 Add model/application source tests for single/group command labels, order, uniqueness, and pin persistence failure reporting.

## 2. Headful and automated verification

### 2.1 Extend focused physical-pointer UTIT

**目的：** Prove the real taskbar interaction rather than only model/source behavior.
**輸入：** `gui-taskbar-window-actions`, fixture window, UI Automation, action trace.
**產出：** Extended script and machine-readable `window-actions/report.json`.
**依賴：** 1.1–1.3.
**Owner／Wave：** Primary verification owner／Wave 2.
**Gate／Evidence：** `G-UTIT-INTERACTION`; focused UTIT report and `evidence/index.jsonl` records for tasks 2.1.1–2.1.3.
**完成門檻：** Physical hover/right-click proves immediate and delayed preview absence, required commands, and exact minimize/maximize/close actions in two consecutive runs.

- [x] 2.1.1 Extend the fixture flow to establish hover preview state, right-click, and assert the `Window previews` surface is absent when the Jump List appears.
- [x] 2.1.2 Assert preview absence again after the configured hover delay and record pin/unpin plus applicable close command order.
- [x] 2.1.3 Execute two consecutive focused runs and validate both run reports with zero failed or blocked cases.

### 2.2 Run package quality gates

**目的：** Detect regressions across platform, UI, application, provider, and UTIT packages.
**輸入：** Completed implementation and tests.
**產出：** Formatting, test, and Clippy results indexed as evidence.
**依賴：** 2.1 implementation complete; focused run may execute before or after these independent gates.
**Owner／Wave：** Primary verification owner／Wave 2.
**Gate／Evidence：** `G-RUST-QUALITY`; `evidence/index.jsonl` records for tasks 2.2.1–2.2.3.
**完成門檻：** Every command exits zero; warnings are denied; no gate is skipped.

- [x] 2.2.1 Run `cargo fmt --all -- --check` and `git diff --check`.
- [x] 2.2.2 Run tests for `platform-win`, `shell-provider-host`, `taskbar-ui`, `superdesktop-app`, and `superdesktop-utit`.
- [x] 2.2.3 Run Clippy with `-D warnings` for every affected package.

## 3. Release integration and evidence

### 3.1 Build and package the integrated product

**目的：** Prove the change is present in complete release binaries and the combined installer.
**輸入：** Passing Wave 2 gates and clean scoped source diff.
**產出：** Release workspace binaries and versioned combined installer.
**依賴：** 2.1, 2.2.
**Owner／Wave：** Primary integrator／Wave 3.
**Gate／Evidence：** `G-RELEASE`; binary/installer hashes and `evidence/index.jsonl` records for tasks 3.1.1–3.1.2.
**完成門檻：** Workspace release build and installer builder exit zero, and their SHA-256 hashes are recorded.

- [ ] 3.1.1 Build the complete SuperDesktop release workspace with locked offline dependencies and record required binary hashes.
- [ ] 3.1.2 Build the combined SuperExplorer/SuperDesktop installer without launching it and record its path, size, and SHA-256.

### 3.2 Close evidence and integrate commits

**目的：** Preserve auditable results while avoiding unrelated user workspace changes.
**輸入：** All gate outputs and existing dirty-worktree inventory.
**產出：** Completed task checklist, evidence index, scoped SuperDesktop commit, and outer gitlink commit.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator／Wave 3 exit.
**Gate／Evidence：** `G-INTEGRATION`; `evidence/index.jsonl` records for tasks 3.2.1–3.2.3.
**完成門檻：** Every L3 is evidence-backed and checked, OpenSpec validates strictly, only scoped paths are committed, and unrelated files remain untouched.

- [ ] 3.2.1 Write unique task evidence records with commands, expected/actual results, exit status, hashes, gate IDs, and timestamps.
- [ ] 3.2.2 Mark completed tasks, run strict OpenSpec validation, and scan artifacts for placeholders or contradictions.
- [ ] 3.2.3 Commit scoped SuperDesktop changes and the outer repository gitlink without staging unrelated modifications.
