# Notification Overflow Taskbar Geometry Tasks

## 1. Production geometry

### 1.1 Implement mode-aware overflow anchoring

**目的：** Eliminate shell double reservation without breaking preview mode.
**輸入：** Approved design and current overflow/taskbar helpers.
**產出：** Mode-aware bounds/options/composition.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-OVERFLOW-GEOMETRY`; focused tests.
**完成門檻：** Matching anchor, rows and 8 DIP gap apply once.

- [x] 1.1.1 Add shell input to overflow bounds/options.
- [x] 1.1.2 Select preview/shell taskbar bottom and bounded panel bottom.
- [x] 1.1.3 Pass runtime shell mode through overflow composition.

### 1.2 Freeze Windows layout ratios

**目的：** Preserve width/cells/rows across all synthetic geometries.
**輸入：** Mode-aware pure helper.
**產出：** DPI/mode/rows/icons/origin matrix.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-DPI-MATRIX`; Rust tests.
**完成門檻：** 4 DPI × 2 modes × 3 rows and icon/boundary matrix pass.

- [x] 1.2.1 Test stale work-area preview/shell anchors.
- [x] 1.2.2 Test icon counts 1/6/7/20/36+ with 344/48/6-column ratios.
- [x] 1.2.3 Test negative origins, constrained size and containment.

## 2. Forced Explorer-free UTIT

### 2.1 Capture owned overflow geometry and UIA

**目的：** Prove actual panel proportions with 20 NotifyIcons.
**輸入：** Existing compatibility fixture/script and release shell.
**產出：** Versioned JSON, screenshot, UIA/callback evidence.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-NOTIFYICON`, `G-UTIT`; headful report.
**完成門檻：** Panel/taskbar/monitor rectangles, DPI/DIP values, hidden buttons and hashes exist.

- [x] 2.1.1 Launch 20-icon fixture and deterministically open owned overflow.
- [x] 2.1.2 Record HWND/PID, rectangles, DPI, logical width/height/gap and containment.
- [x] 2.1.3 Preserve activate/context callbacks, host recovery, screenshot and hashes.

### 2.2 Enforce isolation and runtime thresholds

**目的：** Reject bad geometry, missing overflow, delegation or recovery.
**輸入：** Runtime measurements and runner catalog.
**產出：** Blocking assertions and shell-parity case.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SHELL-NONINTERFERENCE`, `G-UTIT`; focused/full reports.
**完成門檻：** Width ±16 DIP, gap 2–16 DIP, containment, ownership, callbacks and recovery pass.

- [x] 2.2.1 Enforce width/gap/containment/hidden-button thresholds.
- [x] 2.2.2 Add Explorer-watchdog UTIT catalog case and required artifacts.
- [x] 2.2.3 Run focused and full shell-parity and validate hashes.

## 3. Admission and packaging

### 3.1 Run complete gates

**目的：** Reject replacement-shell regressions.
**輸入：** Integrated feature/evidence.
**產出：** Test/lint/governance/release/spec/visual results.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-DPI-MATRIX`, `G-UTIT`, `G-TRACE`.
**完成門檻：** All locally runnable blocking gates exit zero.

- [x] 3.1.1 Run format and focused/workspace tests.
- [x] 3.1.2 Run Clippy, architecture/source audits and release.
- [x] 3.1.3 Inspect screenshot and pass strict/detailed validation.

### 3.2 Commit, package and trace

**目的：** Bind distributables and every leaf without archive.
**輸入：** Passing clean revision.
**產出：** Commits, gitlink, two no-launch packages and 18 records.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PACKAGE`, `G-TRACE`.
**完成門檻：** 18/18, verified installers, clean current gitlink, unarchived.

- [x] 3.2.1 Commit implementation and update parent gitlink.
- [x] 3.2.2 Build both installers `--no-launch` and verify hashes.
- [x] 3.2.3 Create 18 unique records, revalidate and commit without archive.
