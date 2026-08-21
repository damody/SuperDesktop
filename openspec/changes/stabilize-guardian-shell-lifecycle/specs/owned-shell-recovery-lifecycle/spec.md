## ADDED Requirements

### Requirement: Guardian admission accepts equivalent Windows path identities
The system SHALL validate the guardian parent using the sealed process claim, exact process/session/creation identity, immutable executable volume and file index, and a normalized case-insensitive Windows path comparison. It MUST NOT accept a different file merely because its path text is similar.

#### Scenario: Extended path prefix identifies the same executable
- **WHEN** the parent executable is reported once as a normal DOS path and once with a Windows extended path prefix while immutable file identity is equal
- **THEN** guardian lease validation succeeds and publishes the nonce-bound acceptance

#### Scenario: Different executable file is presented
- **WHEN** normalized path text or immutable file identity identifies a different executable
- **THEN** guardian lease validation rejects the claim before Explorer shutdown

### Requirement: Guardian acceptance is bounded and diagnostically exact
The system SHALL wait at most five seconds for a nonce-bound guardian acceptance and SHALL observe guardian process lifetime during that wait. It SHALL distinguish an early guardian exit from a live-child acceptance timeout and preserve the typed child rejection in guardian diagnostics.

#### Scenario: Guardian accepts within the deadline
- **WHEN** the restricted guardian validates the inherited roles and writes the correct nonce-bound acknowledgement within five seconds
- **THEN** startup proceeds to Explorer shutdown and removes the acknowledgement artifact after closing parent-owned handles

#### Scenario: Guardian exits before acceptance
- **WHEN** the guardian process becomes signalled before a valid acknowledgement is observed
- **THEN** the parent reports an early-child-exit admission failure without waiting for the timeout and Explorer remains available

#### Scenario: Guardian remains live without accepting
- **WHEN** the guardian remains live but no valid acknowledgement arrives within five seconds
- **THEN** the parent reports `child-acceptance-timeout` and Explorer remains available

### Requirement: Exact owned shell always has a default-Explorer rollback
The system SHALL ensure a rollback record exists before guardian arming when the observed registry value exactly matches the current admitted SuperDesktop shell command. If history is absent in that exact state, the system SHALL record `explorer.exe` as the prior default shell. It MUST refuse reconstruction for any unknown or third-party shell value.

#### Scenario: Exact owned registration is missing its record
- **WHEN** the registry contains the current SuperDesktop executable with the required owned-shell arguments and the rollback file is absent
- **THEN** the system atomically creates a rollback record targeting `explorer.exe` before guardian arming

#### Scenario: Unknown shell is missing its record
- **WHEN** the rollback file is absent and the registry does not exactly match either the current owned SuperDesktop command or default Explorer
- **THEN** restoration fails closed without changing the registry

### Requirement: Return to default Explorer is idempotent
The system SHALL restore and verify `explorer.exe` from a valid or safely reconstructed rollback record and SHALL treat an already-default Explorer registration as successful when no record exists. Repeated return commands MUST NOT emit a missing-record error or change an unrelated shell value.

#### Scenario: Owned shell returns to Explorer
- **WHEN** the exact owned shell has a valid or reconstructed rollback record and the user invokes return to default Explorer
- **THEN** the registry is verified as `explorer.exe`, Explorer is recovered, and the rollback record is removed

#### Scenario: Explorer is already default
- **WHEN** the registry is already `explorer.exe`, the rollback file is absent, and return is invoked repeatedly
- **THEN** every invocation succeeds without mutation and without a missing-record error

### Requirement: Expected AppBar fallback is trace-only
The system SHALL retain owned monitor geometry and deterministic fallback trace markers when AppBar registration is unavailable in owned-shell mode. This expected degraded mode MUST NOT write a console warning, while genuine taskbar configuration failures MUST remain visible on the console.

#### Scenario: AppBar registration is unavailable
- **WHEN** owned-shell taskbar startup cannot register AppBar but can configure owned monitor geometry
- **THEN** the taskbar remains usable, both fallback trace markers are written, and no AppBar warning is written to stderr

#### Scenario: Taskbar configuration genuinely fails
- **WHEN** shell-hook registration or owned taskbar window configuration fails
- **THEN** startup reports the genuine error on the console and does not mislabel it as an AppBar fallback
