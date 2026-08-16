## ADDED Requirements

### Requirement: Typed desktop mutations
The system SHALL represent rename, recycle, permanent delete, copy, move, refresh, and reposition as typed operations with correlation identifiers and terminal outcomes.

#### Scenario: Rename succeeds
- **WHEN** a valid desktop item is renamed to a non-colliding valid name
- **THEN** the operation reports success and schedules namespace reconciliation

#### Scenario: Invalid mutation is rejected
- **WHEN** an operation has an empty name, unsupported target, or implicit overwrite
- **THEN** the system rejects it before filesystem mutation

### Requirement: Safe delete behavior
The system SHALL use the Recycle Bin by default and require explicit policy for permanent deletion.

#### Scenario: Normal delete is requested
- **WHEN** the user invokes Delete without the permanent-delete modifier
- **THEN** the system requests a recycle operation and does not call permanent filesystem deletion

### Requirement: Cancellable transfers
The system SHALL expose byte/item progress, cooperative cancellation, collision policy, and per-item outcomes for copy and move operations.

#### Scenario: Copy is cancelled
- **WHEN** cancellation is observed during a file copy
- **THEN** the incomplete destination created by that operation is removed and the source remains intact

### Requirement: Deterministic desktop arrangement
The system SHALL support name, kind, size, and modified-time ordering, grid alignment, and persistent logical positions per stable item and monitor identity.

#### Scenario: Items are sorted by name
- **WHEN** name ordering is selected
- **THEN** items use case-insensitive natural ordering with stable identity as the tie breaker

### Requirement: Reconciliation after effects
The system SHALL refresh authoritative namespace state after every terminal mutation and restore selection only for surviving stable identities.

#### Scenario: External race changes an item
- **WHEN** a watcher delta conflicts with an in-flight mutation result
- **THEN** authoritative enumeration wins and stale deltas cannot resurrect removed state
