## ADDED Requirements

### Requirement: Automatic hiding is an authoritative persisted setting
The system SHALL persist an `auto_hide` taskbar setting, default legacy or invalid values to false without changing unrelated fields, and expose one localized accessible toggle in the owned taskbar settings behavior section.

#### Scenario: Legacy settings load
- **WHEN** a settings file has no automatic-hide field
- **THEN** automatic hiding is false and all valid sibling fields retain their values

#### Scenario: Save fails
- **WHEN** the user activates the toggle and atomic persistence fails
- **THEN** the displayed and runtime state return to the previous authoritative value

### Requirement: Owned state controls reveal and delayed hiding
The system SHALL reveal immediately when the pointer reaches the owned reveal edge and SHALL hide only after the pointer remains outside the visible taskbar with no visibility hold for at least 500 milliseconds.

#### Scenario: Pointer reaches hidden edge
- **WHEN** automatic hiding is enabled, the taskbar is hidden, and the pointer reaches its two-physical-pixel reveal rectangle
- **THEN** the taskbar moves to its exact visible endpoint on the next reconciliation tick

#### Scenario: Pointer briefly leaves
- **WHEN** the visible taskbar pointer leaves for less than 500 milliseconds and returns
- **THEN** the pending hide is cancelled and the taskbar never reaches the hidden endpoint

### Requirement: Owned interactions hold visibility
The system SHALL keep the taskbar visible while Start, a task context or Jump List, taskbar settings, notification overflow, a system flyout, keyboard focus, native resize, or task attention requires taskbar interaction.

#### Scenario: Flyout opens during pending hide
- **WHEN** a system flyout opens while hiding is pending
- **THEN** the pending hide is cancelled and the taskbar remains fully visible until the hold ends

#### Scenario: Attention arrives while hidden
- **WHEN** a valid task attention state arrives while the taskbar is hidden
- **THEN** the taskbar reveals and remains visible while attention is active

### Requirement: Only the owned taskbar HWND moves
The system SHALL validate HWND liveness and current-process ownership before each endpoint move and SHALL perform zero mutation for null, retired, reused, or foreign HWNDs.

#### Scenario: Foreign HWND is supplied
- **WHEN** the platform adapter receives a live HWND owned by another process
- **THEN** it returns a typed ownership error and does not change that window's geometry

#### Scenario: Duplicate endpoint is requested
- **WHEN** the owned taskbar already has the requested exact client endpoint
- **THEN** the adapter reports no change without introducing bottom-edge or row-height drift

### Requirement: Preview and Shell preserve their bottom anchors
The system SHALL use the current work-area bottom in Preview, the physical monitor bottom in Shell, preserve the configured one-to-three-row visible height, and leave exactly two physical pixels visible at the hidden endpoint.

#### Scenario: Explorer-present Preview hides and reveals
- **WHEN** automatic hiding runs in Preview
- **THEN** both endpoints retain the pre-existing Explorer work-area bottom and Explorer remains running and unmodified

#### Scenario: Explorer-free Shell hides and reveals
- **WHEN** automatic hiding runs in Shell with Explorer absent
- **THEN** both endpoints use the physical monitor bottom and no Explorer, `Shell_TrayWnd`, or system taskbar UI is invoked

### Requirement: Lifecycle and work-area behavior are recoverable
The system SHALL skip Shell AppBar reservation while automatic hiding is enabled, restore ordinary reservation behavior when disabled, and restore the visible endpoint before normal shutdown or lease teardown.

#### Scenario: Automatic hiding is disabled while hidden
- **WHEN** the authoritative setting changes from enabled to disabled while the taskbar is hidden
- **THEN** the taskbar first returns to the exact visible endpoint and ordinary placement/reservation reconciliation resumes

#### Scenario: Cursor observation fails
- **WHEN** the platform cannot obtain a valid cursor position
- **THEN** it preserves the current endpoint, emits a typed unavailable trace, and does not hide or launch Explorer
