# System Flyout and Taskbar Geometry Tasks

## 1. Geometry contract and production wiring

### 1.1 Implement explicit preview/shell geometry

**目的：** Remove double taskbar reservation while preserving both runtime layouts.
**輸入：** Approved design, `taskbar_physical_geometry`, current system-flyout helper.
**產出：** Mode-aware pure geometry and options wiring.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-FLYOUT-GEOMETRY`; focused Rust tests.
**完成門檻：** Preview uses work-area anchor, shell uses bounds anchor, and both subtract owned taskbar height/gap once.

- [x] 1.1.1 Add explicit shell-mode input to geometry and option helpers.
- [x] 1.1.2 Compute preview and shell taskbar-top anchors from their matching taskbar contract.
- [x] 1.1.3 Pass runtime mode through the owned system-flyout composition path.

### 1.2 Add the logical DPI and boundary matrix

**目的：** Freeze Windows proportions across scale, rows, origins, and constrained screens.
**輸入：** Mode-aware pure helper and preferred kind sizes.
**產出：** Exhaustive focused geometry tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-DPI-MATRIX`; test names and console results.
**完成門檻：** Four DPI values, two modes, three rows, four kinds, negative origin, stale work area, and constrained bounds pass.

- [x] 1.2.1 Test preview and shell bottom anchors with retained work-area reservation.
- [x] 1.2.2 Test preferred widths/heights and containment for 96/144/168/216 DPI and 1–3 rows.
- [x] 1.2.3 Test negative-origin and constrained-monitor clamping without double conversion.

## 2. UTIT authoritative geometry admission

### 2.1 Extend real HWND geometry capture

**目的：** Measure the actual GPUI/DWM result rather than only source constants.
**輸入：** Existing Explorer-free system-status capture and owned popup sequence.
**產出：** Per-kind typed geometry records in versioned JSON.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-UTIT`, `G-FLYOUT-GEOMETRY`; `evidence/headful/headful-report.json`.
**完成門檻：** All four kinds record physical bounds, DPI/logical dimensions, bottom gap, containment and replacement identity.

- [x] 2.1.1 Capture taskbar, popup, monitor and DPI geometry for every owned flyout kind.
- [x] 2.1.2 Derive logical width/height/gap and version the report schema.
- [x] 2.1.3 Preserve screenshot hashes, input-profile restoration and Explorer watchdog evidence.

### 2.2 Enforce Windows ratio and noninterference thresholds

**目的：** Turn measured geometry and shell isolation into blocking failures.
**輸入：** Versioned geometry records and preferred dimensions.
**產出：** Fail-fast containment, width, gap, replacement and recovery assertions.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SHELL-NONINTERFERENCE`, `G-UTIT`; focused and shell-parity reports.
**完成門檻：** Every popup gap is 2–16 DIP, widths meet tolerance, one popup replaces another, and Explorer remains absent/recovered.

- [x] 2.2.1 Enforce per-kind preferred-width tolerance and monitor containment.
- [x] 2.2.2 Enforce 2–16 DIP taskbar gap and exact owned-popup replacement.
- [x] 2.2.3 Run focused system-status and full shell-parity UTIT and validate report hashes.

## 3. Full admission, evidence and packaging

### 3.1 Run complete quality and visual gates

**目的：** Reject regressions elsewhere in the replacement shell.
**輸入：** Integrated implementation and authoritative UTIT geometry evidence.
**產出：** Formatting, tests, lint, governance, release, strict/detailed and visual results.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-DPI-MATRIX`, `G-UTIT`, `G-TRACE`; automated evidence.
**完成門檻：** Every locally runnable blocking gate exits zero against one implementation revision.

- [x] 3.1.1 Run format and locked/offline focused plus workspace tests.
- [x] 3.1.2 Run Clippy warnings-as-errors, architecture/source-boundary audits and release build.
- [x] 3.1.3 Inspect screenshots and pass strict OpenSpec plus detailed-task validation.

### 3.2 Commit, package and finalize traceability

**目的：** Bind distributables and every atomic leaf without archiving.
**輸入：** Passing gates, screenshots and clean implementation revision.
**產出：** Commits, parent gitlink, two no-launch NSIS packages, hashes and 18-record index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PACKAGE`, `G-TRACE`; package/final evidence.
**完成門檻：** 18/18 leaves, validated artifacts, clean submodule, current parent gitlink, both packages and unarchived change.

- [x] 3.2.1 Commit implementation and update the SuperExplorer submodule pointer.
- [x] 3.2.2 Build standalone and combined NSIS installers with `--no-launch` and verify hashes.
- [x] 3.2.3 Create 18 unique evidence records, revalidate, commit evidence and keep the change unarchived.
