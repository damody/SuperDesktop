## ADDED Requirements

### Requirement: Backward-compatible owned notification records
The notification protocol SHALL represent admitted notifications using bounded owned values and SHALL remain readable when older payloads omit all notification-center fields.

#### Scenario: Older icon and snapshot payload
- **WHEN** a serialized icon or snapshot omits the additive notification fields
- **THEN** deserialization SHALL succeed with no notification and an empty notification history

#### Scenario: Oversized notification text
- **WHEN** a notification title, body, application label, or identity exceeds its protocol bound
- **THEN** validation SHALL reject the record before host mutation

### Requirement: Documented native balloon ingress
The committed-shell compatibility boundary SHALL copy documented `NIF_INFO` fields into owned memory before returning from `WM_COPYDATA` and MUST NOT retain borrowed pointers or read fields outside the submitted `cbSize`.

#### Scenario: Valid balloon-bearing add or modify
- **WHEN** a current-session live client submits a supported NotifyIcon payload with `NIF_INFO` and non-empty title or body
- **THEN** the translated operation SHALL contain the copied title, body, severity, realtime flag, owner identity, and generation

#### Scenario: Truncated or cross-session payload
- **WHEN** the frame is truncated, malformed, stale, foreign-session, or owned by a dead window
- **THEN** the compatibility boundary SHALL reject it without changing icon or notification history

#### Scenario: Empty informational fields
- **WHEN** `NIF_INFO` is set but both bounded title and body are empty
- **THEN** icon mutation MAY proceed but no notification-history record SHALL be created

### Requirement: Bounded authoritative notification history
The notification host SHALL retain at most 100 admitted records, deduplicate equivalent records, evict the oldest record first, and expose history only through authoritative snapshots.

#### Scenario: Capacity and eviction
- **WHEN** a distinct 101st notification is admitted
- **THEN** the oldest record SHALL be evicted and exactly 100 newest records SHALL remain

#### Scenario: Duplicate notification
- **WHEN** owner identity, notification generation, title, and body match an existing record
- **THEN** the operation SHALL report no history change and SHALL NOT create a second row

#### Scenario: Client disconnect
- **WHEN** an icon client disconnects after a notification was admitted
- **THEN** its live icons SHALL be removed but its notification history SHALL remain until dismissal, clear, or eviction

### Requirement: Typed dismiss and clear operations
The host SHALL provide generation-aware single-dismiss and clear-all mutations, and the UI SHALL wait for an authoritative snapshot before removing visible records.

#### Scenario: Single dismissal
- **WHEN** the user invokes dismiss for an existing notification at the observed host generation
- **THEN** the host SHALL remove only that record and the next snapshot SHALL omit it

#### Scenario: Stale dismissal
- **WHEN** a dismiss or clear request carries a stale expected generation
- **THEN** the host SHALL return no change and the UI SHALL retain the authoritative rows

#### Scenario: Clear all
- **WHEN** clear-all is invoked for a non-empty current history
- **THEN** the host SHALL atomically remove all history records without removing registered tray icons

### Requirement: Windows 11 combined notification and calendar surface
The owned clock affordance SHALL open one Explorer-free Windows 11-style popup containing notifications above the calendar, with deterministic light, dark, and high-contrast geometry.

#### Scenario: Populated center
- **WHEN** notification history is non-empty
- **THEN** the popup SHALL render localized header, clear-all action, application/title/body/time rows, real icons when available, dismiss actions, and the reachable calendar below

#### Scenario: Empty center
- **WHEN** notification history is empty
- **THEN** the popup SHALL render a localized owned empty state, disable or omit clear-all, retain the calendar, and SHALL NOT render a fake Settings action

#### Scenario: Constrained work area and 175 percent DPI
- **WHEN** the popup opens on a small, negative-origin, or 175% DPI monitor above a one-to-three-row taskbar
- **THEN** its logical bounds SHALL remain inside the monitor work area and the notification list SHALL scroll without clipping the calendar entry point

### Requirement: Accessible localized notification actions
Every notification and action SHALL expose complete UIA identity, keyboard behavior, visible focus, and non-color high-contrast state in Traditional Chinese and English.

#### Scenario: Keyboard dismissal and focus return
- **WHEN** a user reaches a notification by keyboard, invokes Delete or its dismiss button, and later presses Escape
- **THEN** the same typed dismiss action SHALL run and focus SHALL return to the owned clock control

#### Scenario: UIA clear all
- **WHEN** automation invokes the named clear-all button on non-empty history
- **THEN** the typed clear mutation SHALL run and a reconciled empty state SHALL be exposed

#### Scenario: Long visible text
- **WHEN** title or body exceeds visible card space
- **THEN** visual text SHALL be bounded without horizontal overflow while complete content remains in the accessible name

### Requirement: Explorer-free truthful ownership
Production notification-center composition MUST NOT invoke or require Explorer, system notification-center UI, `ShellExperienceHost`, `SystemSettings`, undocumented shell history APIs, or URI delegation.

#### Scenario: Explorer-absent operation
- **WHEN** Explorer is absent and an owned notification is admitted
- **THEN** the center SHALL open, dismiss, clear, close, and restore taskbar focus using only SuperDesktop-owned processes

#### Scenario: Unsupported Windows toast history
- **WHEN** no owned NotifyIcon notification was admitted
- **THEN** the center SHALL show the empty state and SHALL NOT imply that Windows toast history was queried
