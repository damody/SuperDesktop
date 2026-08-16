## ADDED Requirements

### Requirement: The program has a fixed dependency order
The program SHALL enumerate every completion child exactly once in dependency order and SHALL reject missing, duplicate, unexpected, or prematurely completed dependents.

#### Scenario: Verification appears before an implementation child
- **WHEN** the program manifest orders verification before a required production child
- **THEN** program traceability fails and no completion disposition is emitted

### Requirement: Production implementation preserves shell safety
Every completion feature SHALL preserve explicit shell opt-in, bounded owned state, isolated provider execution, fail-closed platform admission, and exact recovery contracts.

#### Scenario: A feature attempts shell mutation during ordinary launch
- **WHEN** any completion surface is launched without explicit takeover or installer authority
- **THEN** it performs no shell mutation and the safety gate fails if mutation is observed

### Requirement: Capability claims are exact
The program SHALL distinguish documented implemented functionality, owned replacement protocols, and unavailable optional compatibility behavior.

#### Scenario: The user asks whether every Windows desktop feature is identical
- **WHEN** legacy Explorer notification protocols or undocumented virtual-desktop operations are not implemented
- **THEN** the program reports those limitations and does not claim complete native parity

### Requirement: Implementation completion differs from release approval
The program SHALL derive `implementation_complete` from production children and `release_allowed` from all mandatory verification gates.

#### Scenario: All local implementation and tests pass but physical gates are pending
- **WHEN** every production child is complete and local verification passes without external evidence
- **THEN** `implementation_complete` is true and `release_allowed` is false

### Requirement: Program state is attributable and unarchived
The program SHALL record child task counts, commits, evidence paths, limitations, blockers, and timestamps, and SHALL not archive without a separate explicit user request.

#### Scenario: Local program roll-up is published
- **WHEN** the parent apply workflow records current status
- **THEN** all child lineage and external blockers are machine-readable while every active change remains unarchived
