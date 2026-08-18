# Windows 11 Notification Overflow Alignment Tasks

## 1. Geometry Contract

### 1.1 Make native bounds DPI-explicit

**Purpose:** Keep logical content and physical popup bounds consistent.
**Inputs:** Current placement function, monitor DPI/work area, approved design.
**Outputs:** Pure placement contract and product window options.
**Dependencies:** None.
**Owner/Wave:** Primary agent / wave 1.
**Gate/Evidence:** `G-OVERFLOW-GEOMETRY`; automated geometry tests.
**Completion threshold:** Preferred and constrained bounds pass at 96–480 DPI and negative origins.

- [ ] 1.1.1 Extract bounded logical overflow dimensions and row calculation.
- [ ] 1.1.2 Convert width, height, origin and eight-pixel gap to physical coordinates.
- [ ] 1.1.3 Add DPI, negative-origin and constrained-work-area tests.

## 2. Windows 11 Surface

### 2.1 Align panel and icon-grid tokens

**Purpose:** Match Windows 11 overflow density and panel treatment.
**Inputs:** Geometry contract and existing owned overflow view.
**Outputs:** Panel, grid and icon visual tokens.
**Dependencies:** 1.1.
**Owner/Wave:** Primary agent / wave 2.
**Gate/Evidence:** `G-OVERFLOW-VISUAL`; source tests and screenshot.
**Completion threshold:** Six-column grid, 48px cells, 24px icons, rounded border and shadow render without clipping.

- [ ] 2.1.1 Apply preferred width, padding, radius, border, background and shadow.
- [ ] 2.1.2 Preserve bounded six-column wrapping and icon sizing.
- [ ] 2.1.3 Add light and high-contrast token tests.

### 2.2 Align interaction and accessibility states

**Purpose:** Make every Windows-like state visible and accessible.
**Inputs:** Existing typed notification actions and focus contract.
**Outputs:** Hover, focus, pressed, pointer, keyboard and UIA behavior.
**Dependencies:** 2.1.
**Owner/Wave:** Primary agent / wave 2.
**Gate/Evidence:** `G-OVERFLOW-A11Y`; automated/UIA evidence.
**Completion threshold:** Equivalent routes emit once and Escape/focus-loss dismiss correctly.

- [ ] 2.2.1 Add hover, focus and pressed state geometry/colors.
- [ ] 2.2.2 Preserve activate/context/keyboard/UIA typed routes.
- [ ] 2.2.3 Verify Escape, focus-loss and focus-return behavior.

## 3. Product Verification

### 3.1 Re-run Explorer-free headful parity

**Purpose:** Prove the corrected popup in the real owned Shell path.
**Inputs:** Release app, ordinary NotifyIcon fixture and capture harness.
**Outputs:** Screenshot, UIA, callback and process evidence.
**Dependencies:** 2.2.
**Owner/Wave:** Primary agent / wave 3.
**Gate/Evidence:** `G-OVERFLOW-GEOMETRY`, `G-OVERFLOW-VISUAL`, `G-OVERFLOW-A11Y`, `G-SHELL-NONINTERFERENCE`; `evidence/headful.json`.
**Completion threshold:** Popup floats above taskbar at 175%, callbacks pass and Explorer is absent.

- [ ] 3.1.1 Run fmt, locked/offline workspace tests and clippy warnings-as-errors.
- [ ] 3.1.2 Capture corrected 175% overflow with ordinary-client UIA/context callbacks.
- [ ] 3.1.3 Verify Explorer absence and host restart non-regression.

### 3.2 Validate and package

**Purpose:** Publish traceable binaries without installer launch.
**Inputs:** Passing source and headful results.
**Outputs:** Evidence index, strict validation and installer hashes.
**Dependencies:** 3.1.
**Owner/Wave:** Primary agent / wave 4.
**Gate/Evidence:** `G-TRACE`, `G-PACKAGE`; evidence/package records.
**Completion threshold:** Every leaf has unique evidence; standalone and combined installers pass.

- [ ] 3.2.1 Build release binaries and record hashes.
- [ ] 3.2.2 Build standalone and combined installers without launch.
- [ ] 3.2.3 Create unique evidence index and pass detailed/strict validation.
