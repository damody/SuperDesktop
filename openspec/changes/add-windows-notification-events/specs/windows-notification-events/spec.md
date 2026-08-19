## ADDED Requirements

### Requirement: Truthful Windows notification access state
The system SHALL expose additive Windows notification-event access and synchronization state in every notification snapshot. It MUST request unspecified access at most once per host lifetime, MUST NOT re-prompt denied access, and MUST preserve independent NotifyIcon operation when Windows access is denied or unavailable.

#### Scenario: Allowed startup
- **WHEN** `UserNotificationListener.GetAccessStatus` returns Allowed
- **THEN** the host subscribes to NotificationChanged and publishes allowed state after an authoritative initial sync

#### Scenario: Unspecified startup
- **WHEN** access is Unspecified
- **THEN** the host calls RequestAccessAsync once and publishes the returned allowed/denied state without opening Settings

#### Scenario: Denied or identity unavailable
- **WHEN** access is denied or WinRT activation/package identity is unavailable
- **THEN** Windows state is truthful, no repeated permission request occurs, and NotifyIcon notifications remain visible/actionable

#### Scenario: Legacy payload
- **WHEN** a client deserializes a snapshot without Windows-event state
- **THEN** it receives a safe unavailable default and all existing fields remain valid

### Requirement: Event-driven authoritative Toast reconciliation
The host SHALL subscribe to documented NotificationChanged added/removed events, coalesce event storms, and reconcile the current Toast snapshot on startup, dirty events, and a bounded authoritative interval. A malformed notification MUST NOT discard other valid notifications or the last valid Windows subset.

#### Scenario: Added event
- **WHEN** Windows raises NotificationChanged Added
- **THEN** the next reconciliation publishes the new valid Toast with added state and a newer generation

#### Scenario: Removed event
- **WHEN** Windows raises NotificationChanged Removed
- **THEN** the next reconciliation removes the absent Windows-origin item without affecting NotifyIcon records

#### Scenario: Event storm
- **WHEN** multiple add/remove callbacks arrive before reconciliation
- **THEN** they coalesce into one bounded authoritative query and the final snapshot matches Windows

#### Scenario: Periodic recovery
- **WHEN** an event is missed or a previous query failed
- **THEN** the five-second authoritative reconciliation restores the current Windows subset and allowed/synchronized state

#### Scenario: Per-item failure
- **WHEN** one UserNotification has a missing binding, invalid text, time, or AppInfo
- **THEN** that item is skipped, other valid items remain, and collection/text/frame bounds still pass

### Requirement: Bounded Windows Toast conversion and coexistence
Each valid UserNotification SHALL convert App display name, native ID, creation time, ToastGeneric title, and body into a bounded owned notification. Windows IDs MUST use the fixed `windows:` domain, raw AUMIDs MUST NOT be exposed, Windows and NotifyIcon records SHALL coexist, and the combined history SHALL remain newest-first at the 100-item cap.

#### Scenario: ToastGeneric text
- **WHEN** a Toast contains multiple text elements
- **THEN** the first becomes title and bounded remaining elements become newline-joined body

#### Scenario: Stable private identity
- **WHEN** an AppInfo has an AUMID and native u32 notification ID
- **THEN** the owned key contains only a stable hash/fixed domain and the notification ID contains only the validated native number

#### Scenario: Duplicate reconciliation
- **WHEN** unchanged Windows notifications are reconciled repeatedly
- **THEN** no duplicate is added and generation does not advance solely because of polling

#### Scenario: Combined capacity
- **WHEN** Windows and NotifyIcon records exceed 100
- **THEN** deterministic newest records are retained without exceeding protocol collection/frame bounds

### Requirement: Confirmed synchronized dismiss and clear
The system MUST validate expected generation and exact Windows identity before mutation. Windows-origin dismiss SHALL call RemoveNotification and confirm absence before local removal. Clear-all SHALL call ClearNotifications and confirm the Windows subset empty before clearing local Windows state. Failure MUST preserve the prior published snapshot; NotifyIcon-only dismissal remains local.

#### Scenario: Successful Windows dismiss
- **WHEN** a current `windows:<id>` notification is dismissed with current generation
- **THEN** RemoveNotification receives that exact u32 ID and the item disappears locally only after authoritative confirmation

#### Scenario: Stale or absent ID
- **WHEN** expected generation is stale, ID is malformed/oversized, or the native item is no longer current
- **THEN** no Windows remove call occurs and a different notification cannot be removed

#### Scenario: Windows remove failure
- **WHEN** RemoveNotification fails or post-query still contains the item
- **THEN** the host returns Rejected/no-change and preserves the prior local item

#### Scenario: Successful clear-all
- **WHEN** clear-all is requested with current generation and Windows-origin items exist
- **THEN** ClearNotifications is called once, absence is confirmed, and both confirmed Windows items and local NotifyIcon history are cleared according to the existing command semantics

### Requirement: Owned provider state and accessible cards
The owned notification center SHALL render Windows-origin cards with app label, title, body, and time, coexist with NotifyIcon cards, and show a localized access/synchronization banner for denied, unspecified, unavailable, or synchronizing states. It MUST NOT render unsupported Toast action buttons. Dismiss/Delete/Clear interactions SHALL retain pointer, keyboard, and UIA parity.

#### Scenario: Allowed populated center
- **WHEN** synchronized Windows and NotifyIcon notifications exist
- **THEN** both origins appear in the bounded scroll list with one clear-all action and accessible dismiss controls

#### Scenario: Denied or unavailable center
- **WHEN** Windows access is denied/unavailable and no owned records exist
- **THEN** the center shows the truthful provider state instead of claiming there are no Windows notifications

#### Scenario: Synchronizing with retained data
- **WHEN** reconciliation is pending or transiently failed after a valid snapshot
- **THEN** retained cards remain visible with provider status and no fabricated changes

#### Scenario: Unsupported actions absent
- **WHEN** production UI source and UIA tree are inspected
- **THEN** no reply/open/custom Toast action is claimed and existing dismiss/Delete/Clear routes remain equivalent

### Requirement: Privacy-safe real Windows evidence
Completion SHALL require protocol/platform/host/UI tests, callback rundown and mutation ordering gates, live access/count evidence, a controlled added/removed Toast event and exact dismiss confirmation when safely available, themed headful UIA evidence, full format/check/test/Clippy, strict OpenSpec, and privacy scanning. Committed evidence MUST NOT contain raw AUMIDs, native IDs, app labels, titles, bodies, or identity-bearing screenshots.

#### Scenario: Controlled Toast available
- **WHEN** a controlled Toast can be generated without touching existing user notifications
- **THEN** Added is observed, the card appears, SuperDesktop dismisses only that Toast, Removed is observed, and redacted pass evidence is saved

#### Scenario: Controlled Toast not applicable
- **WHEN** the environment cannot safely create a controlled Toast
- **THEN** live mutation is evidence-backed not-applicable while real read-only access/count, event source, simulated command-safety, UI and all quality gates still pass

#### Scenario: Privacy scan
- **WHEN** staged evidence is scanned before completion
- **THEN** no raw Windows notification/app/content identity is present and identity-bearing live screenshots are excluded
