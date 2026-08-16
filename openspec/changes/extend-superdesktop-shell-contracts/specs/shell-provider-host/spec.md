## ADDED Requirements

### Requirement: Provider work is isolated
The system SHALL execute provider dispatch in a process separate from the GPUI shell process.

#### Scenario: Provider host exits unexpectedly
- **WHEN** the provider host terminates with active requests
- **THEN** the shell remains alive and reconciles those requests as provider failures

### Requirement: Dispatch is bounded
The provider host SHALL cap active requests and reject duplicate active request identifiers.

#### Scenario: Capacity is exhausted
- **WHEN** a new request arrives while the active-request limit is reached
- **THEN** the host returns a busy terminal outcome without starting the request

### Requirement: Deadlines and cancellation are enforced
The provider host SHALL check deadlines before dispatch and honor cancellation without emitting multiple terminal outcomes.

#### Scenario: Request is already expired
- **WHEN** a request deadline is not later than the host clock
- **THEN** the host emits a timeout terminal outcome and does not invoke the provider

### Requirement: Host health is observable
The provider host SHALL expose protocol, capability, capacity, and health information through a deterministic handshake.

#### Scenario: Client performs handshake
- **WHEN** a client sends a valid handshake request
- **THEN** the host returns supported protocol versions, capabilities, and configured limits
