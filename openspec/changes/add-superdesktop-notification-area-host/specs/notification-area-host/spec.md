## ADDED Requirements

### Requirement: Notification clients are isolated
The system SHALL host notification client registration and event delivery outside the GPUI process.

#### Scenario: Notification host crashes
- **WHEN** the host process exits unexpectedly
- **THEN** the taskbar remains alive, marks the provider unavailable, clears stale icons, and can reconcile a later snapshot

### Requirement: Icon lifecycle is generation-bound
The system SHALL support add, modify, delete, focus, client disconnect, and full snapshot operations with stable client/icon identity and monotonic generations.

#### Scenario: Stale modify arrives
- **WHEN** a modify operation has an older generation than the registered icon
- **THEN** it is ignored without changing rendered state

### Requirement: Registry and events are bounded
The system SHALL bound clients, icons, icon bytes, tooltip text, and pending events while protecting activation and context events.

#### Scenario: Icon capacity is exhausted
- **WHEN** a client adds an icon beyond the configured registry capacity
- **THEN** the host rejects the icon and preserves existing registrations

### Requirement: Visible and overflow layouts are accessible
The system SHALL provide deterministic visible/overflow ordering, DPI-aware icon sizes, tooltips, keyboard focus, activation, context actions, and accessible names.

#### Scenario: Keyboard opens overflow
- **WHEN** the overflow button receives its invoke action
- **THEN** the overflow surface opens and focuses its first available icon

### Requirement: Notification latency is observable
The system SHALL timestamp admitted client events and expose evidence that p95 event-to-model latency remains below 100 milliseconds.

#### Scenario: Activation event completes
- **WHEN** an icon activation is delivered
- **THEN** its correlation and admission/completion timestamps are recorded
