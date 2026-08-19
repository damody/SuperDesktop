# Complete Wi-Fi network flyout design

## Problem

The owned NetworkPower flyout currently renders only Network List Manager connectivity and power summaries. The system-status protocol has no WLAN interfaces, available networks, signal/security metadata, refresh, connect, or disconnect commands. The Wi-Fi taskbar button therefore cannot provide the Windows-style network workflow shown in the reference.

## Design

### Protocol and provider

Extend `NetworkStatus` with additive `wifi: StatusAvailability<WifiStatus>`. `WifiStatus` contains bounded adapter availability and at most 64 deduplicated `WifiNetwork` records: stable interface identity, SSID, optional saved profile name, signal quality, security, connected state, and connectability. Extend `SystemStatusCommand` with `RefreshWifi`, `ConnectWifi { interface_id, profile_name }`, and `DisconnectWifi { interface_id }`. Add an `Accepted` terminal state for WLAN commands accepted by the Windows service but not yet authoritatively observed; the normal snapshot cadence remains authoritative.

`platform-win` adds a scoped WLAN client that closes every handle and frees every list. It enumerates interfaces with `WlanEnumInterfaces`, requests scans, reads `WlanGetAvailableNetworkList`, bounds flexible-array counts before slicing, validates SSID/profile lengths, deduplicates equivalent SSIDs, and sorts connected-first then strongest signal. Connect is admitted only for an exact current interface and saved profile; unsaved secured networks are reported but cannot receive a fake connect action. Disconnect targets an exact current interface. WLAN failures become `Unavailable` reasons without losing the existing Network List Manager summary.

`system-status-host` snapshots WLAN state and executes typed commands. Refresh/connect/disconnect return `Accepted` after the WLAN service admits the request, schedule network reconciliation, and let subsequent snapshots publish the result. Invalid/stale identities fail closed. `superdesktop-app` maps new UI actions to protocol commands with a bounded timeout and refreshes the open flyout from the reconciler.

### Flyout UI

The NetworkPower flyout grows to a preferred 640 DIP height, still clamped above the owned taskbar. It contains:

1. a current-network card with name, Internet/security state, information indicator, and Disconnect when connected;
2. a vertically scrollable available-network list with SSID, signal strength, lock state, connected/saved/unavailable state, and Connect only for saved profiles;
3. a Refresh networks control;
4. a Wi-Fi status tile plus truthful disabled Airplane mode and Mobile hotspot tiles;
5. the existing power summary.

Rows and controls expose stable IDs, Button/Status semantics, localized labels, visible hover/pressed/focus/selected states, and keyboard activation. Unsaved secured networks say that a password is required and no credential UI is available; no operation is sent. Provider unavailable/not-present states remain explicit. Existing light/dark/high-contrast tokens and focus-loss/Escape dismissal remain authoritative.

## Alternatives

A visual-only list was rejected because it would present nonfunctional controls. Delegating to Explorer Quick Settings or `ms-settings:` was rejected because SuperDesktop owns the shell surface and existing architecture forbids that dependency. Implementing password collection and profile creation in this change was rejected because credential handling, EAP, enterprise policy, hidden networks, and secure persistence require a separate security-reviewed scope. Direct UI-thread WLAN calls were rejected in favor of the existing out-of-process system-status host.

## Failure handling and safety

All native list counts, SSID lengths, profile strings, and protocol collections are bounded. WLAN handles and returned memory use RAII. Connect/disconnect require an identity present in a fresh enumeration and never accept arbitrary profile XML or credentials. Accepted commands do not claim connection success; only later snapshots change connected state. Provider crashes/restarts follow the existing generation and stale-terminal rules. No password, key material, or profile XML crosses protocol or appears in logs/evidence.

## Verification

Protocol tests cover additive round-trip, bounds, duplicate/interface identity, invalid commands, and Accepted terminals. Platform tests cover decoding, dedupe/sort, flexible-array guards, no-adapter/unavailable handling, and read-only real WLAN enumeration; live connect/disconnect mutation uses a controlled saved-profile fixture only when explicitly admitted by the headful test environment, otherwise records evidence-backed not-applicable without weakening enumeration/UI gates. Host/client tests cover command routing, stale generations, refresh reconciliation, and restart safety. UI tests cover connected/disconnected/unavailable lists, saved/unsaved security states, scrolling, action gating, localization, theme, keyboard, and UIA. Headful evidence captures the real machine's current networks with identifiers redacted in committed reports, verifies Refresh, and uses the controlled fixture for mutation when available.

## Scope

This change adds scan/list, refresh, saved-profile connect, and disconnect. It does not collect passwords, create/edit/delete profiles, support hidden/EAP credential workflows, toggle radio/airplane mode/mobile hotspot, open Windows Settings, or change Ethernet/VPN management.
