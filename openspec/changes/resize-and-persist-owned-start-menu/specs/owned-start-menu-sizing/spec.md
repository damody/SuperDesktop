## ADDED Requirements

### Requirement: Owned Start supports native edge resizing
The system SHALL expose native resize hit targets on the titlebar-free owned Start window. It SHALL allow width and height changes while keeping the window contained above the active taskbar.

#### Scenario: User resizes from a window edge
- **WHEN** the user drags any supported Start window resize edge
- **THEN** the window updates continuously and remains between its effective minimum and maximum size

#### Scenario: Monitor is smaller than the normal minimum
- **WHEN** available work area is narrower or shorter than 420×360 DIP
- **THEN** the effective minimum collapses to the available size and Start remains fully visible

### Requirement: Start size persists as logical dimensions
The system SHALL persist optional Start width and height in device-independent pixels through the existing atomic settings store. Missing fields SHALL use 640×720 DIP defaults, and invalid or out-of-range values SHALL be normalized without preventing Start from opening.

#### Scenario: Existing settings have no size
- **WHEN** a pre-change settings file is loaded
- **THEN** Start opens at the default size clamped to the current monitor

#### Scenario: Saved size is restored after restart
- **WHEN** the user resizes Start, closes SuperDesktop, and starts it again on the same DPI
- **THEN** Start reopens at the saved logical width and height within native rounding tolerance

#### Scenario: Saved size moves to another DPI or monitor
- **WHEN** Start opens on a monitor with different DPI or available work area
- **THEN** its logical size is preserved where possible and clamped to full containment otherwise

### Requirement: Resize persistence is coalesced and terminal-safe
The system SHALL debounce resize persistence for 250ms and SHALL flush the latest observed size on deactivation and explicit close. It MUST avoid saving unchanged initial geometry and MUST preserve the last valid settings after a save failure.

#### Scenario: Continuous drag emits many bounds events
- **WHEN** multiple resize events occur within the debounce interval
- **THEN** only the latest logical size is submitted for persistence

#### Scenario: Start closes before debounce expires
- **WHEN** the taskbar toggle closes Start immediately after a resize
- **THEN** the latest observed size is flushed before the window is removed

#### Scenario: Settings save fails
- **WHEN** atomic persistence returns an error
- **THEN** the prior valid settings remain authoritative and a failure trace is emitted

### Requirement: Resized Start preserves taskbar anchoring
The system SHALL derive Start position from current taskbar alignment and the saved normalized size. Left alignment SHALL use the work-area margin, center alignment SHALL center the window, and the bottom edge SHALL preserve the configured taskbar gap in preview and owned-shell modes.

#### Scenario: Reopen with left alignment
- **WHEN** a non-default saved size reopens with left taskbar alignment
- **THEN** Start uses the left work-area margin and its bottom remains above the matching taskbar

#### Scenario: Reopen with center alignment
- **WHEN** the same saved size reopens with centered taskbar alignment
- **THEN** Start is horizontally centered and its bottom remains above the matching taskbar
