## Why

The Wi-Fi taskbar button opens a NetworkPower flyout that contains only a generic connectivity card and power text. SuperDesktop lacks the WLAN snapshot and command contracts required to enumerate networks, refresh, connect saved profiles, or disconnect, so the primary Wi-Fi workflow is functionally incomplete.

## What Changes

- Extend the additive system-status protocol with bounded Wi-Fi interfaces/networks and accepted WLAN command terminals.
- Add a native Windows WLAN provider for scan/list, refresh, saved-profile connect, and disconnect with scoped handles and bounded buffers.
- Route WLAN commands through the out-of-process system-status host and refresh authoritative snapshots.
- Replace the minimal network card with a scrollable, localized Wi-Fi network panel containing current connection, available networks, signal/security/profile state, refresh, saved-profile connect, disconnect, and truthful unavailable controls.
- Increase/clamp network flyout geometry and add protocol, platform, host/client, UI, live-WLAN, theme, accessibility, and headful evidence.

## Capabilities

### New Capabilities

- `owned-wifi-network-flyout`: Defines bounded WLAN discovery and commands, complete owned Wi-Fi panel behavior, truthful unsupported states, and Explorer-free evidence.

### Modified Capabilities

None. Previous system-flyout capabilities explicitly excluded Wi-Fi mutation and remain historical; no archived base capability currently owns this new contract.

## Impact

Affected code spans `shell-provider-protocol`, the workspace Windows feature set, `platform-win`, `system-status-host`, `superdesktop-app`, `taskbar-ui`, fixtures, scripts, and evidence. JSON snapshots remain additive for readers using defaults, but the host/client protocol minor version advances. No credentials, profile XML, settings schema, extension ABI, Explorer process, or Windows Settings dependency is introduced.
