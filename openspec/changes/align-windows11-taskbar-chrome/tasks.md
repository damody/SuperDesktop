# Windows 11 Taskbar Chrome Alignment Tasks

## 1. Presentation Contracts

### 1.1 Centralize taskbar chrome tokens

**目的：** Replace scattered colors and state styles with one bounded palette.
**輸入：** Approved design and current `TaskbarView` styles.
**產出：** Light/high-contrast `TaskbarChromeTokens` and tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-CHROME`, `G-TASKBAR-A11Y`; automated record.
**完成門檻：** Panel, border, text, hover, pressed, focus and attention tokens are explicit.

- [x] 1.1.1 Define complete light taskbar chrome tokens.
- [x] 1.1.2 Define high-contrast tokens with non-color focus geometry.
- [x] 1.1.3 Add token/source tests preventing unstyled taskbar controls.

### 1.2 Add localized compact presentation helpers

**目的：** Present Search and IME like Windows without mutating provider values.
**輸入：** Windows locale, input-profile provider values and existing UIA names.
**產出：** Search labels, compact IME helper and fallback tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-LOCALE`, `G-TASKBAR-A11Y`; locale tests.
**完成門檻：** `zh-TW`, `zh-CN`, English and unknown locales produce bounded visible text and full UIA identity.

- [x] 1.2.1 Add Traditional Chinese/English taskbar Search presentation.
- [x] 1.2.2 Map known input locales to `中` or `ENG` with bounded fallback.
- [x] 1.2.3 Add locale fallback and accessible-name tests.

## 2. Taskbar Surface

### 2.1 Align primary controls and rows

**目的：** Match Windows row background, border and core control density.
**輸入：** Chrome tokens and current row layout.
**產出：** Restyled Start, Search, Task View and row surface.
**依賴：** 1.1, 1.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-CHROME`; light/high-contrast capture.
**完成門檻：** Every row stays 40px and core controls expose consistent state styling.

- [x] 2.1.1 Apply panel, top border and row-separation tokens.
- [x] 2.1.2 Align Start, Search and Task View dimensions and state styles.
- [x] 2.1.3 Preserve pointer, keyboard and UIA typed routes.

### 2.2 Align fixed and running task buttons

**目的：** Reduce oversized labels while preserving requested indicators and overlays.
**輸入：** Existing fixed/task render paths and visual-state model.
**產出：** 160px labeled cells, compact icon cells and aligned indicators.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-CHROME`, `G-TASKBAR-A11Y`; state-matrix evidence.
**完成門檻：** Labels ellipsize accessibly; progress, attention, grouping and long/short indicators remain correct.

- [x] 2.2.1 Align fixed entry padding, icon, label and indicator.
- [x] 2.2.2 Align running labeled/icon-only button widths and state styles.
- [x] 2.2.3 Reverify progress, attention, grouped and unavailable overlays.

### 2.3 Align notification and system region

**目的：** Match Windows spacing and typography on the taskbar right edge.
**輸入：** Notification nodes and system-status snapshot.
**產出：** Compact tray, network, volume, IME and clock controls.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-CHROME`, `G-TASKBAR-LOCALE`, `G-TASKBAR-A11Y`; system-region evidence.
**完成門檻：** Region fits 210px, clock is two-line, IME is compact and unavailable providers stay independent.

- [x] 2.3.1 Align notification/overflow cells and hover/focus states.
- [x] 2.3.2 Align network, volume and compact IME controls.
- [x] 2.3.3 Align two-line clock/date typography and accessible bounds.

### 2.4 Preserve Explorer-free ownership

**目的：** Prevent presentation changes from introducing delegated shell UI.
**輸入：** Existing source guards and typed taskbar callbacks.
**產出：** Expanded forbidden-delegation and route tests.
**依賴：** 2.2, 2.3.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-SHELL-NONINTERFERENCE`; automated source record.
**完成門檻：** All visible controls remain owned and every callback path remains typed.

- [x] 2.4.1 Preserve task, fixed, notification and system typed callbacks.
- [x] 2.4.2 Reject Explorer, Shell_TrayWnd and system Start-host presentation calls.
- [x] 2.4.3 Preserve independent truthful unavailable states.

## 3. Verification and Packaging

### 3.1 Capture reference-host taskbar parity

**目的：** Prove aligned chrome at 175% in representative states.
**輸入：** Release app and taskbar state/NotifyIcon harnesses.
**產出：** Light/high-contrast screenshots, UIA and state traces.
**依賴：** 2.4.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-TASKBAR-CHROME`, `G-TASKBAR-LOCALE`, `G-TASKBAR-A11Y`, `G-SHELL-NONINTERFERENCE`; `evidence/headful.json`.
**完成門檻：** Two-row labeled/icon-only, progress/attention, tray, IME and clock fit without Explorer delegation.

- [x] 3.1.1 Run fmt, locked/offline workspace tests and clippy warnings-as-errors.
- [x] 3.1.2 Capture light and high-contrast taskbar at host 175% DPI.
- [x] 3.1.3 Verify UIA routes, representative states and Explorer-free process evidence.

### 3.2 Validate traceability and installers

**目的：** Publish reproducible evidence and updated packages.
**輸入：** Passing source/headful gates and admitted revision.
**產出：** Release hashes, evidence index and two NSIS installers.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence/package records.
**完成門檻：** Every leaf has unique evidence; strict validation and both installer builds pass without launch.

- [x] 3.2.1 Build release binaries and record hashes.
- [ ] 3.2.2 Build standalone and combined installers without launch.
- [ ] 3.2.3 Create unique evidence index and pass detailed/strict validation.
