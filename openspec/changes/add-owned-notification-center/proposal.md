## Why

SuperDesktop owns the taskbar, NotifyIcon ingress, and calendar popup, but it discards documented `NIF_INFO` notification content and has no notification history surface. This leaves a visible and functional Explorer dependency gap when SuperDesktop and SuperExplorer are the only shell applications.

## What Changes

- Extend the versioned notification protocol with backward-compatible owned notification records, snapshot history, dismiss, and clear operations.
- Copy and validate documented NotifyIcon balloon title/body/severity fields before `WM_COPYDATA` returns.
- Retain a deduplicated, oldest-first-evicted, 100-record notification history in `notification-area-host`.
- Combine a Windows 11-style notification list with the existing owned calendar flyout.
- Add localized empty, populated, overflow, unavailable, keyboard, UIA, dismiss, clear-all, and high-contrast behavior.
- Preserve Explorer-free operation, truthful provider limitations, bounded resources, and existing icon clients.
- Produce deterministic, headful, traceability, release, and standalone/combined NSIS evidence without archiving the change.

## Capabilities

### New Capabilities

- `owned-notification-center`: Defines owned notification admission, bounded history, actions, Windows 11 presentation, accessibility, localization, and Explorer-free behavior.

### Modified Capabilities

None.

## Impact

Changes `shell-provider-protocol`, `platform-win` NotifyIcon decoding, `notification-area-host`, the SuperDesktop notification client/reconciliation path, `taskbar-ui` system flyouts, capture scripts, evidence, and packaged binaries. Wire changes are additive with serde defaults; no registry, database, installer schema, undocumented Windows API, Explorer UI, or SuperExplorer source change is introduced.
