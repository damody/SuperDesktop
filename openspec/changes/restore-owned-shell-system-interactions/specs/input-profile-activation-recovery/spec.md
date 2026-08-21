## ADDED Requirements

### Requirement: One bounded generation resynchronization
Input-profile activation SHALL use a current system-status host generation and exact enabled profile identity. A stale host generation SHALL trigger at most one authoritative refresh and replay only if the same target identity remains available.

#### Scenario: Host restarts before selection
- **WHEN** the user selects a profile from a snapshot whose host generation is stale
- **THEN** SuperDesktop refreshes once and activates the same exact still-enabled profile against the fresh generation

#### Scenario: Profile disappears during refresh
- **WHEN** the target profile is absent or changed after resynchronization
- **THEN** no different profile is activated and a truthful scoped failure is reported

### Requirement: Bounded exact-profile observation
After Windows accepts exact TSF or HKL activation, the provider SHALL refresh authoritative foreground/profile state until the requested stable ID is observed or the command deadline expires, without marking optimistic success.

#### Scenario: Delayed TSF propagation
- **WHEN** exact TSF activation succeeds but the first snapshot still reports the previous profile
- **THEN** bounded fresh observations continue and success is returned only after the requested stable ID appears

#### Scenario: Observation deadline
- **WHEN** the requested stable ID is not authoritatively observed before deadline
- **THEN** the terminal remains a provider failure or timeout, the prior UI selection is retained, and SuperDesktop remains alive

### Requirement: Pointer and Win+Space input switching parity
Pointer selection and Win+Space cycling SHALL resolve through the same exact-profile activation coordinator and SHALL not depend on Explorer being present.

#### Scenario: Explorer absent
- **WHEN** Explorer is absent and the user selects a profile by pointer or presses Win+Space
- **THEN** the selected/cycled exact enabled profile becomes authoritatively active or a truthful bounded error is shown

