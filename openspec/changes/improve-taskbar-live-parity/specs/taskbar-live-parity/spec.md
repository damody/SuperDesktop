## ADDED Requirements

### Requirement: New taskbars must default to never combine
New settings and taskbar objects missing `combine_groups` SHALL use `false`; explicitly stored `true` SHALL remain round-trippable.

#### Scenario: New settings
- **WHEN** default settings are created
- **THEN** each eligible window remains an independent task button

#### Scenario: Explicit grouping
- **WHEN** settings explicitly contain `combine_groups=true`
- **THEN** decoding and encoding preserve grouping capability

### Requirement: Multi-row taskbars must fill rows before columns
The task region SHALL place fixed/running buttons top-to-bottom across the configured row count before wrapping into the next horizontal column.

#### Scenario: Two-row taskbar with five entries
- **WHEN** two rows and five visible entries are rendered
- **THEN** both rows contain buttons and entries retain stable sequence

#### Scenario: One and three rows
- **WHEN** row count is one or three
- **THEN** the same column-major policy produces valid 40-logical-pixel hit targets without overlap

### Requirement: Production clock must reflect local system time
The platform layer SHALL read owned local date/time values from Windows, and each visible taskbar SHALL update its formatted time/date when those values change.

#### Scenario: Taskbar opens
- **WHEN** a production taskbar entity is created
- **THEN** its status uses current local time rather than a compile-time fixture

#### Scenario: Minute changes
- **WHEN** the observed local minute or date differs from the entity status
- **THEN** the entity replaces status and requests a new frame without changing task state
