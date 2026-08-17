## 1. Desktop Pointer Selection

### 1.1 Marquee geometry and state

**Outcome:** Deterministic transient rectangle state and item intersection.
**Inputs:** Desktop item positions, 104x112 hitboxes, pointer events, modifier baseline.
**Outputs:** Marquee state/helpers and geometry tests.
**Dependencies:** None.
**Owner/Wave:** Primary agent / wave 1.
**Gate/Evidence:** `G-DESKTOP-START-PARITY`; `evidence/evidence-index.json`.
**Completion threshold:** Normal/reverse/threshold/Ctrl geometry tests pass without cumulative drift.

- [x] 1.1.1 Add normalized marquee bounds, threshold, and inclusive hit-test helpers.
- [x] 1.1.2 Add transient anchor/current/baseline/modifier state to DesktopView.
- [x] 1.1.3 Add normal, reverse, threshold, and Ctrl-additive geometry tests.

### 1.2 Pointer routing and visual feedback

**Outcome:** Empty-space drag selects live items without breaking item interactions.
**Inputs:** 1.1 helpers, GPUI mouse events, current desktop rendering.
**Outputs:** Root/item listeners, marquee element, interaction tests.
**Dependencies:** 1.1.
**Owner/Wave:** Primary agent / wave 1.
**Gate/Evidence:** `G-DESKTOP-START-PARITY`; marquee screenshot and tests.
**Completion threshold:** Gesture starts only on empty space, updates live, cancels safely, and leaves final selection.

- [x] 1.2.1 Wire empty-space left down, move, up, and lost-button cancellation.
- [x] 1.2.2 Stop item primary-down propagation while preserving click/drag/drop behavior.
- [x] 1.2.3 Paint the DPI-correct translucent blue rectangle at the proper z-order.
- [x] 1.2.4 Add selection completion, item propagation, and stale-capture regression tests.

## 2. Windows 11 Start Model

### 2.1 Home, All apps, search, and power state

**Outcome:** Bounded model APIs for every visible Start mode.
**Inputs:** Existing StartModel catalogs/search/persistence and Windows 11 section contract.
**Outputs:** Page state, bounded slices, power state, model tests.
**Dependencies:** None.
**Owner/Wave:** Primary agent / wave 2.
**Gate/Evidence:** `G-DESKTOP-START-PARITY`; model test output.
**Completion threshold:** Home/all/search transitions, bounds, sorting, persistence, and power dismissal are deterministic.

- [x] 2.1.1 Add StartPage and power-flyout state with explicit transitions.
- [x] 2.1.2 Expose deduplicated 12-pin, six-recommendation, alphabetical-app, and search slices.
- [x] 2.1.3 Preserve page/pins across search commits, clear, stale batches, and activation.
- [x] 2.1.4 Add model tests for page transitions, bounds, sorting, focus, persistence, and power Escape.

## 3. Windows 11 Start View

### 3.1 Icon-bearing section rendering

**Outcome:** Windows 11 home, All apps, and search layouts with native icons.
**Inputs:** 2.1 model APIs, shared Shell/BC7 renderer, existing activation callbacks.
**Outputs:** StartView layout and rendering tests.
**Dependencies:** 2.1.
**Owner/Wave:** Primary agent / wave 3.
**Gate/Evidence:** `G-DESKTOP-START-PARITY`; Start screenshots and UIA report.
**Completion threshold:** All modes render bounded sections, correct roles/names, icons/fallbacks, and actionable controls.

- [x] 3.1.1 Reuse the taskbar Shell/BC7 icon renderer for Start application paths.
- [x] 3.1.2 Render Search and the Pinned six-column plus Recommended two-column home layout.
- [x] 3.1.3 Render All apps/back and ranked search-result list modes with subtitles.
- [x] 3.1.4 Render account, Settings, one Power button, and keyboard-addressable power flyout.
- [x] 3.1.5 Add source/model view tests for sections, roles, labels, icon fallback, and safe power structure.

### 3.2 Placement and owned verification route

**Outcome:** Start opens centered/clamped and can be verified without shell takeover.
**Inputs:** Monitor work area, current start composition, bounded verification environment.
**Outputs:** Placement helper/tests and verification-only owned Start route.
**Dependencies:** 3.1.
**Owner/Wave:** Primary agent / wave 3.
**Gate/Evidence:** `G-DESKTOP-START-PARITY`; placement tests and screenshots.
**Completion threshold:** Normal and small-monitor placement stays inside work area with the required gap; production authority is unchanged.

- [x] 3.2.1 Extract and test centered/clamped Start geometry with a 12-logical-pixel bottom gap.
- [x] 3.2.2 Add a bounded verification fixture that opens owned Start without shell mutation.
- [x] 3.2.3 Preserve Escape, arrows, Enter, IME, UIA focus, and exactly-once dismissal.

## 4. Verification and Packaging

### 4.1 Automated and headful gate

**Outcome:** Auditable proof of desktop and Start behavior on the active Windows host.
**Inputs:** Integrated desktop/Start implementation.
**Outputs:** Test logs, marquee/Start screenshots, UIA JSON, evidence index, validation result.
**Dependencies:** 1.2 and 3.2.
**Owner/Wave:** Primary agent / wave 4.
**Gate/Evidence:** `G-DESKTOP-START-PARITY`; `evidence/evidence-index.json`.
**Completion threshold:** Every automated command exits zero and headful evidence passes at host DPI.

- [x] 4.1.1 Run formatting, complete locked/offline workspace checks/tests, and clippy with warnings denied.
- [x] 4.1.2 Capture and inspect an active marquee at 175% DPI with selected UIA items.
- [x] 4.1.3 Capture and inspect Start home and All apps with icons, sections, footer, and hit targets.
- [ ] 4.1.4 Record unique task-linked evidence and pass strict OpenSpec validation.

### 4.2 Installer integration

**Outcome:** Standalone and combined installers contain the verified implementation.
**Inputs:** Gate-passing source and release binaries.
**Outputs:** Hashed NSIS installers.
**Dependencies:** 4.1.
**Owner/Wave:** Primary agent / wave 5.
**Gate/Evidence:** Installer hashes in evidence index.
**Completion threshold:** Both installers build without launch and validate as PE artifacts.

- [ ] 4.2.1 Build and hash the standalone SuperDesktop installer without launching it.
- [ ] 4.2.2 Build and hash the combined SuperExplorer installer with formal submodule admission.
