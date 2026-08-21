## ADDED Requirements

### Requirement: Smooth optimistic volume dragging
The system flyout SHALL update the displayed volume synchronously for pointer motion while sending native volume mutations through a bounded latest-value-wins coordinator with at most one command in flight.

#### Scenario: Continuous drag
- **WHEN** the pointer moves repeatedly across the volume slider faster than provider round-trips
- **THEN** the thumb follows each bounded local value and intermediate native requests are coalesced rather than queued

#### Scenario: Pointer release final value
- **WHEN** a volume drag ends at a value different from the last observed provider snapshot
- **THEN** that final value is committed after any in-flight request and authoritative reconciliation follows

#### Scenario: Native command failure
- **WHEN** the provider rejects or times out a volume mutation
- **THEN** SuperDesktop logs the scoped error, refreshes authoritative status, and remains interactive

### Requirement: Unified pointer and keyboard volume semantics
Pointer, Enter/Space controls, and keyboard volume increments SHALL use the same bounded coordinator and preserve exact 0 through 100 clamping and mute reconciliation.

#### Scenario: Boundary input
- **WHEN** a requested volume is below 0 or above 100 through repeated interaction
- **THEN** the submitted and displayed value remains clamped to the inclusive 0 through 100 range

