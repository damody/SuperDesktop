# Windows Notification Events Design

## Goal

Extend SuperDesktop's owned notification center from NotifyIcon balloon history to the user's real current Windows app notifications. Use only documented Windows Runtime notification-listener APIs, remain functional without Explorer, synchronize dismiss/clear operations back to Windows, and report permission or identity limitations truthfully.

## Chosen architecture

Create a `platform-win` `WindowsNotificationEventSource` around `Windows.UI.Notifications.Management.UserNotificationListener`. The adapter initializes a scoped WinRT apartment, checks or requests listener access, subscribes to `NotificationChanged`, and sets an atomic dirty flag from the no-unwind event callback. On startup, every dirty event, and a bounded periodic reconciliation, the notification host calls `GetNotificationsAsync(NotificationKinds::Toast)` and replaces only the Windows-origin portion of its owned history with the authoritative result.

Polling alone was rejected because it does not fulfill event support and introduces avoidable latency. Reading the private notification database or using ShellExperienceHost internals was rejected because the format and interfaces are undocumented. Reusing NotifyIcon ingress alone was rejected because it cannot observe modern Toast notifications.

## Protocol and identity

`NotificationSnapshot` gains an additive `windows_events` status containing access state (`allowed`, `denied`, `unspecified`, or `unavailable`), synchronization state, last event kind, and a bounded reason. Existing JSON defaults to `unavailable`, preserving compatibility.

Windows notifications use `notification_id = windows:<native-u32-id>` and a fixed `windows-events` client domain. A stable bounded hash of the AppUserModelId becomes the icon key's numeric component; raw AUMIDs are not exposed in IDs, traces, or evidence. Native IDs are revalidated against the current listener snapshot before removal. NotifyIcon notifications retain their existing identities and behavior.

## Content conversion

For each `UserNotification`, copy the native ID, creation time, App display name, and ToastGeneric text elements into owned Rust values. The first text element is the title; remaining elements are joined into the body. Empty/unusable Toast bindings are skipped. Every notification is processed inside an independent error boundary so one malformed source does not discard other notifications. Text, item count, and frame bounds are enforced before publication. App-logo extraction is deferred; Windows events use a truthful no-icon state until a separately bounded stream decoder is designed.

## Host lifecycle and events

`notification-area-host` owns the listener for its whole process lifetime. The WinRT event token is removed before apartment teardown. The callback copies only event kind/native ID into atomics/owned state and never blocks, panics, calls UI, or borrows WinRT event objects after return.

Before serving Snapshot or Health, the host reconciles if dirty or if the authoritative interval expired. Event storms coalesce into one reconciliation. A failed reconcile preserves the last valid Windows subset and changes `windows_events` to unavailable with a bounded reason. A later successful reconcile recovers without restarting the host.

## Dismiss and clear semantics

For `windows:<id>`, `DismissNotification` first checks expected generation and current native identity, then calls `RemoveNotification(id)`. `ClearNotifications` calls `ClearNotifications()` when the Windows subset is non-empty. Local state changes only after the Windows call succeeds and a fresh authoritative reconcile confirms the outcome. NotifyIcon-only dismissals remain local. Stale generation, denied access, disappeared ID, and WinRT failure are truthful rejected/no-change results and never remove a different notification.

## Owned UI

The existing notification cards display Windows app label, title, body, and creation time without inventing action buttons. The center adds a compact localized status banner when Windows access is denied, unspecified, unavailable, or still synchronizing. The empty state distinguishes "no current notifications" from "Windows notifications unavailable" while preserving existing NotifyIcon records. Clear-all and per-card dismiss continue to share pointer, keyboard, and UIA routes.

The Windows listener does not expose another app's Toast action buttons, so reply/open/custom buttons are out of scope. Do Not Disturb, Focus sessions, grouped-summary expansion, notification settings, and app activation are also separate capabilities.

## Permission and deployment behavior

At host startup, an `Allowed` listener begins immediately. `Unspecified` invokes `RequestAccessAsync` once; the resulting status is published. `Denied` never loops or opens Settings. If the current deployment lacks required package identity/capability, the adapter reports unavailable and keeps NotifyIcon functionality independent. The current machine's live preflight is `Allowed`; no packaging migration is required for this implementation cycle.

## Verification

Protocol tests cover additive defaults, access states, bounded reasons, and round trips. Platform tests cover timestamp conversion, ToastGeneric text reduction, app-key hashing, callback rundown, event coalescing, denied/unavailable states, and source contracts. Host tests cover startup sync, added/removed event reconciliation, recovery, deduplication, 100-item cap, stale dismissal, Windows remove/clear ordering, and NotifyIcon coexistence.

Live verification records access state and counts only, generates a controlled test Toast when possible, observes `NotificationChanged`, dismisses it through SuperDesktop, and confirms it disappears from Windows. Existing user notifications are never cleared by the test. Headful light/dark/high-contrast runs verify Windows-origin cards, provider status, keyboard/UIA dismiss, and no raw app/notification identity in committed evidence. Format, locked all-target check/test, warnings-as-errors Clippy, strict OpenSpec, and privacy scans remain blocking.

## Scope limits

This change does not read private Windows notification databases, invoke Explorer or ShellExperienceHost, simulate Toast action buttons, clear pre-existing user notifications during tests, alter notification permissions/settings, add Focus/Do Not Disturb controls, or claim Windows support when listener access is denied.
