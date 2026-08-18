# Windows 11 Owned Taskbar Settings Layout Tasks

## 1. Logical geometry and visual contracts

### 1.1 Correct settings-window DPI geometry

**目的：** Remove double scaling and keep the owned window inside its monitor.
**輸入：** Monitor bounds/work area, DPI, current WindowOptions helpers.
**產出：** Logical placement/size helpers and geometry tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-SETTINGS-DPI`; automated geometry evidence.
**完成門檻：** 100–225% and negative/small monitor cases pass with one conversion.

- [x] 1.1.1 Return logical origin and dimensions from settings placement.
- [x] 1.1.2 Pass logical bounds directly into GPUI WindowOptions.
- [x] 1.1.3 Add DPI matrix, negative-origin, compact, and source-contract tests.

### 1.2 Define responsive layout and Windows tokens

**目的：** Freeze testable page, card, row, switch, and theme metrics.
**輸入：** Approved design and current theme logic.
**產出：** Pure layout/token structs and deterministic tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-SETTINGS-LAYOUT`, `G-SETTINGS-THEME`; unit evidence.
**完成門檻：** Wide/compact geometry and light/dark/high-contrast metrics are exact.

- [x] 1.2.1 Add responsive padding/content-width metrics.
- [x] 1.2.2 Centralize card, typography, focus, border, and switch tokens.
- [x] 1.2.3 Add wide, compact, theme, and high-contrast distinction tests.

## 2. Owned responsive page implementation

### 2.1 Replace fixed canvas with full-window scroll layout

**目的：** Eliminate right-side dead canvas and make the entire page reachable.
**輸入：** Layout metrics, current render tree and section model.
**產出：** Full-window root, centered bounded content, one vertical scroll owner.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SETTINGS-LAYOUT`; source/headful evidence.
**完成門檻：** Top and bottom cards fit horizontally and all controls scroll into view.

- [x] 2.1.1 Change the root to fill actual window width and height.
- [x] 2.1.2 Center a max-width responsive content column with compact padding.
- [x] 2.1.3 Preserve one vertical scroll owner and bottom clearance.

### 2.2 Align cards, rows, switches, focus, and accessibility

**目的：** Match Windows 11 proportions without changing model behavior.
**輸入：** Visual tokens, localized rows, existing typed actions.
**產出：** Restyled headers/rows/toggles/focus and retained UIA semantics.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SETTINGS-VISUAL`, `G-A11Y-I18N`; UIA/headful evidence.
**完成門檻：** Geometry matches tokens, focus is visible, and authoritative actions remain equivalent.

- [x] 2.2.1 Apply card/header/row/text/switch Windows metrics.
- [x] 2.2.2 Add visible keyboard focus and high-contrast non-color cues.
- [x] 2.2.3 Re-run localization, UIA toggle, save-failure, and non-delegation tests.

## 3. Headful verification and packaging

### 3.1 Prove responsive reachability on the reference host

**目的：** Verify the actual Windows 11 page at 175% DPI.
**輸入：** Release settings example/app and UIA capture harness.
**產出：** Top/middle/bottom screenshots, bounds, scroll, UIA and theme reports.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-SETTINGS-DPI`, `G-SETTINGS-LAYOUT`, `G-SETTINGS-VISUAL`, `G-A11Y-I18N`; headful evidence.
**完成門檻：** No dead canvas/clipping; auto-hide and related rows are visible and operable.

- [x] 3.1.1 Run fmt, locked/offline workspace tests, clippy, and release build.
- [x] 3.1.2 Capture top, scrolled bottom, light/dark, and 175% geometry evidence.
- [x] 3.1.3 Capture keyboard/UIA reachability, save error, and zero-delegation evidence.

### 3.2 Admit the revision and installers

**目的：** Complete traceability and distributable packaging without archive.
**輸入：** Passing automated/headful gates and committed implementation.
**產出：** Binary/installers, hashes, 18-leaf evidence index, strict validation.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; packaging/evidence records.
**完成門檻：** 18/18 unique evidence, both installers, detailed/strict pass, unarchived.

- [ ] 3.2.1 Commit implementation and record release binary/revision hashes.
- [ ] 3.2.2 Build standalone and combined NSIS installers without launch.
- [ ] 3.2.3 Create the unique evidence index and pass detailed/strict validation.
