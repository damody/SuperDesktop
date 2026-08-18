## Why

When Explorer is absent, ordinary Windows applications that call `Shell_NotifyIcon` have no compatible taskbar notification host, so their icons and callbacks disappear even though SuperDesktop can already render its own generation-bound notification registry. This change closes that Explorer dependency without loading third-party code or raw icon handles into the GPUI process.

## What Changes

- Add an Explorer-exclusive compatibility window/class in `notification-area-host`, admitted only after controlled Shell ownership.
- Validate supported `NOTIFYICONDATA` layouts and copy tooltip, identity, visibility, callback route and HICON pixels into bounded owned DTOs.
- Map add, modify, delete, set-focus and version negotiation to the existing notification registry with stale-generation rejection.
- Deliver pointer/context/focus callbacks only to the validated owning process/session/window.
- Emit taskbar-created recovery after host restart or takeover and reconcile re-registering clients.
- Render supported client icons in SuperDesktop's visible/overflow notification area; unsupported private toolbar protocols remain truthfully unavailable.

## Capabilities

### New Capabilities

- `shell-notifyicon-compatibility`: Explorer-free admission, bounded native structure ingestion, notification registry mapping, callback delivery, recovery and audit requirements for supported `Shell_NotifyIcon` clients.

### Modified Capabilities

None.

## Impact

Affected components are `notification-area-host`, `platform-win`, `shell-provider-protocol`, `superdesktop-app`, `taskbar-ui`, lifecycle admission, NSIS packaging and verification scripts. The compatibility identity is exclusive with Explorer and therefore disabled in preview mode. No undocumented Explorer toolbar protocol or private notification-center history is claimed.
