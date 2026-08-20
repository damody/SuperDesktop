## ADDED Requirements

### Requirement: Standalone Windows key toggles owned Start

When SuperDesktop owns the Windows shell, it SHALL consume a standalone left or right Windows logo key gesture and SHALL toggle the owned Start menu exactly once when the matching key is released.

#### Scenario: Left Windows key opens Start
- **WHEN** owned Start is closed and the user presses and releases the left Windows key without pressing another key
- **THEN** SuperDesktop opens the owned Start menu on release and does not delegate the gesture to Explorer

#### Scenario: Right Windows key opens Start
- **WHEN** owned Start is closed and the user presses and releases the right Windows key without pressing another key
- **THEN** SuperDesktop opens the same owned Start menu on release

#### Scenario: Windows key closes Start
- **WHEN** owned Start is open and the user completes a standalone Windows-key gesture
- **THEN** SuperDesktop closes that Start menu without opening a replacement surface

#### Scenario: Repeated keydown emits one toggle
- **WHEN** the held Windows key produces repeated keydown messages before its matching release
- **THEN** SuperDesktop emits exactly one Start toggle for the completed gesture

### Requirement: Windows-key chords never produce a trailing Start toggle

SuperDesktop SHALL cancel standalone eligibility when any other key is pressed while a Windows key is held, independently of whether that chord is implemented by SuperDesktop.

#### Scenario: Supported chord remains exclusive
- **WHEN** the user presses a supported chord such as Win+E and then releases the Windows key
- **THEN** SuperDesktop runs the chord action and does not open or close Start on the trailing Windows-key release

#### Scenario: Unsupported chord passes without trailing Start
- **WHEN** the user presses an unsupported key while holding Windows and later releases Windows
- **THEN** SuperDesktop does not emit a Start toggle and leaves the unsupported chord available to downstream Windows processing

#### Scenario: Dual Windows keys are ambiguous
- **WHEN** the second Windows key is pressed while a standalone Windows-key candidate is held
- **THEN** SuperDesktop cancels the candidate and emits no Start toggle from that dual-key sequence

#### Scenario: Mismatched release preserves the candidate
- **WHEN** a key-up message unrelated to the tracked Windows key is observed without a corresponding keydown
- **THEN** SuperDesktop does not emit a Start toggle and retains the candidate until a decisive event occurs

### Requirement: Toggle reuses the owned Start lifecycle

SuperDesktop SHALL dispatch the standalone gesture through the taskbar Start callback so keyboard and pointer activation share window ownership, monitor placement, alignment settings, focus behavior, dismissal, and panic containment.

#### Scenario: Runtime dispatch uses the shared callback
- **WHEN** the hook queue yields a Start toggle action
- **THEN** the GPUI runtime invokes `callbacks.start` on the UI context and records a shell-hotkey trace

#### Scenario: Taskbar callback is temporarily unavailable
- **WHEN** a Start toggle reaches a runtime with no live taskbar Start callback
- **THEN** SuperDesktop writes a contextual error to the console and continues running

#### Scenario: Preview mode leaves Windows in control
- **WHEN** SuperDesktop runs without owned-shell hotkeys
- **THEN** it does not intercept the Windows key and the host Windows shell retains its normal behavior

### Requirement: Headful verification restores the host shell

The Windows UTIT verification SHALL prove a real standalone Win gesture opens and closes owned Start and SHALL restore the prior shell registry and Explorer process state after success or failure.

#### Scenario: Owned Start opens and closes in UTIT
- **WHEN** the headful harness injects one standalone Win gesture and then a second gesture into a running owned-shell build
- **THEN** evidence shows the Start window appearing after the first gesture and disappearing after the second

#### Scenario: Harness exits through failure cleanup
- **WHEN** any launch, input, observation, or assertion step fails
- **THEN** the harness executes bounded cleanup and records that the original Winlogon Shell and Explorer availability were restored
