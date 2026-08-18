# Windows 11 Owned Start Visual Alignment Tasks

## 1. Presentation Contracts

### 1.1 Add bounded Start localization

**目的：** Localize presentation without changing stable model identity.
**輸入：** Approved design, Windows locale adapter and existing Start labels.
**產出：** `StartStrings` selection and fallback tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-START-LOCALE`; automated locale record.
**完成門檻：** `zh-TW`, English and unsupported fallback tables cover every visible/UIA label.

- [ ] 1.1.1 Define complete Traditional Chinese and English Start string tables.
- [ ] 1.1.2 Select deterministic override then Windows user locale with English fallback.
- [ ] 1.1.3 Add completeness, fallback and long-label tests.

### 1.2 Centralize Windows 11 visual tokens

**目的：** Keep Start colors, radius, borders and interaction states consistent.
**輸入：** Existing Start render code and Windows 11 host reference.
**產出：** Light and high-contrast `StartVisualTokens`.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-START-VISUAL`, `G-START-A11Y`; token tests.
**完成門檻：** Panel, search, cells, footer, flyout, hover, pressed and focus tokens are explicit.

- [ ] 1.2.1 Define light panel, search, cell, footer and flyout tokens.
- [ ] 1.2.2 Define high-contrast borders, fills and focus geometry.
- [ ] 1.2.3 Add source/token tests preventing flat unstyled controls.

## 2. Owned Start Surface

### 2.1 Align Home and All apps hierarchy

**目的：** Match Windows 11 search, headings and bounded content density.
**輸入：** Start strings/tokens and existing model snapshots.
**產出：** Restyled search, Pinned, Recommended and All apps views.
**依賴：** 1.1, 1.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-START-VISUAL`, `G-START-LOCALE`; Home/All apps screenshots.
**完成門檻：** 640×720 Home and All apps fit at 175% with six/two-column density and no clipping.

- [ ] 2.1.1 Restyle localized search field and section/navigation headings.
- [ ] 2.1.2 Restyle Pinned and Recommended cells with Windows interaction states.
- [ ] 2.1.3 Restyle localized All apps list and Back navigation.

### 2.2 Align footer and Power flyout

**目的：** Replace oversized footer buttons with compact Windows-like controls.
**輸入：** Current Account, Settings and Power actions.
**產出：** Compact localized footer and owned Power flyout.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-START-VISUAL`, `G-START-A11Y`; footer/flyout capture.
**完成門檻：** Account, Settings, Power and three confirmed actions remain reachable by pointer, keyboard and UIA.

- [ ] 2.2.1 Render account identity and compact Settings/Power icon-led controls.
- [ ] 2.2.2 Restyle and localize Sign out, Restart and Shut down flyout actions.
- [ ] 2.2.3 Preserve collapsed-by-default and explicit confirmation behavior.

### 2.3 Preserve owned routing and failure truthfulness

**目的：** Prevent visual work from reintroducing system Start delegation.
**輸入：** Current source guards and typed Start actions.
**產出：** Expanded guards and route equivalence tests.
**依賴：** 2.1, 2.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-SHELL-NONINTERFERENCE`, `G-START-A11Y`; automated record.
**完成門檻：** All routes remain typed and unavailable states stay inside owned Start.

- [ ] 2.3.1 Preserve pointer, keyboard, UIA, IME and repeated-toggle routes.
- [ ] 2.3.2 Localize truthful unavailable/empty states without provider substitution.
- [ ] 2.3.3 Expand forbidden system Start/Settings presentation source guards.

## 3. Verification and Packaging

### 3.1 Capture and verify current-host parity

**目的：** Prove localized aligned Start on the real 175% reference host.
**輸入：** Release binaries and owned Start capture harness.
**產出：** Home, All apps, Power, UIA and process evidence.
**依賴：** 2.3.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-START-LOCALE`, `G-START-VISUAL`, `G-START-A11Y`, `G-SHELL-NONINTERFERENCE`; `evidence/headful.json`.
**完成門檻：** Traditional Chinese captures fit, expose correct UIA and create no system Start process transition.

- [ ] 3.1.1 Run fmt, locked/offline workspace tests and clippy warnings-as-errors.
- [ ] 3.1.2 Capture Traditional Chinese Home, All apps and Power at 175% DPI.
- [ ] 3.1.3 Verify UIA bounds, typed traces and unchanged system Start process IDs.

### 3.2 Validate traceability and installers

**目的：** Publish reproducible evidence and packages without archive or launch.
**輸入：** Passing source/headful gates and admitted submodule revision.
**產出：** Release hashes, evidence index and two NSIS installers.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence/package records.
**完成門檻：** Every leaf has unique evidence, strict validation passes and both installers build without launch.

- [ ] 3.2.1 Build release binaries and record hashes.
- [ ] 3.2.2 Build standalone and combined installers without launching them.
- [ ] 3.2.3 Create unique evidence index and pass detailed/strict validation.
