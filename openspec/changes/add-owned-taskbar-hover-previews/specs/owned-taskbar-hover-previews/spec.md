## ADDED Requirements

### Requirement: Delayed stale-safe task hover
SuperDesktop SHALL open an owned preview only after 400 ms of continuous hover over an available single or grouped task.

#### Scenario: Delay boundary
- **WHEN** a pointer leaves before 400 ms
- **THEN** no preview opens or window action occurs

#### Scenario: Stable hover
- **WHEN** the same task remains hovered for at least 400 ms
- **THEN** one owned preview opens with fresh matching windows

#### Scenario: Rapid switch
- **WHEN** the pointer moves from task A to task B before A's timer completes
- **THEN** A never opens and only B can open after its own delay

### Requirement: Popup crossing and dismissal
The preview SHALL remain open while either its source task or popup is hovered and SHALL close 250 ms after both are left.

#### Scenario: Pointer crosses into popup
- **WHEN** the task is left and the popup is entered within the grace period
- **THEN** the popup remains open and the pending close becomes stale

#### Scenario: Both surfaces left
- **WHEN** neither source task nor popup is hovered for 250 ms
- **THEN** the popup closes and unregisters all thumbnails

#### Scenario: Auto-hidden taskbar preview
- **WHEN** an auto-hidden taskbar is revealed over a task and its owned preview opens
- **THEN** the preview holds taskbar visibility, closes after its 250 ms grace, and the existing 500 ms hide delay completes within 1500 ms total scheduler tolerance

### Requirement: Owned Windows preview presentation
The popup SHALL render content-sized Windows-style single/group cards with live thumbnail, title, close control, themes, keyboard, and UIA semantics.

#### Scenario: Live and unavailable cards
- **WHEN** DWM admits a source window
- **THEN** its live client thumbnail renders, otherwise a truthful Preview unavailable state renders without delegation

#### Scenario: Accessible actions
- **WHEN** pointer, Enter, Delete, Escape, or UIA invokes a supported action
- **THEN** the same typed activate, close, or dismiss path runs exactly once

### Requirement: Explorer-free automated admission
The hover feature SHALL be tested by UTIT with real pointer timing while Explorer is absent and safely restored.

#### Scenario: UTIT hover matrix
- **WHEN** the controlled hover case runs
- **THEN** early absence, delayed open, popup persistence, delayed close, task switch, UIA tree, process absence, and recovery are recorded with hashes

#### Scenario: Release admission
- **WHEN** the change is complete
- **THEN** automated, UTIT, strict/detailed, release, traceability, and both NSIS package gates pass while the change remains unarchived
