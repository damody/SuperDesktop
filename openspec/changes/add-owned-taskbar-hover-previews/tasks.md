# Owned Taskbar Hover Previews Tasks

## 1. Hover lifetime model and composition

### 1.1 Implement the stale-safe hover controller

**目的：** Freeze deterministic open/close timing independent of GPUI/native state.
**輸入：** Approved timing design and existing flyout model.
**產出：** Generation controller, tokens, effects, unit tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-HOVER-MODEL`; focused tests.
**完成門檻：** Early leave, stable enter, rapid switch, popup crossing, close grace, and stale timers are exact.

- [x] 1.1.1 Add task/popup hover state, generation tokens, and open/close predicates.
- [x] 1.1.2 Add 400 ms open and 250 ms close constants/effect transitions.
- [x] 1.1.3 Test boundary timing, switches, crossing, repeated cycles, and stale tokens.

### 1.2 Wire fresh single/group preview composition

**目的：** Open owned previews for every task without changing click semantics.
**輸入：** Controller effects, authoritative task snapshots, current popup slot.
**產出：** Shared card resolver/open path and GPUI scheduling.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-HOVER-COMPOSITION`, `G-SHELL-NONINTERFERENCE`; app tests.
**完成門檻：** Single/group hover resolves fresh identities; stale/empty/disabled paths are zero-effect.

- [x] 1.2.1 Add typed task-hover and popup-hover callbacks.
- [x] 1.2.2 Refactor shared single/group card resolution and owned popup opening.
- [x] 1.2.3 Add executor timers, exact popup-slot dismissal, and source no-delegation tests.

## 2. Windows presentation and automated GUI gate

### 2.1 Align preview card presentation and accessibility

**目的：** Match Windows live-thumbnail popup geometry and interaction.
**輸入：** Existing DWM cards, theme tokens, typed actions.
**產出：** Content-sized popup, theme/focus/hover states, UIA/keyboard behavior.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-HOVER-UI`, `G-HOVER-A11Y`; UI/source tests.
**完成門檻：** Single/group light/dark/high-contrast cards remain actionable and never use fixed empty height.

- [x] 2.1.1 Add Windows light/dark/high-contrast popup/card/close/focus tokens.
- [x] 2.1.2 Make geometry content-sized and preserve live DWM canvas plus truthful fallback.
- [x] 2.1.3 Test Button roles/names, Enter/Delete/Escape, pointer actions, geometry, and RAII source contracts.

### 2.2 Add real-pointer UTIT hover coverage

**目的：** Prove production hover timing and recovery with Explorer absent.
**輸入：** Release app, controlled windows, UTIT runner and watchdog.
**產出：** Capture script, catalog case, screenshots, JSON/JUnit/Markdown evidence.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-UTIT`, `G-SHELL-NONINTERFERENCE`; `evidence/runs/*`.
**完成門檻：** Early/delayed/crossing/close/switch/UIA/recovery observations all pass against one binary hash.

- [x] 2.2.1 Add controlled single/group fixture and real pointer hover capture.
- [x] 2.2.2 Add UTIT catalog/artifact/recovery contract and focused runner tests.
- [x] 2.2.3 Execute focused and shell-parity UTIT runs and validate every report hash.

## 3. Full admission and packaging

### 3.1 Run complete automated and visual gates

**目的：** Reject regressions across the replacement shell.
**輸入：** Integrated hover implementation and UTIT evidence.
**產出：** Workspace tests, Clippy, release, strict/detailed reports.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-HOVER-*`, `G-UTIT`, `G-TRACE`; automated evidence.
**完成門檻：** All automated gates exit zero and current-host shell-parity remains passed.

- [x] 3.1.1 Run fmt and locked/offline focused plus workspace tests.
- [x] 3.1.2 Run Clippy warnings-as-errors, architecture, release, and source boundaries.
- [x] 3.1.3 Inspect screenshots and validate strict OpenSpec plus detailed task structure.

### 3.2 Commit, package, and finalize traceability

**目的：** Produce distributable, hash-bound results without archive mutation.
**輸入：** Passing gates and clean implementation revision.
**產出：** Commits, both NSIS packages, hashes, 18-leaf evidence index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PACKAGE`, `G-TRACE`; package/final evidence.
**完成門檻：** 18/18 leaves, both installers, validated reports, and unarchived change.

- [x] 3.2.1 Commit implementation and update the SuperExplorer submodule pointer.
- [ ] 3.2.2 Build standalone and combined NSIS installers without launch and record hashes.
- [ ] 3.2.3 Create 18 unique evidence records, revalidate, commit evidence, and keep change unarchived.
