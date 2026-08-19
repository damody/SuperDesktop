## ADDED Requirements

### Requirement: Settings window exposes fixed close control
The owned taskbar settings window SHALL display a fixed top-right close button that remains visible while content scrolls and SHALL invoke the same authoritative dismissal route as Escape.

#### Scenario: Pointer closes settings
- **WHEN** the user activates the visible close button
- **THEN** the settings window is removed and its shared window slot is cleared

#### Scenario: Content scrolls beneath fixed chrome
- **WHEN** the user scrolls from the top toward the bottom of the settings content
- **THEN** the close button remains at the same window-relative position and remains operable

#### Scenario: Close control is keyboard and UIA accessible
- **WHEN** keyboard or automation clients inspect and activate the close control in English or Traditional Chinese
- **THEN** it exposes Button semantics, a localized close name, visible focus, and successful dismissal

### Requirement: Settings window exposes synchronized vertical scrollbar
The owned taskbar settings window SHALL display a vertical scrollbar when content height exceeds viewport height and SHALL synchronize its thumb with the tracked content scroll offset.

#### Scenario: Overflowing content opens at the top
- **WHEN** the expanded settings page exceeds the viewport
- **THEN** a visible right-edge scrollbar exposes 0–100 accessibility range and its thumb starts at the top

#### Scenario: Wheel scrolling updates thumb
- **WHEN** wheel or keyboard scrolling changes the tracked content offset
- **THEN** the thumb position and accessibility percentage update to represent the bounded offset

#### Scenario: User drags thumb
- **WHEN** the user drags the scrollbar thumb toward the bottom of its track
- **THEN** the tracked content offset moves proportionally, bottom content becomes visible, and the thumb remains inside the track

#### Scenario: Content fits viewport
- **WHEN** viewport height is at least the complete content height
- **THEN** no active scrollbar is painted and scrolling arithmetic performs no division by zero

#### Scenario: Expanded content later shrinks
- **WHEN** sections collapse after the page has been scrolled
- **THEN** the content offset and thumb are clamped to the new maximum without stale or out-of-track state

### Requirement: Settings chrome preserves visual and behavioral contracts
Close and scrollbar chrome SHALL use existing settings theme tokens, preserve content width and DPI conversion, and SHALL NOT change settings persistence or add a native title bar.

#### Scenario: Theme and DPI matrix
- **WHEN** settings render in light, dark, or high contrast at supported DPI scales
- **THEN** close and scrollbar controls remain visible, non-overlapping, and focus-distinguishable within the window bounds

#### Scenario: Existing settings interaction
- **WHEN** users toggle, expand, save, reject, or reopen existing settings
- **THEN** model revisions, persistence, error handling, and section behavior remain unchanged
