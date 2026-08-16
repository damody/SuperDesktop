## ADDED Requirements

### Requirement: Group flyouts provide window previews
The system SHALL show a bounded flyout for grouped windows with title, state, activation, close, and a live thumbnail when supported.

#### Scenario: DWM preview is unavailable
- **WHEN** preview capability is unavailable or capture exceeds 500 milliseconds
- **THEN** the flyout shows an explicit title/icon fallback and keeps activation and close actions

### Requirement: Jump Lists are safe and actionable
The system SHALL render sanitized recent, frequent, task, pin, and close commands with typed invocation.

#### Scenario: Provider returns too many destinations
- **WHEN** a Jump List exceeds the configured item limit
- **THEN** excess items are dropped deterministically before rendering

### Requirement: Task state overlays are independent
The system SHALL represent normal, indeterminate, paused, error, and cleared progress independently from attention state.

#### Scenario: Attention arrives during progress
- **WHEN** a task requests attention while progress is active
- **THEN** both states remain observable and clearing one does not clear the other

### Requirement: Advanced preferences persist and reconcile
The system SHALL persist pin order, grouping, label, preview, and multi-monitor preferences in a versioned snapshot.

#### Scenario: A persisted application is unavailable
- **WHEN** settings load with a pin that no longer resolves
- **THEN** the missing pin is omitted while surviving pins retain relative order

### Requirement: Advanced interactions are accessible
The system SHALL provide keyboard, pointer, and accessibility parity for flyout focus, activation, close, Jump Lists, and dismissal.

#### Scenario: Keyboard closes a previewed window
- **WHEN** the focused flyout item receives its close shortcut
- **THEN** it emits the same revalidated close action as pointer close
