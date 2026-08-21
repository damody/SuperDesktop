## ADDED Requirements

### Requirement: Long-lived event-driven Windows notification reconciliation
The notification host SHALL keep the WinRT apartment, `UserNotificationListener`, and `NotificationChanged` subscription alive together, convert native callbacks into bounded dirty state without borrowing UI state, and reconcile on startup, dirty events, and a bounded recovery cadence.

#### Scenario: Added notification event
- **WHEN** Windows raises an Added event for a valid Toast
- **THEN** the next coalesced refresh publishes the notification once with a newer authoritative generation

#### Scenario: Removed notification event
- **WHEN** Windows raises a Removed event
- **THEN** the next coalesced refresh removes only the absent Windows-origin notification

#### Scenario: Event storm or callback panic
- **WHEN** multiple events arrive before refresh or one callback payload is malformed
- **THEN** events coalesce, panic is contained, valid records remain available, and the host stays alive

### Requirement: Confirmed Windows notification mutations
Dismiss and clear commands SHALL validate the current generation and exact native identity, execute the documented WinRT mutation, and publish removal only after an authoritative snapshot confirms absence.

#### Scenario: Exact dismiss succeeds
- **WHEN** a current Windows-origin notification is dismissed
- **THEN** only its exact native ID is removed and the updated snapshot is published after confirmation

#### Scenario: Clear succeeds
- **WHEN** clear-all is invoked against current Windows notification state
- **THEN** Windows notifications are cleared once and the owned center publishes the confirmed empty Windows subset

#### Scenario: Mutation failure
- **WHEN** access is denied, identity is stale, or post-mutation confirmation fails
- **THEN** the prior published notification state is retained, a scoped error is logged, and NotifyIcon interaction remains available

