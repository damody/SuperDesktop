## Why

The taskbar currently displays fixed `online`, volume `40`, unmuted and `zh-TW` values, so its right-side system region is not truthful when Explorer is absent. SuperDesktop needs an isolated provider that reads and changes real Windows status while keeping native COM/TSF handles outside GPUI.

## What Changes

- Add versioned owned DTOs for network, Core Audio volume/mute, power, clock/calendar and installed/active input profiles.
- Add an isolated `system-status-host` process with bounded handshake, snapshots, commands, generations, health and restart behavior.
- Replace every fixed taskbar status value with real provider snapshots or a truthful unavailable state.
- Add owned GPUI input-language, volume, network, power and calendar flyouts.
- Make input-profile and volume commands complete only after an observed confirming snapshot.

## Capabilities

### New Capabilities

- `system-status-ime-host`: Defines isolated real Windows status acquisition, input-profile switching, status commands, owned flyouts, failure handling and evidence.

### Modified Capabilities

None.

## Impact

- Adds a workspace crate/binary and extends `shell-provider-protocol` DTOs.
- Adds documented Windows adapters in `platform-win` for TSF/keyboard layouts, Core Audio, network and power state.
- Changes `taskbar-ui` status models/views and `superdesktop-app` broker composition/reconciliation.
- Updates installer manifests to package `system-status-host.exe`.
- Does not implement legacy `Shell_NotifyIcon` compatibility or terminate Explorer; those remain subsequent changes.
