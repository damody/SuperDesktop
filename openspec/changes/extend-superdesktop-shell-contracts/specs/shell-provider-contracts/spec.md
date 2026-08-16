## ADDED Requirements

### Requirement: Versioned provider envelopes
The system SHALL encode every provider request and response in a versioned envelope containing a request identifier and correlation identifier.

#### Scenario: Supported envelope is accepted
- **WHEN** a frame uses the supported protocol major version and valid identifiers
- **THEN** the frame is decoded into a typed provider request

#### Scenario: Unsupported envelope fails closed
- **WHEN** a frame uses an unsupported protocol major version
- **THEN** the system returns an unsupported-version terminal outcome without dispatching provider work

### Requirement: Bounded and validated inputs
The system SHALL validate frame size, text length, collection cardinality, deadlines, and payload-specific invariants before dispatch.

#### Scenario: Oversized frame is rejected
- **WHEN** an input frame exceeds the configured maximum byte length
- **THEN** the frame is rejected before deserialization or provider dispatch

### Requirement: Explicit request lifecycle
The system SHALL represent deadlines, cancellation, capabilities, progress, and exactly one terminal outcome with typed values.

#### Scenario: Request reaches one terminal state
- **WHEN** a dispatched request completes, expires, is cancelled, or fails
- **THEN** exactly one terminal response identifies the outcome and original request

### Requirement: Stable shell DTOs
The system SHALL provide owned DTOs for shell items, commands, search results, notification icons, task previews, and virtual desktops without exposing Win32 or GPUI types.

#### Scenario: Consumer uses DTO without platform dependency
- **WHEN** a platform-neutral consumer imports the protocol crate
- **THEN** it can construct and validate all shared DTOs without linking GPUI or Win32 adapters
