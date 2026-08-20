## ADDED Requirements

### Requirement: Task preview clears the visible SuperDesktop taskbar
SuperDesktop SHALL place the task preview outer bounds entirely above the visible SuperDesktop taskbar with the configured popup gap.

#### Scenario: Explorer-compatible preview mode
- **WHEN** Explorer owns the Windows work area and SuperDesktop displays its taskbar immediately above the native taskbar
- **THEN** the preview bottom is no lower than the SuperDesktop taskbar top and does not overlap either taskbar

#### Scenario: Owned-shell mode
- **WHEN** SuperDesktop owns the shell taskbar at the monitor bounds bottom
- **THEN** the preview bottom is no lower than the owned taskbar top

### Requirement: Preview clearance follows taskbar layout
Preview clearance SHALL use the same effective shell mode, DPI, selected monitor, and supported 1–3 row count as the taskbar surface that initiated the preview.

#### Scenario: Multi-row taskbar
- **WHEN** the taskbar uses two or three rows
- **THEN** preview placement reserves the full DPI-scaled multi-row height

#### Scenario: Mixed-DPI negative-origin monitor
- **WHEN** the task button is on a monitor with non-96 DPI and a negative desktop origin
- **THEN** preview and taskbar bounds remain on that monitor and do not intersect

#### Scenario: Delayed hover
- **WHEN** the hover delay elapses after the taskbar captured its mode and row count
- **THEN** the opened preview uses that captured layout and still clears the visible taskbar

### Requirement: Recoverable preview geometry
Preview placement SHALL use a 96 DPI and one-row minimum fallback for invalid layout inputs and SHALL clamp the popup to the selected monitor without panicking.

#### Scenario: Invalid layout metrics
- **WHEN** DPI or row data is absent or outside supported bounds
- **THEN** SuperDesktop produces bounded non-overlapping geometry using fallback metrics and reports any platform error to the console
