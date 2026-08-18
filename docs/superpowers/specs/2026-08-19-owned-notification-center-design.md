# Explorer-free owned notification center design

## Outcome

SuperDesktop will render a Windows 11-style notification center above its owned taskbar without launching Explorer, `ShellExperienceHost`, `SystemSettings`, or an `ms-settings:` URI. The center will retain only notifications actually admitted through the documented NotifyIcon `NIF_INFO` compatibility ingress or an explicit owned provider contract. It will not claim access to Windows toast history.

The clock/calendar affordance will open one owned popup containing a bounded notification section above the existing calendar. Users can dismiss one notification, clear all notifications, navigate the center by keyboard, and retain full UI Automation names and actions.

## Chosen approach

The implementation extends the existing versioned notification protocol and host instead of creating a UI-only placeholder or querying undocumented Windows notification databases.

- A UI-only empty surface was rejected because it would look complete while providing no notification behavior.
- Undocumented shell/WinRT history scraping was rejected because it would create an Explorer-owned dependency and a brittle release boundary.
- A bounded owned history sourced from `NIF_INFO` is selected because the compatibility window already copies documented `NOTIFYICONDATAW` payloads and validates client process/session identity.

## Protocol and compatibility

`OwnedNotification` contains a stable notification ID, owning icon identity, title, body, severity, admitted timestamp, and generation. Title and body are bounded by the existing protocol text limits. The optional notification payload on `OwnedNotifyIcon` uses `#[serde(default)]`, and `NotificationSnapshot.notifications` also defaults to an empty list so older serialized clients remain readable.

`NotificationMutation` gains `DismissNotification` and `ClearNotifications`. Both operations are host-authoritative and generation-aware. A stale dismissal is `NoChange`; an unknown client cannot clear another session's history. The host keeps at most 100 notifications, deduplicates repeated icon/generation/content records, and evicts the oldest record first. Disconnecting an icon client removes its active icon but does not erase already admitted history, matching notification-center history semantics.

## Native ingress

The NotifyIcon compatibility decoder copies documented `NIF_INFO` fields only when the submitted `cbSize` contains them and `NIF_INFO` is set. It copies `szInfo`, `szInfoTitle`, `dwInfoFlags`, timeout/version union, and `NIF_REALTIME` into owned Rust data before returning from `WM_COPYDATA`. Empty title and body produce no history item. Invalid UTF-16 uses a bounded lossy conversion, while oversized or malformed frames fail closed before mutation.

Preview mode never claims the `Shell_TrayWnd` identity and cannot admit arbitrary external NotifyIcon traffic. Committed shell mode continues to require current-session, live-window ownership checks.

## Host state and data flow

1. `WM_COPYDATA` is copied and validated in `platform-win`.
2. Compatibility translation produces an add/modify operation with an optional owned notification.
3. `notification-area-host` applies icon and notification changes atomically, increments one authoritative generation, and returns a snapshot.
4. `superdesktop-app` reconciles the latest snapshot and passes notification history plus typed dismiss/clear callbacks into the owned calendar popup.
5. `taskbar-ui` renders the notification list and emits only typed host operations.

No UI component mutates history optimistically. It waits for the next authoritative snapshot; rejected requests leave the current rows visible and expose a non-sensitive error state.

## Windows 11 presentation

The combined popup uses the existing system-flyout light, dark, and high-contrast tokens. At normal width it is 380 logical pixels wide and clamps vertically to the monitor work area above one to three taskbar rows. The notification portion owns vertical scrolling; the calendar remains reachable below it.

Each notification card has an application label, bounded title, up to three visible body lines, relative/clock time, and a 32-pixel icon when real pixels exist. The dismiss button is 32 by 32 logical pixels with hover, pressed, focus, and high-contrast border states. The header contains localized `Notifications`/`通知` and an enabled `Clear all`/`全部清除` button only when history is non-empty. Empty history displays an owned localized empty state without a fake Settings link.

## Accessibility and input

The popup is a dialog. The notification collection is a list, each notification is a list item, dismiss uses a named button, and clear-all uses a named button. Tab/Shift+Tab follow visual order; Enter/Space invoke; Delete dismisses the focused notification; Escape closes the popup and restores focus to the taskbar clock control. Full title/body/application identity remains in UIA even when visible text is ellipsized.

High contrast uses explicit borders and focus geometry, not color alone. Traditional Chinese and English visible strings are complete; other locales fall back to English without changing reading order or action identity.

## Failure handling and limits

- Maximum retained notifications: 100.
- Maximum title/body size: existing `MAX_TEXT_BYTES` per field.
- Duplicate notification key/content/generation: no change.
- Stale dismiss or clear generation: no change and no row loss.
- Host unavailable: existing visible history remains, actions disable, and the popup reports provider unavailability.
- Malformed or cross-session ingress: rejected before registry mutation.
- Popup creation failure: taskbar remains usable and the next invocation retries.

## Verification

Automated gates cover protocol round trips, old-payload defaults, malformed UTF-16, capacity/eviction, deduplication, stale dismiss, clear-all, disconnect retention, and no-delegation source contracts. Headful gates capture light, dark, high contrast, empty, populated, overflow, keyboard, UIA dismiss, and Explorer-absent composition at 175% DPI. Full locked/offline tests, Clippy, release builds, standalone NSIS, combined NSIS, hashes, strict OpenSpec validation, and a unique evidence index are required.

## Rollback

Rollback is a source revert. The history is in-memory and bounded; there is no registry, database, settings-schema, installer, or migration rollback. Existing icon snapshots remain wire-compatible because new fields are defaulted.

## Scope boundaries

This change does not implement Windows toast database ingestion, notification settings, Focus Assist, Do Not Disturb scheduling, actionable toast buttons, calendar event mutation, or undocumented shell APIs. Those require separate owned provider designs.
