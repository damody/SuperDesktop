## ADDED Requirements

### Requirement: Source-anchored content-sized Jump List
Owned Jump Lists SHALL center over the invoking task, clamp to the monitor, sit above the owned taskbar and size height from visible commands.

#### Scenario: Common two-command list
- **WHEN** a task exposes two local commands
- **THEN** the panel uses bounded row-derived height rather than 480 DIP

#### Scenario: Edge and fallback
- **WHEN** the task is at an edge or cursor is unavailable
- **THEN** placement clamps or centers without leaving the monitor

### Requirement: DPI and runtime mode parity
Geometry SHALL preserve logical width/rows across 96–216 DPI, preview/shell modes and one-to-three taskbar rows.

#### Scenario: Matrix
- **WHEN** synthetic geometry is evaluated
- **THEN** conversion occurs once and popup never overlaps taskbar

### Requirement: Automated owned admission
UTIT SHALL right-click a controlled task and record source/popup/taskbar geometry, Menu/MenuItem UIA, screenshot, binary hash and no-delegation evidence.

#### Scenario: Runtime gate
- **WHEN** the taskbar case runs
- **THEN** popup is content-sized, anchored, contained, actionable and owned
