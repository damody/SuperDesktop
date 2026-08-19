# Owned Start and Taskbar Geometry Tasks

## 1. Production geometry

### 1.1 Implement mode-aware Start placement

**目的：** Eliminate Start/taskbar overlap in preview and shell modes.
**輸入：** Approved design and existing taskbar geometry contract.
**產出：** Mode/row-aware pure helper and options wiring.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-START-GEOMETRY`; focused tests.
**完成門檻：** Matching taskbar anchor, row height and 12 DIP gap are applied exactly once.

- [x] 1.1.1 Add shell and row inputs to Start geometry/options.
- [x] 1.1.2 Compute matching preview/shell taskbar top and bounded Start size.
- [x] 1.1.3 Pass runtime mode and settings rows through Start composition.

### 1.2 Freeze DPI, row and boundary ratios

**目的：** Preserve Windows dimensions on every supported synthetic geometry.
**輸入：** Mode-aware Start helper.
**產出：** DPI/mode/row/origin/constrained test matrix.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-DPI-MATRIX`; Rust tests.
**完成門檻：** 4 DPI × 2 modes × 3 rows plus stale/negative/constrained cases pass.

- [x] 1.2.1 Test preview and shell anchors with stale work-area reservation.
- [x] 1.2.2 Test 640×720 preference across DPI and 1–3 rows.
- [x] 1.2.3 Test negative origins, small monitors, centering and non-overlap.

## 2. Explorer-free UTIT Start admission

### 2.1 Capture authoritative runtime geometry

**目的：** Measure actual Start/taskbar HWND results and preserve UI semantics.
**輸入：** Existing Start capture, release app and watchdog pattern.
**產出：** Versioned geometry report and screenshots.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-UTIT`, `G-OWNED-START`; headful report.
**完成門檻：** Actual rectangles, DPI, logical size/gap, containment, PIDs and home/all-apps/power evidence exist.

- [x] 2.1.1 Add Explorer suppression, watchdog, shell launch and recovery fields.
- [x] 2.1.2 Record Start/taskbar/monitor geometry and derived DIP values.
- [x] 2.1.3 Preserve UIA sections/actions, screenshots, locale and binary hashes.

### 2.2 Enforce geometry and system-host isolation

**目的：** Fail incorrect proportions, overlap, delegation or recovery.
**輸入：** Versioned runtime measurements and host PID snapshots.
**產出：** Blocking UTIT assertions and runner catalog contract.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SHELL-NONINTERFERENCE`, `G-UTIT`; focused/shell reports.
**完成門檻：** Width ±16 DIP, gap 4–20 DIP, non-overlap, containment, no new system host and recovery all pass.

- [x] 2.2.1 Enforce width, gap, containment and non-overlap thresholds.
- [x] 2.2.2 Mark `gui-start` Explorer-free and reject new system Start/Search/Shell PIDs.
- [x] 2.2.3 Run focused and full shell-parity UTIT and validate report hashes.

## 3. Full admission and packaging

### 3.1 Run quality and visual gates

**目的：** Reject replacement-shell regressions.
**輸入：** Integrated implementation and UTIT evidence.
**產出：** Test/lint/governance/release/spec/visual results.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-DPI-MATRIX`, `G-UTIT`, `G-TRACE`; automated evidence.
**完成門檻：** Every locally runnable blocking gate exits zero.

- [x] 3.1.1 Run format and focused/workspace locked offline tests.
- [x] 3.1.2 Run Clippy, architecture/source audits and release build.
- [x] 3.1.3 Inspect screenshots and pass strict/detailed validation.

### 3.2 Commit, package and trace

**目的：** Bind distributables and all leaves without archiving.
**輸入：** Passing gates and clean implementation revision.
**產出：** Commits, parent gitlink, two no-launch packages and 18-record index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PACKAGE`, `G-TRACE`; final evidence.
**完成門檻：** 18/18, clean submodule, current gitlink, verified hashes and unarchived change.

- [x] 3.2.1 Commit implementation and update parent gitlink.
- [x] 3.2.2 Build both NSIS installers with `--no-launch` and verify hashes.
- [x] 3.2.3 Create 18 unique evidence records, revalidate and commit without archive.
