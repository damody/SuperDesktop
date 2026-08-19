## ADDED Requirements

### Requirement: System status exposes bounded authoritative Wi-Fi state
The system-status snapshot SHALL add Wi-Fi availability containing at most 64 deterministically ordered current networks with opaque interface identity, SSID, optional saved profile, signal quality, security, connected, and connectable state.

#### Scenario: WLAN interfaces and networks are available
- **WHEN** one or more WLAN interfaces return available networks
- **THEN** the snapshot deduplicates equivalent SSIDs connected-first then saved/strongest, bounds every field/collection, and validates successfully

#### Scenario: No WLAN adapter
- **WHEN** WLAN enumeration returns no interfaces
- **THEN** Wi-Fi is NotPresent while the existing Network List Manager connectivity summary remains truthful

#### Scenario: WLAN service or list fails
- **WHEN** the WLAN service, interface list, or every available-network list fails
- **THEN** Wi-Fi is Unavailable with a bounded sanitized reason and the host remains healthy

#### Scenario: Malformed native lengths or oversized results
- **WHEN** native counts, SSID lengths, profile strings, or result collections exceed their bounds
- **THEN** the adapter rejects or truncates at the documented safe boundary without out-of-bounds access or credential disclosure

### Requirement: WLAN commands are typed, identity-bound, and eventually reconciled
The host SHALL support Refresh Wi-Fi, Connect exact saved profile, and Disconnect exact interface commands and SHALL return Accepted only after the WLAN service admits the request; final UI state MUST come from a later authoritative snapshot.

#### Scenario: Refresh current interfaces
- **WHEN** Refresh Wi-Fi targets the current host generation before its deadline
- **THEN** every current interface is scanned, at least one admitted scan yields Accepted, and Network reconciliation is scheduled

#### Scenario: Connect saved profile
- **WHEN** Connect references an exact current interface/profile pair advertised as saved and connectable
- **THEN** WlanConnect is submitted without credentials/profile XML and the terminal is Accepted rather than falsely Observed

#### Scenario: Unsaved, stale, or mismatched connect
- **WHEN** Connect references an unsaved profile, stale interface, mismatched profile, invalid text, generation, or deadline
- **THEN** the command fails closed without invoking WlanConnect

#### Scenario: Disconnect current interface
- **WHEN** Disconnect references an exact current connected interface
- **THEN** WlanDisconnect is submitted, Accepted is returned, and later snapshots report the authoritative result

#### Scenario: Provider restart after accepted command
- **WHEN** the host generation changes before final connectivity is observed
- **THEN** stale terminals/snapshots are rejected and the restarted host publishes a new authoritative WLAN snapshot

### Requirement: Wi-Fi flyout provides complete truthful network workflow
The owned NetworkPower flyout SHALL display current Wi-Fi state, bounded available networks, refresh, saved-profile connect, disconnect, security/signal state, and power summary in a taskbar-clamped scrollable panel.

#### Scenario: Connected secure network
- **WHEN** the snapshot marks a secure network connected
- **THEN** it is selected first with connected/secure/Internet detail, information semantics, and an actionable Disconnect control

#### Scenario: Saved connectable network
- **WHEN** a disconnected network has a saved profile and is connectable
- **THEN** its expanded row exposes Connect and pointer/Enter/Space emit the exact typed action once

#### Scenario: Unsaved secured network
- **WHEN** a secured network has no saved profile
- **THEN** the row states that a password is required and credential entry is unavailable, and no Connect action is exposed

#### Scenario: Unsecured discovery network
- **WHEN** an unsecured network has no saved profile
- **THEN** the row is shown truthfully but remains non-actionable in this change because discovery-profile mutation is outside scope

#### Scenario: Refresh and eventual update
- **WHEN** the user activates Refresh networks
- **THEN** RefreshWifi is emitted and subsequent reconciled snapshots update the open list without reopening the flyout

#### Scenario: Provider unavailable or no adapter
- **WHEN** Wi-Fi is Unavailable or NotPresent
- **THEN** the panel explains the state, exposes no connect/disconnect actions, retains NLM/power summaries, and remains dismissible

### Requirement: Wi-Fi flyout preserves owned accessible themed behavior
The Wi-Fi panel SHALL remain SuperDesktop-owned, localized, keyboard/UIA accessible, light/dark/high-contrast visible, and clamped above one-to-three taskbar rows without Explorer, Quick Settings, Windows Settings, password, or profile XML delegation.

#### Scenario: Theme, DPI, and long-list matrix
- **WHEN** 0, 1, or 64 networks render at supported DPI/theme/taskbar-row combinations
- **THEN** geometry remains within the monitor, the list scrolls, focus/action states are distinguishable, and bottom controls remain reachable

#### Scenario: Unsupported quick controls
- **WHEN** Airplane mode or Mobile hotspot is displayed
- **THEN** each is explicitly unavailable/disabled and emits no mutation

#### Scenario: Escape and focus loss
- **WHEN** the flyout receives Escape or loses activation
- **THEN** the owned popup closes and returns to the existing taskbar lifecycle without delegated shell UI

#### Scenario: Evidence privacy
- **WHEN** live WLAN validation records reports, logs, or screenshots
- **THEN** committed machine-specific SSID, profile, and interface identifiers are redacted or hashed and no credentials/profile XML are recorded
