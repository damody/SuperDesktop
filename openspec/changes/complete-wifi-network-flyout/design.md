## Context

The NetworkPower view consumes `SystemStatusSnapshot.network`, which currently contains only Network List Manager connectivity/name. The command protocol covers input and audio only. Native WLAN APIs and `Win32_NetworkManagement_WiFi`/`Ndis` features are absent. Existing flyout geometry is 228 DIP and the prior system-flyout change explicitly excluded Wi-Fi controls, so this requires a new cross-layer capability rather than a visual patch.

## Goals / Non-Goals

**Goals:**

- Enumerate bounded real WLAN interfaces and available networks with connected, signal, security, profile, and connectability state.
- Refresh WLAN scans, connect exact saved profiles, and disconnect exact interfaces through the provider host.
- Render a localized, scrollable Windows-style owned Wi-Fi panel with truthful action gating.
- Preserve generation safety, provider failure behavior, Explorer-free ownership, themes, accessibility, and taskbar-relative geometry.

**Non-Goals:**

- Collect/store credentials; create, edit, delete, or export profiles; support hidden/EAP credential workflows.
- Toggle Wi-Fi radio, airplane mode, mobile hotspot, VPN, or Ethernet; open Quick Settings or Windows Settings.

## Decisions

### Additive protocol with accepted WLAN commands

Add `wifi: StatusAvailability<WifiStatus>` to `NetworkStatus` with serde default for old snapshots. `WifiStatus` holds `enabled`, bounded interfaces/networks, and network records keyed by opaque interface identity plus SSID/profile. Add Refresh, Connect saved profile, and Disconnect commands. Introduce `SystemStatusTerminalKind::Accepted`: `WlanConnect`, `WlanDisconnect`, and `WlanScan` report service admission, not final connectivity. The existing authoritative snapshot cadence reports eventual state. This avoids a false Observed terminal and UI-thread waits.

### Scoped native WLAN adapter

Enable `Win32_NetworkManagement_WiFi` and `Ndis`. A private RAII client closes `WlanOpenHandle`; list wrappers free `WlanEnumInterfaces` and `WlanGetAvailableNetworkList` memory. Counts are capped before flexible-array slicing. SSID length is capped at 32 bytes; profile/interface/text fields use protocol text bounds. Equivalent SSIDs across adapters are deduplicated connected-first, then saved-profile, then strongest signal. Interface identity is an opaque canonical GUID string resolved only against a fresh enumeration.

Refresh scans every current interface and accepts individual scan errors only if at least one interface admitted a scan. Connect requires a nonempty profile name that exactly matches a current available network with `HAS_PROFILE`; no arbitrary XML/credentials cross the boundary. Disconnect requires an exact current interface. Provider errors are sanitized and do not contain profile XML or keys.

### Host and app reconciliation

`system-status-host` snapshots WLAN alongside NLM status and routes typed commands. Accepted WLAN commands enqueue a Network provider event so the next snapshot is authoritative. Protocol minor advances while major remains compatible. `superdesktop-app` maps UI actions to typed commands, uses the existing bounded request, applies Accepted terminals, requests an immediate snapshot, and continues periodic updates to open flyouts.

### Complete owned panel

NetworkPower preferred height becomes 640 DIP, clamped by existing geometry. `SystemFlyoutView` derives Wi-Fi state from the snapshot and renders:

- connected summary with Disconnect and information status;
- scrollable available networks sorted by provider order;
- signal/security/connected/saved/unavailable descriptions;
- Connect only when a saved profile is available and network is connectable;
- a password-required status for unsaved secured networks;
- Refresh networks;
- Wi-Fi state plus disabled Airplane mode/Mobile hotspot tiles;
- existing power summary.

Rows and controls have stable IDs, localized labels, Button/Status/Group semantics, pointer/keyboard parity, and existing theme tokens. No fake action is attached to unsupported tiles.

## Risks / Trade-offs

- [Flexible-array/native memory] → Cap counts before pointer slicing, validate lengths, RAII-free every output, and test malformed decoders separately.
- [Connect accepted but later fails] → Use Accepted terminal and only change connected UI from later snapshots.
- [SSID is not UTF-8] → Decode lossily for display but keep profile/interface identity for commands; never accept display SSID as authority.
- [Multiple adapters duplicate SSIDs] → Deterministically choose connected/saved/strongest and keep exact interface identity.
- [Credential exposure] → No password/profile XML API is used; logs/evidence redact SSID/profile/interface values.
- [No WLAN adapter/service] → Preserve NLM summary and render Wi-Fi NotPresent/Unavailable without actionable rows.
- [Large network lists] → Cap at 64 and use a bounded scroll viewport.

## Migration Plan

1. Land additive protocol/types/tests and Windows feature flags.
2. Land read-only WLAN enumeration plus no-adapter/live-read evidence.
3. Land host commands and app reconciliation.
4. Land UI/geometry and headful fixtures.
5. Run full gates; connect/disconnect mutation is executed only against an explicitly controlled saved-profile fixture. Without that fixture it is evidence-backed not-applicable while command validation/admission tests remain mandatory.

Rollback reverts protocol consumers and provider/UI together. No persisted schema or credential migration exists.

## Planning adjustments

- **A — task refinement:** Commands, task order, and evidence paths may be refined without changing requirements or gates.
- **B — design/spec correction:** Platform signature/behavior corrections within this approved WLAN scope require design/spec/tasks updates and invalidate dependent evidence.
- **C — material change:** Password collection, profile mutation/XML, EAP/hidden-network support, radio/hotspot/airplane controls, delegation, permissions, or weakened gates require user approval.

## Open Questions

None. Live mutation is conditional on a controlled fixture; all read-only and UI gates are unconditional.
