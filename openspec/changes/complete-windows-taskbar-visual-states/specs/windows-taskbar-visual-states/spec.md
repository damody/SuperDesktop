## ADDED Requirements

### Requirement: Every running task has a Windows-style indicator
SuperDesktop SHALL render a stable bottom indicator for every available running task and SHALL distinguish active, inactive, minimized, grouped and unavailable states using Windows 11-compatible geometry.

#### Scenario: Active and inactive tasks coexist
- **WHEN** one task is foreground and another eligible task is inactive
- **THEN** both have indicators, with the active indicator wider and accented without removing the inactive indicator

#### Scenario: Grouped and minimized task
- **WHEN** a grouped task is minimized
- **THEN** grouped geometry remains visible at reduced emphasis and does not become indistinguishable from a single inactive task

### Requirement: Ordinary application progress is observed
The taskbar-state provider SHALL accept documented `ITaskbarList3` progress behavior from unchanged ordinary applications in committed Shell mode and MUST NOT fabricate progress in preview or provider failure.

#### Scenario: Determinate progress updates
- **WHEN** an application calls `SetProgressValue` with a live same-session HWND and valid completed/total values
- **THEN** the associated task displays the exact proportional normal progress and accessibility exposes the percentage

#### Scenario: Progress is removed
- **WHEN** the application sets `TBPF_NOPROGRESS`, closes, or its HWND generation retires
- **THEN** progress disappears without changing active, grouped, badge or attention state

#### Scenario: Invalid progress input
- **WHEN** total is zero, arithmetic is invalid, or PID/session/HWND identity is stale
- **THEN** the mutation fails closed and visible progress remains authoritative

### Requirement: Progress states and groups follow Windows priority
Normal, paused, error and indeterminate progress SHALL remain mutually exclusive per window and grouped buttons SHALL select error before paused before normal before indeterminate.

#### Scenario: Group priority collision
- **WHEN** grouped windows publish error, paused, normal and indeterminate progress
- **THEN** the group displays error, and removing it reveals paused then normal then indeterminate in priority order

#### Scenario: Same-priority determinate collision
- **WHEN** multiple grouped windows publish determinate progress at the same priority
- **THEN** the group displays the least complete valid progress

### Requirement: Progress appearance matches Windows semantics
SuperDesktop SHALL render normal progress green, paused progress yellow, error progress red and indeterminate progress as a moving green segment behind task content.

#### Scenario: Determinate progress presentation
- **WHEN** progress is normal, paused or error at 40 percent
- **THEN** exactly 40 percent of the task background uses the corresponding Windows state color while icon, label and running indicator remain visible

#### Scenario: Reduced motion
- **WHEN** reduced motion is enabled with indeterminate progress
- **THEN** a steady non-deceptive indeterminate state replaces motion and retains its accessible state

### Requirement: Attention requests flash and terminate correctly
SuperDesktop SHALL observe Windows attention requests, alternate a Windows-style amber task surface at the admitted cadence, and stop on count exhaustion, foreground activation, close or retirement.

#### Scenario: Finite attention request
- **WHEN** a background window requests a finite taskbar flash count
- **THEN** the task alternates exactly that bounded count and then retains a steady attention indicator until activation

#### Scenario: Flash until foreground
- **WHEN** a background window requests timer-no-foreground attention
- **THEN** flashing continues at the admitted cadence until that window becomes foreground and then clears immediately

#### Scenario: Progress and attention coexist
- **WHEN** a task has active progress and requests attention
- **THEN** attention flashing does not erase or reset progress, and clearing attention reveals unchanged progress

### Requirement: Taskbar state is isolated and recoverable
Taskbar progress and attention callbacks SHALL be process-isolated, generation-bound, bounded and no-unwind; host failure SHALL clear overlays without terminating task switching.

#### Scenario: Provider crashes
- **WHEN** the taskbar-state provider exits unexpectedly
- **THEN** task switching and indicators remain alive, progress/flash clear truthfully, and bounded restart waits for a new authoritative generation

#### Scenario: Event storm overflows
- **WHEN** progress or flash callbacks exceed capacity
- **THEN** terminal/stop events remain protected, overflow is recorded and one authoritative reconciliation is scheduled

### Requirement: Visual states remain accessible
Every task SHALL expose stable UIA role, task state, progress kind, exact determinate percentage and attention state; high contrast SHALL not rely only on opacity or color.

#### Scenario: UIA reads a progressing attention task
- **WHEN** a normal-progress task at 60 percent also requires attention
- **THEN** its accessible name/state reports running, 60 percent normal progress and attention without duplicate controls

#### Scenario: High contrast
- **WHEN** Windows high contrast is active
- **THEN** indicators, progress and attention remain distinguishable through explicit borders/patterns and accessible state

### Requirement: Completion is auditable on Windows 11
The change SHALL provide automated, headful, timing, accessibility, resource, Explorer-free and packaging evidence linked uniquely to tasks and binary hashes.

#### Scenario: Controlled Windows applications drive states
- **WHEN** controlled unchanged applications call `ITaskbarList3` and `FlashWindowEx`
- **THEN** captures and traces prove each visible state, cadence, HWND/PID/session ownership and subsequent cleanup

#### Scenario: Required reference comparison is missing
- **WHEN** geometry, color, cadence, UIA, Explorer-present non-interference or Explorer-free evidence is absent
- **THEN** the corresponding blocking gate remains failed
