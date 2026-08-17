## ADDED Requirements

### Requirement: Completion evidence covers every implemented domain
The verifier SHALL require exactly one attributable evidence source for each completion child and SHALL reject missing, duplicate, malformed, or unexpected sources.

#### Scenario: One child artifact is absent
- **WHEN** the roll-up runs without evidence for any required child change
- **THEN** traceability fails and release remains blocked

### Requirement: Cross-domain behavior remains deterministic and bounded
The verifier SHALL execute desktop, context-menu, Start/search, taskbar, notification-area, virtual-desktop, provider-host, and installer tests with their published bounds.

#### Scenario: A provider result arrives after cancellation
- **WHEN** a stale provider result is replayed during the cross-domain suite
- **THEN** it cannot modify visible state and the terminal result remains exactly once

### Requirement: Capability claims are truthful
The verifier SHALL distinguish implemented documented capabilities, owned replacement protocols, and unavailable optional compatibility features.

#### Scenario: Undocumented virtual-desktop controls are unavailable
- **WHEN** enumerate, switch, create, remove, or rename is not supported by the documented adapter
- **THEN** the roll-up records the limitation and makes no native parity claim

### Requirement: External release gates fail closed
The verifier SHALL require exact Windows 11 build 26200.9168＋ExplorerPatcher 26100.8457.70.3 lifecycle/reboot evidence, physical mixed-DPI evidence, and independent review before allowing release. Windows 10 compatibility SHALL be recorded as not claimed and SHALL NOT block this release.

#### Scenario: Only local automated evidence exists
- **WHEN** all workspace tests pass but one or more external artifacts are absent
- **THEN** local gates may pass while `release_allowed` remains false

### Requirement: Verification performs no shell mutation
The local verifier SHALL run only memory fixtures and live read-only preflight; physical mutation requires the separate explicit-opt-in collector.

#### Scenario: Local verification runs on a developer workstation
- **WHEN** the completion collector executes without physical apply authority
- **THEN** it neither writes the Winlogon Shell value nor creates installer rollback metadata

### Requirement: Final disposition is derived and auditable
The verifier SHALL emit versioned machine-readable gate, source, limitation, command, timestamp, and derived release fields without secrets.

#### Scenario: A required gate is pending
- **WHEN** any required gate is not `passed`
- **THEN** the emitted disposition is blocked and identifies the exact pending gate
