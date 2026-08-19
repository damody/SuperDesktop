## ADDED Requirements

### Requirement: Adaptive labeled task width
Labeled tasks SHALL shrink uniformly from 160 DIP to no less than 44 DIP before overflow, using available width and row count.

#### Scenario: Crowded one row
- **WHEN** labeled tasks do not fit at 160 DIP
- **THEN** all visible tasks shrink uniformly, remain ordered/non-overlapping and stop before reserved controls

#### Scenario: State layers
- **WHEN** progress, attention or running indicators render
- **THEN** their geometry stays inside the adaptive hit target

### Requirement: Automated admission
UTIT SHALL record task rectangles and prove width bounds, shrink, order, non-overlap, right exclusion and hashes.

#### Scenario: Runtime gate
- **WHEN** crowded taskbar capture runs
- **THEN** every visible task satisfies the adaptive contract without Explorer delegation
