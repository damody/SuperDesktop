## Why

The owned notification center currently contains only NotifyIcon balloon records admitted through SuperDesktop's compatibility host. Modern Windows Toast additions/removals are invisible, and dismiss/clear affects only local history, so the center does not support the Windows notification events shown in the supplied reference.

## What Changes

- Add a documented WinRT `UserNotificationListener` event source with access-state reporting and bounded authoritative reconciliation.
- Convert current Windows Toast app labels, creation times, and ToastGeneric text into the existing owned notification model.
- Merge Windows-origin and NotifyIcon notifications with stable identities, deduplication, ordering, capacity, and per-item failure isolation.
- Synchronize Windows-origin dismiss and clear-all operations back through `RemoveNotification` and `ClearNotifications` before local state changes.
- Publish additive Windows-event provider state and render truthful denied/unavailable/synchronizing status in the owned center.
- Add protocol, platform, host, client/UI, real-event, access, recovery, privacy, theme, accessibility, and Explorer-free evidence.

## Capabilities

### New Capabilities

- `windows-notification-events`: Defines documented Windows Toast event access, conversion, reconciliation, synchronized removal, provider state, and complete evidence.

### Modified Capabilities

None. The archived owned-notification-center capability explicitly excluded Windows Toast history; this change introduces the previously excluded capability without modifying an active base spec.

## Impact

Affected code spans Windows crate features, `shell-provider-protocol`, a new `platform-win` WinRT adapter, `notification-area-host`, `superdesktop-app`, `taskbar-ui`, headful fixtures, scripts, and evidence. Snapshot fields are additive. No private notification database, Explorer/ShellExperienceHost route, arbitrary app activation, Toast action simulation, permission mutation, or settings-schema migration is introduced.
