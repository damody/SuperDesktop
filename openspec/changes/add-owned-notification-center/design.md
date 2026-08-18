## Context

The notification-area host owns registered tray icons and the taskbar owns an Explorer-free calendar popup. The documented NotifyIcon `NIF_INFO` payload is currently admitted at the native boundary but its title/body fields are not copied into the owned protocol, so no notification history can be rendered. The approved source design is `docs/superpowers/specs/2026-08-19-owned-notification-center-design.md`.

## Goals / Non-Goals

**Goals:**

- Copy documented notification balloon fields into owned memory before the native callback returns.
- Add backward-compatible, bounded, generation-aware notification history and typed dismiss/clear operations.
- Render a Windows 11-style notification center combined with the owned calendar.
- Preserve localization, UIA, keyboard, high contrast, resource bounds, authoritative reconciliation, and Explorer-free operation.
- Build and hash both NSIS packages while leaving this change unarchived.

**Non-Goals:**

- Windows toast database/history ingestion, actionable toast buttons, Focus Assist, Do Not Disturb schedules, notification Settings, or calendar-event mutation.
- Undocumented shell interfaces, `ShellExperienceHost`, Explorer UI, `SystemSettings`, or URI delegation.
- Persistent notification storage or a settings-schema migration.

## Decisions

### Additive protocol evolution

`OwnedNotification` carries stable identity, owning icon identity, application label, title, body, severity, admitted time, and generation. `OwnedNotifyIcon.notification` and `NotificationSnapshot.notifications` use serde defaults, preserving older JSON fixtures and clients. Dismiss and clear are explicit `NotificationMutation` variants; UI code never mutates history optimistically.

### Documented native fields only

The compatibility decoder copies `szInfo`, `szInfoTitle`, `dwInfoFlags`, timeout/version, and `NIF_REALTIME` only when `cbSize` proves those fields are present. No pointer or borrowed native buffer crosses the callback boundary. Empty title/body is not a notification, malformed frames fail closed, and live process/session/window validation remains mandatory.

### Bounded host-authoritative history

The host retains at most 100 records. Repeated records with the same owner, generation, title, and body deduplicate; when full, the oldest record is evicted before insertion. Stale dismiss/clear generations have no effect. Client disconnect removes live icons but retains admitted history until user dismissal or eviction.

### Combined notification and calendar popup

The existing Calendar flyout becomes a combined notification-center/calendar surface. Notifications own the upper scrollable region and calendar remains below it. This matches Windows 11 interaction while retaining one owned popup and one taskbar focus-return path. The empty state is truthful and contains no fake Settings link.

### Evidence correction policy

- **A — task refinement:** task order, helper extraction, fixture values, commands, or evidence paths may change without changing scope, public contracts, gates, or required evidence.
- **B — design/spec correction:** an unreasonable in-scope protocol or geometry assumption requires design/spec/tasks updates, reopening affected leaves, and stale-evidence replacement lineage.
- **C — material change:** undocumented APIs, persistence, new provider authority, new external writes, weaker gates, broader platform claims, or destructive actions require user approval.

Blocking gates cannot be silently weakened.

## Blocking Gates

- `G-NOTIFICATION-PROTOCOL`: additive round trip, old payload, validation, native copy, malformed/stale cases.
- `G-NOTIFICATION-HISTORY`: capacity, deduplication, eviction, disconnect, dismiss, clear, and reconciliation evidence.
- `G-NOTIFICATION-CENTER-UI`: light/dark/high-contrast, empty/populated/overflow, calendar reachability, and 175% geometry evidence.
- `G-NOTIFICATION-A11Y`: role/name/value, keyboard, Escape/focus restore, UIA dismiss, and clear-all evidence.
- `G-SHELL-NONINTERFERENCE`: no forbidden process/API/source path and Explorer-absent headful evidence.
- `G-TRACE`: detailed task validation, strict OpenSpec validation, and one unique record per leaf.
- `G-PACKAGE`: release binary and both NSIS installers built without launch and hashed.

## Risks / Trade-offs

- **[NotifyIcon balloon data is not complete Windows toast history]** → Label the capability as owned NotifyIcon notification history and render only admitted records.
- **[Protocol drift breaks older clients]** → Default all additive fields and keep old fixtures in the round-trip matrix.
- **[Notification storms consume memory]** → Enforce 100 records, existing text limits, deduplication, and oldest-first eviction.
- **[Dismiss races a newer snapshot]** → Require expected generation and reconcile only from authoritative host responses.
- **[Small work areas clip the calendar]** → Clamp popup height and give only the notification list vertical scrolling.
- **[High contrast loses selected/dismiss states]** → Use explicit focus and card borders in addition to color.

## Migration Plan

1. Land protocol DTO/defaults and compatibility tests.
2. Copy native balloon fields and translate them into owned records.
3. Add bounded host history and mutation tests.
4. Reconcile snapshots in the app and render the combined surface.
5. Run automated, headful, Explorer-free, traceability, release, and packaging gates.

Rollback is a source revert. History is in memory, so there is no registry, file, database, or installer migration to undo.

## Open Questions

None.
