# Windows 11 Notification Overflow Alignment Tasks

## 1. Geometry Contract

### 1.1 Make native bounds DPI-explicit

**目的：** Keep logical content and physical popup bounds consistent.
**輸入：** Current placement function, monitor DPI/work area, approved design.
**產出：** Pure placement contract and product window options.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-OVERFLOW-GEOMETRY`; automated geometry tests.
**完成門檻：** Preferred and constrained bounds pass at 96–480 DPI and negative origins.

- [x] 1.1.1 Extract bounded logical overflow dimensions, row calculation and current taskbar height.
- [x] 1.1.2 Keep `WindowOptions` logical so GPUI scales once, then reserve taskbar height and eight-pixel gap.
- [x] 1.1.3 Add DPI, negative-origin, row-height and constrained-work-area tests.

## 2. Windows 11 Surface

### 2.1 Align panel and icon-grid tokens

**目的：** Match Windows 11 overflow density and panel treatment.
**輸入：** Geometry contract and existing owned overflow view.
**產出：** Panel, grid and icon visual tokens.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-OVERFLOW-VISUAL`; source tests and screenshot.
**完成門檻：** Six-column grid, 48px cells, 24px icons, rounded border and shadow render without clipping.

- [x] 2.1.1 Apply preferred width, padding, radius, border, background and shadow.
- [x] 2.1.2 Preserve bounded six-column wrapping and icon sizing.
- [x] 2.1.3 Add light and high-contrast token tests.

### 2.2 Align interaction and accessibility states

**目的：** Make every Windows-like state visible and accessible.
**輸入：** Existing typed notification actions and focus contract.
**產出：** Hover, focus, pressed, pointer, keyboard and UIA behavior.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-OVERFLOW-A11Y`; automated/UIA evidence.
**完成門檻：** Equivalent routes emit once and Escape/focus-loss dismiss correctly.

- [x] 2.2.1 Add hover, focus and pressed state geometry/colors.
- [x] 2.2.2 Preserve activate/context/keyboard/UIA typed routes.
- [x] 2.2.3 Verify Escape, focus-loss and focus-return behavior.

## 3. Product Verification

### 3.1 Re-run Explorer-free headful parity

**目的：** Prove the corrected popup in the real owned Shell path.
**輸入：** Release app, ordinary NotifyIcon fixture and capture harness.
**產出：** Screenshot, UIA, callback and process evidence.
**依賴：** 2.2.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-OVERFLOW-GEOMETRY`, `G-OVERFLOW-VISUAL`, `G-OVERFLOW-A11Y`, `G-SHELL-NONINTERFERENCE`; `evidence/headful.json`.
**完成門檻：** Popup floats above taskbar at 175%, callbacks pass and Explorer is absent.

- [x] 3.1.1 Run fmt, locked/offline workspace tests and clippy warnings-as-errors.
- [x] 3.1.2 Capture corrected 175% overflow with ordinary-client UIA/context callbacks.
- [x] 3.1.3 Verify Explorer absence and host restart non-regression.

### 3.2 Validate and package

**目的：** Publish traceable binaries without installer launch.
**輸入：** Passing source and headful results.
**產出：** Evidence index, strict validation and installer hashes.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence/package records.
**完成門檻：** Every leaf has unique evidence; standalone and combined installers pass.

- [x] 3.2.1 Build release binaries and record hashes.
- [ ] 3.2.2 Build standalone and combined installers without launch.
- [ ] 3.2.3 Create unique evidence index and pass detailed/strict validation.
