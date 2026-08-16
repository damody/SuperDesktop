## ADDED Requirements

### Requirement: SuperDesktop owns Start in shell mode
The system SHALL display a GPUI-owned Start surface with pinned, recent, all-apps, settings, and power sections when shell mode is active.

#### Scenario: Start is invoked in shell mode
- **WHEN** pointer, keyboard, or accessibility invokes Start
- **THEN** the same owned Start model opens and receives focus without delegating to Explorer

### Requirement: Search providers are cancellable and generation-bound
The system SHALL query application, file, and setting providers with deadlines, cancellation, and query generations.

#### Scenario: Query changes while results are pending
- **WHEN** a new committed query supersedes the active query
- **THEN** the old query is cancelled and its later batches are ignored

### Requirement: Results are deterministic and actionable
The system SHALL normalize, rank, group, and activate results deterministically with stable identity and typed actions.

#### Scenario: Prefix and substring results compete
- **WHEN** one title has a normalized prefix match and another only a substring match
- **THEN** the prefix match ranks first before stable tie breakers

### Requirement: Input and accessibility are complete
The system SHALL support keyboard navigation, focus return, IME composition/commit, localization-safe text, and accessible result/list/section semantics.

#### Scenario: IME composition changes
- **WHEN** composition text changes without commit
- **THEN** the displayed composition updates but no provider query is dispatched

### Requirement: Start performance is bounded
The system SHALL render the Start first frame within 250 milliseconds, return local app results within 150 milliseconds, and terminate all providers within two seconds.

#### Scenario: File provider exceeds deadline
- **WHEN** file search has not completed within two seconds
- **THEN** it becomes timed out while already returned app/settings results remain usable
