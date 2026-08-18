## ADDED Requirements

### Requirement: Taskbar chrome is visually consistent
The owned taskbar SHALL apply one Windows 11 light or high-contrast token set to its panel, border, task controls and system controls.

#### Scenario: Pointer and keyboard states
- **WHEN** a control is hovered, pressed or keyboard-focused
- **THEN** it exposes the shared state treatment without changing its typed action

### Requirement: Task density preserves extended behavior
The taskbar SHALL retain 40-pixel logical rows, multi-row layout, optional labels, long labeled indicators, short icon indicators, grouping, progress and attention while using bounded Windows-like widths.

#### Scenario: Labeled and icon-only tasks coexist
- **WHEN** labeled and icon-only running tasks render together
- **THEN** labeled tasks use a 160-pixel cell with 144-pixel indicator and icon-only tasks retain compact short indicators

### Requirement: System region uses compact localized presentation
The notification/system region SHALL use bounded Windows-like spacing, explicit time/date typography and compact input-language text while preserving full accessible provider values.

#### Scenario: Traditional Chinese input profile
- **WHEN** the provider reports `zh-TW`
- **THEN** the visible input label is `中` and its UIA name retains the full provider identity

#### Scenario: Provider unavailable
- **WHEN** one system provider is unavailable
- **THEN** only that control reports unavailable and the remaining taskbar stays usable

### Requirement: Taskbar remains Explorer-free
SuperDesktop MUST render and operate every taskbar control without invoking Explorer, `Shell_TrayWnd`, system Start hosts or Explorer tray UI.

#### Scenario: Product source and process observation
- **WHEN** taskbar controls, overflow, IME and clock are exercised
- **THEN** owned callbacks complete without an Explorer-owned visible surface
