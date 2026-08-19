# Taskbar Preview Anchor and Focus Tasks

## 1. Policy and geometry

### 1.1 Add typed preview-open focus policy

**目的：** Make activation and keyboard-focus behavior explicit for hover and click.
**輸入：** Existing shared preview opener and grouped click/hover call sites.
**產出：** Typed source policy with focused unit/source tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-FOCUS`; focused Rust tests.
**完成門檻：** Hover is non-activating/non-focusing and click remains activating/focusing at every layer.

- [x] 1.1.1 Add `Hover` and `Click` source variants with exact policy methods.
- [x] 1.1.2 Pass the source explicitly from hover and grouped-click call sites.
- [x] 1.1.3 Apply policy to window options, native activation, and view focus with tests.

### 1.2 Implement source-relative clamped geometry

**目的：** Place previews above their source instead of at monitor center.
**輸入：** Physical cursor coordinate, monitor work area/DPI, card count.
**產出：** Pure logical geometry and popup options.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-ANCHOR`; geometry matrix tests.
**完成門檻：** Interior, edge, negative-origin, DPI, fallback, and 1-4 card cases remain within bounds.

- [x] 1.2.1 Add pure geometry result and physical-to-logical anchor conversion.
- [x] 1.2.2 Clamp both edges and retain monitor-center fallback for unavailable pointer.
- [x] 1.2.3 Test 96-216 DPI, negative origins, both edges, fallback, and card widths.

## 2. Composition and UTIT admission

### 2.1 Wire non-activating hover and focused click behavior

**目的：** Preserve the foreground application during hover without regressing grouped click accessibility.
**輸入：** Source policy, geometry helper, existing `TaskFlyoutView`.
**產出：** Conditional popup activation/focus and unchanged typed actions.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-FOCUS`, `G-SHELL-NONINTERFERENCE`; app/UI tests.
**完成門檻：** Hover never activates/focuses; click keyboard behavior and pointer/UIA actions remain intact.

- [x] 2.1.1 Capture the physical source anchor during preview admission.
- [x] 2.1.2 Make popup activation and internal focus conditional on open source.
- [x] 2.1.3 Add regression tests for both paths and Explorer-free source boundaries.

### 2.2 Extend the real-pointer UTIT gate

**目的：** Prove production placement and focus behavior with Explorer absent.
**輸入：** Release app, controlled fixtures, existing hover capture and watchdog.
**產出：** Versioned JSON evidence, screenshot, catalog pass, report hashes.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-UTIT`, `G-ANCHOR`, `G-FOCUS`; `evidence/headful/*`.
**完成門檻：** Foreground HWND is preserved, popup is contained and anchored within two physical pixels, and recovery passes.

- [x] 2.2.1 Record foreground, source/popup/monitor rectangles, expected center, delta, and clamp state.
- [x] 2.2.2 Fail the case on activation, containment, anchor, Explorer absence, or recovery violations.
- [x] 2.2.3 Run focused and shell-parity UTIT and validate JSON/JUnit/Markdown hashes.

## 3. Full admission and packaging

### 3.1 Run complete automated and visual gates

**目的：** Reject regressions across the replacement shell.
**輸入：** Integrated anchor/focus behavior and current UTIT evidence.
**產出：** Formatting, tests, lint, architecture/source, release, strict/detailed results.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-ANCHOR`, `G-FOCUS`, `G-UTIT`, `G-TRACE`; automated evidence.
**完成門檻：** Every locally runnable blocking gate exits zero against one source revision.

- [x] 3.1.1 Run format and locked/offline focused plus workspace tests.
- [x] 3.1.2 Run Clippy warnings-as-errors, architecture/source checks, and release build.
- [x] 3.1.3 Inspect the popup screenshot and validate strict OpenSpec plus detailed tasks.

### 3.2 Commit, package, and finalize traceability

**目的：** Produce hash-bound distributables without archiving.
**輸入：** Passing gates, clean implementation revision, captured evidence.
**產出：** Commits, both NSIS installers, hashes, and 18-leaf evidence index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PACKAGE`, `G-TRACE`; package and final evidence.
**完成門檻：** 18/18 leaves, both installers, validated reports, parent gitlink update, and unarchived change.

- [x] 3.2.1 Commit implementation and update the SuperExplorer submodule pointer.
- [x] 3.2.2 Build standalone and combined NSIS installers without launch and record hashes.
- [x] 3.2.3 Create 18 unique evidence records, revalidate, commit evidence, and keep the change unarchived.
