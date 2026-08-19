## ADDED Requirements

### Requirement: Locale-correct clock content
The owned status model SHALL format authoritative local seconds into locale-correct long time, weekday, and short date strings. Traditional Chinese SHALL use `上午/下午 hh:mm:ss`, `星期X`, and `yyyy/M/d`; English SHALL use `hh:mm:ss AM/PM`, a full weekday, and `M/d/yyyy`.

#### Scenario: Traditional Chinese afternoon
- **WHEN** local time is 2026-08-19 15:30:23 in Traditional Chinese
- **THEN** the values are `下午 03:30:23`, `星期三`, and `2026/8/19`

#### Scenario: Midnight and noon
- **WHEN** the formatter receives hour 0 or 12
- **THEN** it emits 12 as the 12-hour value with the correct AM/PM marker

#### Scenario: English order
- **WHEN** the same value is formatted in English
- **THEN** it emits `03:30:23 PM`, `Wednesday`, and `8/19/2026`

#### Scenario: One-second advancement
- **WHEN** the authoritative local second changes
- **THEN** status equality changes and the visible clock updates without fabricating intermediate time

### Requirement: Row-aware centered presentation
The clock SHALL reserve one 112-DIP Button column. At three taskbar rows it SHALL render time, weekday, and date; at one or two rows it SHALL render time and date. Every visible line MUST occupy the full control width, remain non-wrapping, and explicitly center its text. The task-button reservation MUST include the same clock-width delta.

#### Scenario: Three-row taskbar
- **WHEN** the taskbar has three rows
- **THEN** all three strings are visible, centered on the same axis, and contained within the clock Button

#### Scenario: One- or two-row taskbar
- **WHEN** the taskbar has one or two rows
- **THEN** weekday is omitted and the two remaining lines stay contained and centered

#### Scenario: DPI and theme matrix
- **WHEN** the taskbar renders at 96, 144, 168, or 216 DPI in light, dark, or high contrast
- **THEN** the clock remains inside the taskbar and does not overlap tasks or adjacent status controls

### Requirement: Accessible unchanged activation
The clock SHALL remain one Button whose UIA label lists the visible time fields in visual order. Pointer, Enter, and Space SHALL continue opening the owned calendar exactly once, and focus styling SHALL remain visible.

#### Scenario: Three-row UIA label
- **WHEN** UI Automation inspects a three-row Traditional Chinese clock
- **THEN** its accessible name contains long time, weekday, and date in that order

#### Scenario: Calendar activation parity
- **WHEN** the clock is invoked by pointer, Enter, or Space
- **THEN** every route emits the same owned Calendar flyout action exactly once
