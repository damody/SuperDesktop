## ADDED Requirements

### Requirement: Compatibility identity is Explorer-exclusive
The notification host SHALL create the supported `Shell_NotifyIcon` compatibility identity only after controlled Shell ownership and MUST NOT compete with Explorer in preview mode.

#### Scenario: Preview with Explorer present
- **WHEN** SuperDesktop runs without committed Shell ownership
- **THEN** no compatibility class/window is created and Explorer notification behavior is unchanged

#### Scenario: Explorer-free admission
- **WHEN** lifecycle ownership, session identity and notification-host health are valid
- **THEN** one compatibility identity is created in the owned interactive session

### Requirement: Native notification input is bounded and owned
The compatibility boundary SHALL validate supported native layouts and SHALL copy identity, tooltip, state, version and icon pixels before registry mutation; raw pointers and handles MUST NOT cross into GPUI.

#### Scenario: Supported add and modify
- **WHEN** a live same-session client submits a supported add or newer modify
- **THEN** the host publishes one generation-bound owned icon with copied pixels and tooltip

#### Scenario: Malformed or hostile input
- **WHEN** size, version, window, session, string, icon dimensions or identity are invalid
- **THEN** the request is rejected without registry mutation, callback delivery or resource growth

### Requirement: Lifecycle operations map deterministically
The host SHALL map add, modify, delete, set-focus and version negotiation to monotonic registry operations and SHALL reject stale or duplicate mutation.

#### Scenario: Full icon lifecycle
- **WHEN** a controlled client adds, modifies, focuses, versions and deletes an icon
- **THEN** each accepted mutation has one terminal generation and the deleted icon is absent from the next snapshot

#### Scenario: Stale mutation arrives
- **WHEN** a mutation belongs to an older icon or host generation
- **THEN** visible state and callback route remain unchanged

### Requirement: User callbacks return only to the validated owner
Pointer, context, hover and focus events SHALL be delivered only after process, session and callback HWND identity are revalidated and no unwind may cross the native callback boundary.

#### Scenario: User activates an icon
- **WHEN** SuperDesktop emits an activate or context event for a live registered icon
- **THEN** the host posts the negotiated callback message once to that icon's validated callback window

#### Scenario: Client exits or HWND is reused
- **WHEN** the owner is dead or the callback identity no longer matches
- **THEN** the event fails closed and the stale icon is removed without messaging another process

### Requirement: Restart and overflow recover authoritatively
Client/icon/event queues SHALL be bounded, protected events SHALL not be silently dropped, and host restart or overflow SHALL clear stale state and request documented re-registration.

#### Scenario: Modify storm overflows
- **WHEN** client modifications exceed queue capacity
- **THEN** modifications coalesce, overflow is recorded and one authoritative reconciliation is scheduled

#### Scenario: Host restarts
- **WHEN** the compatibility host loses and reacquires ownership
- **THEN** old icons remain cleared, `TaskbarCreated` recovery is emitted and only newly registered clients appear

### Requirement: Owned notification UI remains accessible
SuperDesktop SHALL render compatible icons and overflow with stable ordering, copied pixels, tooltip, pointer, keyboard and UIA actions while independently reporting provider unavailability.

#### Scenario: Visible and overflow icons
- **WHEN** controlled clients exceed visible capacity
- **THEN** stable icons remain in the taskbar and the remainder appear in one owned accessible overflow surface

#### Scenario: Provider unavailable
- **WHEN** the notification compatibility host is unavailable
- **THEN** no fake icon/action is shown and Start, desktop and other system status providers remain usable

### Requirement: Completion is auditable and packaged
The change SHALL provide unique task-linked automated, headful, isolation, resource, lifecycle and installer evidence for the reference Windows 11 profile.

#### Scenario: Controlled legacy client gate
- **WHEN** Explorer is absent during the measured interval
- **THEN** a controlled ordinary client completes add, modify, callback and delete through SuperDesktop and the evidence records process/HWND ownership and binary hashes

#### Scenario: Required evidence is missing
- **WHEN** non-interference, malformed input, callback, recovery, accessibility or packaging proof is incomplete
- **THEN** the corresponding blocking gate remains failed
