## 1. Freeze pointer contracts and failing tests

### 1.1 Typed gesture and popup contract

**目的：** Freeze exact child primary/context routing and popup ownership before production changes.
**輸入：** Approved design and `taskbar-pointer-interaction-parity` spec.
**產出：** Failing Rust tests in taskbar UI/composition modules.
**依賴：** None.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** G-PTR-UNIT; `evidence/unit-pointer-contract.log`.
**完成門檻：** Tests independently reject child-event fallthrough, duplicate action, wrong task reduction, and stale popup dismissal.

- [x] 1.1.1 Add pure unit cases for left/right/keyboard/UIA gesture classification.
- [x] 1.1.2 Add unit/source-contract cases proving handled task, notification, input, and volume gestures stop background propagation.
- [x] 1.1.3 Add popup-domain unit cases for toggle, conflict replacement, deactivation, Escape, and stale dismissal.

### 1.2 Headful case definitions

**目的：** Make the requested physical-pointer behaviors mandatory and independently failing in UTIT.
**輸入：** Existing UTIT catalog and headful scripts.
**產出：** Catalog tests plus initial failing fixture assertions/report schemas.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** G-UTIT-CATALOG; `evidence/utit-catalog.log`.
**完成門檻：** Catalog exposes mandatory system-control, notification-icon, and task-application pointer cases with watchdog declarations.

- [x] 1.2.1 Add UTIT catalog entries and catalog assertions for all three pointer case families.
- [x] 1.2.2 Strengthen system-status fixture schema to require input/volume left/right intent and unintended-popup absence.
- [x] 1.2.3 Strengthen NotifyIcon fixture schema to require ordered exact Activate and Context callbacks for visible and overflow icons.
- [x] 1.2.4 Extend taskbar window-actions fixture schema to require left-click state transitions and exclusive right-click Jump List behavior.

## 2. Implement exact taskbar interaction routing

### 2.1 View input isolation

**目的：** Route each handled gesture exactly once at the child control.
**輸入：** Failing contracts from 1.1 and existing `TaskbarCallbacks`.
**產出：** Typed taskbar UI callbacks/helpers and passing view tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** G-PTR-UNIT; `evidence/unit-pointer-contract.log`.
**完成門檻：** All affected child controls distinguish primary/context gestures and cannot open the background menu through propagation.

- [x] 2.1.1 Implement typed gesture classification and child propagation guards in `taskbar-ui`.
- [x] 2.1.2 Route visible and overflow notification buttons to exclusive Activate/Context callbacks.
- [x] 2.1.3 Route task application buttons to exclusive primary/Jump List callbacks.
- [x] 2.1.4 Route input and volume controls to distinct primary/context callbacks with keyboard/UIA primary parity.

### 2.2 Owned context surfaces and fixed actions

**目的：** Provide Explorer-aligned input and volume right-click menus without widening launch authority.
**輸入：** Typed context callbacks from 2.1 and existing popup styles/status commands.
**產出：** Owned context view, compile-time fixed platform actions, composition callbacks, and traces.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** G-CONTEXT-SAFETY; `evidence/context-safety.log`.
**完成門檻：** Input exposes only Language preferences; volume exposes only fixed mixer/sound actions; rejection is truthful and no caller-controlled target exists.

- [x] 2.2.1 Implement the accessible owned system-control context view with Escape/focus-loss dismissal.
- [x] 2.2.2 Implement compile-time Open volume mixer and Sound settings platform launch functions and tests.
- [x] 2.2.3 Compose input/volume context actions, stable traces, and fixed-target rejection handling.

### 2.3 Popup ownership integration

**目的：** Ensure only one intended taskbar popup survives each gesture.
**輸入：** Existing popup slots and new system-control context slot.
**產出：** Central dismissal/exclusivity logic with stale-slot protection.
**依賴：** 2.1 and 2.2.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** G-POPUP-OWNERSHIP; `evidence/popup-ownership.log`.
**完成門檻：** System flyouts, context menus, overflow, previews, Jump Lists, and background context obey toggle/conflict/deactivation/Escape/stale-dismiss requirements.

- [x] 2.3.1 Centralize conflicting-popup dismissal before every owned taskbar popup open.
- [x] 2.3.2 Add generation/identity checks so stale dismiss callbacks cannot clear replacement ownership.
- [x] 2.3.3 Run focused popup and task state tests and record passing output.

## 3. Prove real-pointer parity

### 3.1 System controls and notification icons

**目的：** Prove input, volume, visible icons, and overflow icons with physical left/right input.
**輸入：** Production implementation and fixtures from 1.2.
**產出：** Versioned reports, traces, screenshots, and callback logs.
**依賴：** 2.2 and 2.3.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** G-HEADFUL-SYSTEM, G-HEADFUL-NOTIFY; `evidence/headful-system/`, `evidence/headful-notify/`.
**完成門檻：** Exact intended actions pass, unintended background actions are absent, and Explorer recovery is observed.

- [x] 3.1.1 Implement bounded real-pointer input/volume left/right checks and privacy-safe report fields.
- [x] 3.1.2 Implement bounded visible/overflow NotifyIcon left/right checks with ordered exact native callback assertions.
- [x] 3.1.3 Run focused UTIT system-control and notification-icon cases and retain hashed artifacts.

### 3.2 Task application buttons

**目的：** Prove Explorer-like application left-click state reduction and exclusive right-click Jump List behavior.
**輸入：** Window fixture, observed task state, and integrated popup ownership.
**產出：** Versioned application-pointer report and traces.
**依賴：** 2.3.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** G-HEADFUL-TASK; `evidence/headful-task/`.
**完成門檻：** Activate, minimize, restore, group preview, and Jump List subchecks pass without unintended primary/background behavior.

- [x] 3.2.1 Add real-pointer inactive/active/minimized single-window state checks.
- [x] 3.2.2 Add multi-window preview and exclusive right-click Jump List checks.
- [x] 3.2.3 Run the focused UTIT task-application case and retain hashed artifacts.

## 4. Integration and final gates

### 4.1 Repository quality and traceability

**目的：** Close all blocking requirements with reproducible quality and OpenSpec evidence.
**輸入：** Completed implementation and headful artifacts.
**產出：** Formatted source, passing targeted/full tests, strict spec validation, and final evidence index.
**依賴：** 3.1 and 3.2.
**Owner／Wave：** Primary agent / Wave 4.
**Gate／Evidence：** G-FMT, G-CHECK, G-TEST, G-CLIPPY, G-OPENSPEC; `evidence/quality/` and `evidence/evidence-index.json`.
**完成門檻：** All gates pass, every leaf has a unique evidence record/subcheck, no placeholder or unresolved P0/P1 remains, and unrelated working-tree changes are preserved.

- [x] 4.1.1 Run Rust formatting check and save the result.
- [x] 4.1.2 Run focused crate tests and locked offline workspace check/test.
- [x] 4.1.3 Run locked offline workspace Clippy with warnings denied.
- [x] 4.1.4 Run strict OpenSpec validation and scan artifacts for placeholders/contradictions.
- [x] 4.1.5 Write the evidence index with commands, actual results, exit states, hashes, timestamps, and task/gate mappings.
- [x] 4.1.6 Review the final diff for pointer contract completeness, fixed-target safety, report integrity, and unrelated-change preservation.
