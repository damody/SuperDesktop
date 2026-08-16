## ADDED Requirements

### Requirement: Virtual desktop capabilities are explicit
The system SHALL separately report query, move-window, enumerate, switch, create, remove, and rename capabilities.

#### Scenario: Only documented manager is available
- **WHEN** the host supports `IVirtualDesktopManager` but no admitted enumeration adapter
- **THEN** window query/move are enabled while enumerate/switch/create/remove/rename remain explicitly unavailable

### Requirement: Window desktop operations are revalidated
The system SHALL validate window liveness and desktop identity immediately before query or move and reconcile after completion.

#### Scenario: Window retires before move
- **WHEN** a tracked window no longer maps to a live HWND at effect admission
- **THEN** the move fails closed and no COM operation is issued

### Requirement: Task View state is generation-bound
The system SHALL accept owned desktop snapshots with stable IDs and monotonic generations and ignore stale snapshots.

#### Scenario: Older snapshot arrives late
- **WHEN** a snapshot generation is below the current task-view generation
- **THEN** it is ignored without changing focus or window membership

### Requirement: Task View is accessible
The system SHALL provide keyboard, pointer, and accessibility parity for desktop focus, switching when supported, window movement, and dismissal.

#### Scenario: Unsupported switch is invoked
- **WHEN** a switch action is requested without switch capability
- **THEN** no effect is emitted and an accessible unavailable reason remains exposed

### Requirement: Optional mutations fail closed
The system SHALL never invoke undocumented create, remove, rename, enumeration, or switch contracts unless a separately admitted adapter provides them.

#### Scenario: Optional adapter probe fails
- **WHEN** its identity, version, or behavior probe fails
- **THEN** all optional mutation capabilities remain disabled for the session
