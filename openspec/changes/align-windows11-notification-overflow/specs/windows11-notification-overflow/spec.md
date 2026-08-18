## ADDED Requirements

### Requirement: Overflow floats above the taskbar at every DPI
SuperDesktop SHALL convert logical overflow geometry to physical native bounds, clamp it to the active monitor work area and leave an eight-logical-pixel edge gap.

#### Scenario: Host 175 percent DPI
- **WHEN** hidden notification icons open on a 168-DPI monitor
- **THEN** the native popup has 1.75-scaled outer dimensions and is fully above rather than overlapping the taskbar edge

#### Scenario: Negative-origin or constrained monitor
- **WHEN** the active monitor has a negative origin or less space than the preferred popup
- **THEN** every popup edge remains inside that monitor's work area

### Requirement: Overflow uses Windows 11 visual density
The owned overflow SHALL render a rounded, bordered, shadowed panel with a bounded six-column grid, 48-pixel logical cells, 24-pixel icons and distinct hover, focus and pressed states.

#### Scenario: Pointer and keyboard states
- **WHEN** a user hovers, focuses, presses or keyboard-activates an icon
- **THEN** the state is visible and the same typed activate or context action is emitted exactly once

#### Scenario: High contrast
- **WHEN** high contrast is active
- **THEN** the panel, focus and unavailable states remain distinguishable without opacity or color alone

### Requirement: Overflow remains Explorer-free and accessible
The popup SHALL remain a SuperDesktop-owned dialog with stable UIA names, Escape/focus-loss dismissal and no Explorer or system overflow delegation.

#### Scenario: Ordinary client in Explorer-free Shell mode
- **WHEN** an ordinary `Shell_NotifyIcon` client is placed in overflow
- **THEN** its copied icon and tooltip are actionable by pointer, keyboard and UIA without an Explorer process
