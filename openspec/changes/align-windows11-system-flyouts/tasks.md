# Windows 11 Owned System Flyout Tasks

## 1. Presentation Contracts

### 1.1 Centralize flyout chrome and localization

**目的：** Give every system flyout one complete Windows 11 presentation contract.
**輸入：** Approved design, current `SystemFlyoutView`, taskbar chrome conventions.
**產出：** Theme tokens, locale strings, compact identity helpers, deterministic tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-SYSTEM-FLYOUT-CHROME`, `G-SYSTEM-FLYOUT-A11Y`; automated record.
**完成門檻：** Light, dark, high contrast, zh-TW, English, and fallback values are explicit and tested.

- [x] 1.1.1 Define complete light, dark, and high-contrast flyout tokens.
- [x] 1.1.2 Define localized visible strings and bounded input-profile tags.
- [x] 1.1.3 Add token, locale, fallback, and non-color state tests.

### 1.2 Make popup geometry monitor- and taskbar-aware

**目的：** Keep each popup visible and correctly anchored at every supported DPI/topology.
**輸入：** Monitor work-area records, taskbar row count, flyout kind, input profile count.
**產出：** Pure `system_flyout_geometry` contract and boundary tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-SYSTEM-FLYOUT-CHROME`; automated geometry record.
**完成門檻：** Preferred and constrained positive/negative-origin cases stay above owned taskbar bounds.

- [x] 1.2.1 Define kind-specific preferred width and content-driven height.
- [x] 1.2.2 Clamp logical bounds using DPI, work area, taskbar rows, and popup gap.
- [x] 1.2.3 Add 175% reference, constrained, and negative-origin geometry tests.

## 2. Owned Flyout Surfaces

### 2.1 Align input-language and volume flyouts

**目的：** Match Windows interaction density while preserving typed provider mutations.
**輸入：** Presentation helpers and observed input/audio snapshots.
**產出：** Restyled input rows, volume slider/mute control, unavailable states.
**依賴：** 1.1, 1.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-SYSTEM-FLYOUT-CHROME`, `G-SYSTEM-FLYOUT-A11Y`, `G-SYSTEM-FLYOUT-TRUTH`; automated and headful records.
**完成門檻：** Pointer/keyboard/UIA routes emit only bounded typed actions and never mutate unavailable providers.

- [x] 2.1.1 Match the supplied Windows keyboard-layout header, shortcut hint, two-line profile rows, active accent bar, glyphs, and full accessible provider identity.
- [x] 2.1.2 Restyle mute, slider, thumb, percentage, and observed unavailable audio state.
- [x] 2.1.3 Add Enter/Space, Arrow/Home/End, role/value, and typed-action tests.

### 2.2 Align network/power and calendar flyouts

**目的：** Present truthful status and localized calendar content without fake actions.
**輸入：** Presentation helpers and observed network, power, clock, and date snapshots.
**產出：** Informational status cards, localized 7×6 calendar, unavailable states.
**依賴：** 1.1, 1.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-SYSTEM-FLYOUT-CHROME`, `G-SYSTEM-FLYOUT-A11Y`, `G-SYSTEM-FLYOUT-TRUTH`; automated and headful records.
**完成門檻：** Network/power distinguish real states, calendar boundaries pass, and unsupported mutation has no action.

- [x] 2.2.1 Restyle network and power summaries with not-present versus unavailable distinction.
- [x] 2.2.2 Localize date metadata, weekday labels, month heading, and selected-day geometry.
- [x] 2.2.3 Add leap-year, provider-failure, no-battery, and no-fake-action tests.

### 2.3 Wire explicit presentation and geometry into app composition

**目的：** Make production popup creation use deterministic theme, locale, DPI, and taskbar inputs.
**輸入：** Taskbar settings/theme signals, monitor record, flyout helpers.
**產出：** Updated `SystemFlyoutView::new`, popup options, lifecycle tests.
**依賴：** 2.1, 2.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-SYSTEM-FLYOUT-CHROME`, `G-SYSTEM-FLYOUT-A11Y`; app integration record.
**完成門檻：** All four kinds open with correct inputs, toggle/switch/dismiss deterministically, and preserve focus behavior.

- [x] 2.3.1 Pass explicit locale/theme/high-contrast presentation into each owned popup.
- [x] 2.3.2 Replace fixed window options with pure taskbar-aware geometry.
- [x] 2.3.3 Reverify toggle, kind switch, Escape, activation-loss, and creation-failure lifecycle.

### 2.4 Enforce Explorer-free truthful composition

**目的：** Prevent visual convergence from introducing delegated or fabricated shell behavior.
**輸入：** Production app/view sources and typed status contracts.
**產出：** Expanded forbidden-source and truthful-action tests.
**依賴：** 2.3.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-SYSTEM-FLYOUT-TRUTH`, `G-SHELL-NONINTERFERENCE`; source/process records.
**完成門檻：** Production sources contain only owned popup routes and ordinary failure never launches a shell surface.

- [x] 2.4.1 Guard against Explorer, shell hosts, Quick Settings, notification center, and Settings URI delegation.
- [x] 2.4.2 Guard network, power, calendar, and unavailable states against fake mutation controls.
- [x] 2.4.3 Preserve typed status actions and independent provider-loss recovery.

## 3. Verification and Packaging

### 3.1 Capture reference-host visual and interaction evidence

**目的：** Prove the four owned flyouts work and align at 175% DPI without Explorer.
**輸入：** Release app, flyout fixtures, UIA/process harnesses.
**產出：** Theme screenshots, UIA/action traces, Explorer-free process observation.
**依賴：** 2.4.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-SYSTEM-FLYOUT-CHROME`, `G-SYSTEM-FLYOUT-A11Y`, `G-SYSTEM-FLYOUT-TRUTH`, `G-SHELL-NONINTERFERENCE`; `evidence/headful-*.json`.
**完成門檻：** All kinds, themes, representative states, UIA routes, and Explorer absence pass on the reference host.

- [x] 3.1.1 Run fmt, locked/offline workspace tests, and clippy warnings-as-errors.
- [x] 3.1.2 Capture light and dark input, volume, network/power, and calendar matrices at 175% DPI.
- [x] 3.1.3 Capture high-contrast UIA/keyboard states and Explorer-free process/action evidence.

### 3.2 Validate traceability, release, and installers

**目的：** Produce reproducible completion evidence and updated distributable packages.
**輸入：** Passing source/headful gates and admitted implementation revision.
**產出：** Release hashes, unique evidence index, standalone and combined NSIS installers.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** `G-TRACE`, `G-PACKAGE`; evidence and package records.
**完成門檻：** Every leaf has unique evidence; detailed/strict validation and both installer builds pass without launch.

- [ ] 3.2.1 Build release binaries and record reproducible hashes.
- [ ] 3.2.2 Build standalone and combined NSIS installers without launch and record hashes.
- [ ] 3.2.3 Create the unique evidence index and pass detailed and strict validation.
