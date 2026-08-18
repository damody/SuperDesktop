# Windows 11 Notification Overflow Alignment Tasks

## 1. Geometry Contract

### 1.1 Make native bounds DPI-explicit

**?桃?嚗?* Keep logical content and physical popup bounds consistent.
**頛詨嚗?* Current placement function, monitor DPI/work area, approved design.
**?Ｗ嚗?* Pure placement contract and product window options.
**靘陷嚗?* None.
**Owner嚗ave嚗?* Primary agent / wave 1.
**Gate嚗vidence嚗?* `G-OVERFLOW-GEOMETRY`; automated geometry tests.
**摰??瑼鳴?** Preferred and constrained bounds pass at 96–480 DPI and negative origins.

- [ ] 1.1.1 Extract bounded logical overflow dimensions and row calculation.
- [ ] 1.1.2 Convert width, height, origin and eight-pixel gap to physical coordinates.
- [ ] 1.1.3 Add DPI, negative-origin and constrained-work-area tests.

## 2. Windows 11 Surface

### 2.1 Align panel and icon-grid tokens

**?桃?嚗?* Match Windows 11 overflow density and panel treatment.
**頛詨嚗?* Geometry contract and existing owned overflow view.
**?Ｗ嚗?* Panel, grid and icon visual tokens.
**靘陷嚗?* 1.1.
**Owner嚗ave嚗?* Primary agent / wave 2.
**Gate嚗vidence嚗?* `G-OVERFLOW-VISUAL`; source tests and screenshot.
**摰??瑼鳴?** Six-column grid, 48px cells, 24px icons, rounded border and shadow render without clipping.

- [ ] 2.1.1 Apply preferred width, padding, radius, border, background and shadow.
- [ ] 2.1.2 Preserve bounded six-column wrapping and icon sizing.
- [ ] 2.1.3 Add light and high-contrast token tests.

### 2.2 Align interaction and accessibility states

**?桃?嚗?* Make every Windows-like state visible and accessible.
**頛詨嚗?* Existing typed notification actions and focus contract.
**?Ｗ嚗?* Hover, focus, pressed, pointer, keyboard and UIA behavior.
**靘陷嚗?* 2.1.
**Owner嚗ave嚗?* Primary agent / wave 2.
**Gate嚗vidence嚗?* `G-OVERFLOW-A11Y`; automated/UIA evidence.
**摰??瑼鳴?** Equivalent routes emit once and Escape/focus-loss dismiss correctly.

- [ ] 2.2.1 Add hover, focus and pressed state geometry/colors.
- [ ] 2.2.2 Preserve activate/context/keyboard/UIA typed routes.
- [ ] 2.2.3 Verify Escape, focus-loss and focus-return behavior.

## 3. Product Verification

### 3.1 Re-run Explorer-free headful parity

**?桃?嚗?* Prove the corrected popup in the real owned Shell path.
**頛詨嚗?* Release app, ordinary NotifyIcon fixture and capture harness.
**?Ｗ嚗?* Screenshot, UIA, callback and process evidence.
**靘陷嚗?* 2.2.
**Owner嚗ave嚗?* Primary agent / wave 3.
**Gate嚗vidence嚗?* `G-OVERFLOW-GEOMETRY`, `G-OVERFLOW-VISUAL`, `G-OVERFLOW-A11Y`, `G-SHELL-NONINTERFERENCE`; `evidence/headful.json`.
**摰??瑼鳴?** Popup floats above taskbar at 175%, callbacks pass and Explorer is absent.

- [ ] 3.1.1 Run fmt, locked/offline workspace tests and clippy warnings-as-errors.
- [ ] 3.1.2 Capture corrected 175% overflow with ordinary-client UIA/context callbacks.
- [ ] 3.1.3 Verify Explorer absence and host restart non-regression.

### 3.2 Validate and package

**?桃?嚗?* Publish traceable binaries without installer launch.
**頛詨嚗?* Passing source and headful results.
**?Ｗ嚗?* Evidence index, strict validation and installer hashes.
**靘陷嚗?* 3.1.
**Owner嚗ave嚗?* Primary agent / wave 4.
**Gate嚗vidence嚗?* `G-TRACE`, `G-PACKAGE`; evidence/package records.
**摰??瑼鳴?** Every leaf has unique evidence; standalone and combined installers pass.

- [ ] 3.2.1 Build release binaries and record hashes.
- [ ] 3.2.2 Build standalone and combined installers without launch.
- [ ] 3.2.3 Create unique evidence index and pass detailed/strict validation.
