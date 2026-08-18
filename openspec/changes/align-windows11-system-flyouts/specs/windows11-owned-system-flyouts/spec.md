## ADDED Requirements

### Requirement: Owned system flyouts use consistent Windows 11 chrome
SuperDesktop SHALL render input, volume, network/power, and calendar flyouts with one explicit light, dark, or high-contrast token family covering panel, cards, typography, border, accent, hover, pressed, focus, selection, unavailable state, and shadow geometry.

#### Scenario: Light and dark theme
- **WHEN** either light or dark presentation is selected
- **THEN** every flyout uses the corresponding shared token set without an unstyled actionable control

#### Scenario: High contrast selection
- **WHEN** high contrast is selected and a row is focused or active
- **THEN** the row exposes non-color focus or selection geometry in addition to color

### Requirement: Flyout geometry remains visible above the owned taskbar
SuperDesktop SHALL calculate system-flyout bounds from flyout kind, content count, per-monitor work area, DPI, and owned taskbar row count, and SHALL clamp the popup on positive- or negative-origin monitors.

#### Scenario: Reference host at 175 percent DPI
- **WHEN** each flyout opens on the Windows 11 reference host with a two-row owned taskbar
- **THEN** it is right-aligned with the designed gap, remains above the taskbar, and does not exceed the monitor work area

#### Scenario: Constrained monitor
- **WHEN** the logical work area is smaller than the preferred popup
- **THEN** width, height, and origin are clamped without producing negative size or off-monitor placement

### Requirement: Input and volume actions remain typed and accessible
The input flyout SHALL expose observed profiles and emit only typed profile activation, and the volume flyout SHALL expose observed mute/volume state and emit only typed volume or mute actions. Every action SHALL have a stable role, accessible name/value, pointer route, and keyboard route.

#### Scenario: Activate another input profile
- **WHEN** the user invokes a non-active profile by pointer or Enter/Space
- **THEN** exactly one `ActivateInputProfile` action is emitted and success remains dependent on the confirming provider snapshot

#### Scenario: Adjust volume with keyboard
- **WHEN** the focused volume slider receives Arrow, Home, or End
- **THEN** it emits one bounded `SetVolume` action and preserves the observed value until confirmation

#### Scenario: Provider unavailable
- **WHEN** input or audio is unavailable
- **THEN** the affected flyout shows a localized accessible unavailable status and exposes no mutation control

### Requirement: Network, power, and calendar remain truthful
The network/power flyout SHALL present only observed provider state, and the calendar SHALL present the observed date, time, locale metadata, localized weekday labels, and a valid 7×6 month grid. The UI MUST NOT render unsupported mutation as an actionable control.

#### Scenario: No system battery
- **WHEN** power reports not-present on a desktop computer
- **THEN** the flyout distinguishes AC power with no battery from provider failure

#### Scenario: Calendar at a leap-day boundary
- **WHEN** the observed date is 2028/02/29
- **THEN** the localized month grid contains exactly 29 February days and selects day 29

#### Scenario: Unsupported Quick Settings capability
- **WHEN** the provider contract has no Wi-Fi, Bluetooth, brightness, or Settings mutation
- **THEN** the flyout renders no fake toggle, inert chevron, or delegated link for that capability

### Requirement: System flyouts are localized without losing identity
Visible flyout labels SHALL support Traditional Chinese and English with bounded fallback, while accessible profile and provider identity SHALL retain the complete observed values.

#### Scenario: Traditional Chinese presentation
- **WHEN** the active locale is `zh-TW`
- **THEN** headings, status text, weekdays, and action labels use Traditional Chinese and profile UIA names retain their full observed names

#### Scenario: Keyboard layout reference structure
- **WHEN** the input flyout renders installed Traditional or Simplified Chinese profiles
- **THEN** it shows the `Win + Space` hint, two-line localized rows, profile glyphs, and a left accent bar on the observed active row without inventing additional profiles

#### Scenario: Unknown locale
- **WHEN** the presentation locale is unsupported
- **THEN** the flyout uses bounded English fallback without panicking or changing provider data

### Requirement: System flyouts remain Explorer-free and recoverable
Normal product composition MUST render and operate every system flyout without invoking Explorer, `Shell_TrayWnd`, system Quick Settings, system notification center, system Start hosts, or Settings URI handlers. One flyout SHALL remain open at most, and Escape, repeated invocation, kind switching, activation loss, or provider loss SHALL terminate or update it deterministically.

#### Scenario: Switch flyout kinds
- **WHEN** a second system control is invoked while another flyout is open
- **THEN** the old owned popup is removed before the new owned popup opens and no stale handle remains

#### Scenario: Explorer absent
- **WHEN** all four flyouts and their available actions are exercised while Explorer is absent
- **THEN** the owned callbacks and provider commands complete without a system-owned visible shell surface

#### Scenario: Popup or provider failure
- **WHEN** popup creation fails or the status host becomes unavailable
- **THEN** taskbar and desktop stay alive, affected state becomes truthfully unavailable, and ordinary failure does not launch Explorer

### Requirement: Flyout completion is auditable and packaged
The change SHALL provide automated, headful, accessibility, process, traceability, release, and installer evidence with a unique link for every atomic task.

#### Scenario: Evidence is incomplete
- **WHEN** a required theme capture, action route, process observation, unique task link, release hash, or installer hash is missing
- **THEN** the corresponding blocking gate remains failed and the change is not complete
