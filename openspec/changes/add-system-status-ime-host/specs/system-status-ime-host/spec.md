## ADDED Requirements

### Requirement: System status uses real Windows providers
SuperDesktop SHALL obtain network, volume, mute, power, clock/calendar and input-language state from documented Windows providers. Product code MUST NOT substitute fixed values when a provider is missing or failed.

#### Scenario: Live status snapshot
- **WHEN** the status host completes its initial observation
- **THEN** it publishes a generation-bound snapshot whose available values match the current Windows state

#### Scenario: One provider fails
- **WHEN** the network provider fails while audio, power, clock and input remain available
- **THEN** only network becomes unavailable and every other status remains truthful and actionable

#### Scenario: Desktop has no battery
- **WHEN** Windows reports no system battery
- **THEN** power reports a valid not-present state rather than a fabricated percentage or generic provider failure

### Requirement: Native status integration is process-isolated
System status native adapters SHALL run outside the GPUI process in `system-status-host`, SHALL copy native data to owned bounded DTOs and SHALL prevent panic/unwind or raw handles from crossing process/FFI boundaries.

#### Scenario: Host process crashes
- **WHEN** `system-status-host` exits unexpectedly
- **THEN** desktop/taskbar remain alive, all host-owned states become unavailable and bounded restart waits for a new full snapshot

#### Scenario: Malformed or oversized frame arrives
- **WHEN** the host/client receives an invalid version, duplicate correlation, malformed field or oversized frame
- **THEN** it rejects the frame without changing visible state or exhausting the queue

#### Scenario: Native callback panics during shutdown
- **WHEN** a fixture injects panic into a registered native callback while shutdown races
- **THEN** no unwind crosses the ABI and each resource is released at most once

### Requirement: Input profiles are real and switchable
The status host SHALL enumerate bounded installed input profiles with stable identity, language tag and display name, SHALL publish the observed active profile and SHALL expose typed activation commands.

#### Scenario: Input flyout opens
- **WHEN** the user invokes the taskbar input-language control
- **THEN** an owned GPUI flyout lists the observed profiles, marks the active profile and focuses an actionable entry

#### Scenario: Input profile activation succeeds
- **WHEN** the user chooses a different available profile
- **THEN** success is emitted only after the host observes that exact profile active and the taskbar updates to it

#### Scenario: Activation times out or is stale
- **WHEN** activation is not observed before its deadline or a result belongs to an older generation
- **THEN** the previous observed profile remains visible and one recoverable terminal failure is exposed

#### Scenario: Start has active IME composition
- **WHEN** a taskbar profile switch occurs while Start owns an IME composition
- **THEN** Start does not dispatch uncommitted text or lose its focus/composition state

### Requirement: Audio state and commands are observed
The host SHALL publish the default render endpoint volume and mute state and SHALL apply bounded set-volume and mute commands through documented Core Audio APIs.

#### Scenario: External volume change
- **WHEN** another application changes default endpoint volume or mute
- **THEN** the next event/reconciliation snapshot updates the taskbar without reordering other status controls

#### Scenario: Owned volume flyout changes volume
- **WHEN** the user moves the GPUI volume control or toggles mute
- **THEN** the command completes only after a snapshot confirms the requested state

#### Scenario: Audio endpoint disappears
- **WHEN** the default endpoint is removed or Core Audio becomes unavailable
- **THEN** audio becomes truthfully unavailable and the flyout disables mutation without affecting other providers

### Requirement: Owned system flyouts remain independent and accessible
SuperDesktop SHALL render owned input, volume, network/power and clock/calendar flyouts, SHALL allow only one open system flyout and SHALL support pointer, keyboard, UIA, Escape and focus return.

#### Scenario: Toggle one flyout
- **WHEN** a status control is invoked twice
- **THEN** the first invocation opens one owned flyout and the second closes exactly that flyout

#### Scenario: Switch between flyouts
- **WHEN** one flyout is open and a different status control is invoked
- **THEN** the prior flyout closes before the new one opens and no stale window handle remains

#### Scenario: Provider is unavailable
- **WHEN** a flyout's provider is unavailable
- **THEN** the flyout shows a localized accessible unavailable state and exposes no fake action

### Requirement: Provider state is bounded and recoverable
Status frames, profiles, strings, commands, callbacks and pending events SHALL have explicit capacity/deadline bounds, monotonic generations and full-snapshot recovery after overflow or restart.

#### Scenario: Event storm overflows the queue
- **WHEN** audio/network/input callbacks exceed queue capacity
- **THEN** protected terminal events remain, an overflow state is emitted and one authoritative full reconciliation is scheduled

#### Scenario: Stale snapshot follows restart
- **WHEN** a pre-restart snapshot arrives after a new host generation is accepted
- **THEN** the stale snapshot is rejected and cannot restore old status

### Requirement: Real status completion is auditable
The change SHALL provide automated, headful, resource, accessibility and packaging evidence with unique task links.

#### Scenario: Hard-coded product status is detected
- **WHEN** the source guard finds fixed online/volume/mute/input-language values in product status composition
- **THEN** `G-STATUS-REAL` fails and the change cannot complete

#### Scenario: Headful or trace evidence is missing
- **WHEN** real input/audio/status evidence or unique task linkage is incomplete
- **THEN** the corresponding gate remains failed even if unit tests pass
