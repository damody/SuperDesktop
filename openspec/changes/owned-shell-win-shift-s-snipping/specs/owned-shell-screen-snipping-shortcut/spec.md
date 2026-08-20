## ADDED Requirements

### Requirement: Native Win+Shift+S routing
SuperDesktop SHALL consume `Win+Shift+S` in owned-shell mode and enqueue exactly one screen-snipping action per physical press, while preserving `Win+S` search and passing unsupported Control/Alt variants onward.

#### Scenario: Initial owned-shell chord
- **WHEN** Windows and Shift are down and S receives its initial key-down in owned-shell mode
- **THEN** SuperDesktop consumes the event and enqueues one screen-snipping action

#### Scenario: Held key repeat and release
- **WHEN** S repeats before release or its matching key-up arrives
- **THEN** SuperDesktop consumes the event without enqueueing a second action and clears the active-key fence on release

#### Scenario: Search chord remains distinct
- **WHEN** Windows and S are pressed without Shift
- **THEN** SuperDesktop enqueues the existing search action instead of screen snipping

#### Scenario: Unsupported modifier combination
- **WHEN** Control or Alt is also down with Windows, Shift, and S
- **THEN** SuperDesktop does not enqueue or consume the chord through its shell reducer

### Requirement: Built-in Snipping Tool overlay activation
SuperDesktop SHALL activate the Windows-registered built-in image-snipping overlay through `IApplicationActivationManager::ActivateApplication` using only the fixed built-in AUMID `Microsoft.ScreenSketch_8wekyb3d8bbwe!App`, observed native-hotkey argument `ms-screenclip:///?source=HotKey`, and `AO_NONE`; it SHALL NOT use ShellExecute, `ActivateForProtocol`, discover a Snipping Tool executable path, register a capture callback, or launch a third-party capture program.

#### Scenario: Protocol activation accepted
- **WHEN** the queued screen-snipping action is dispatched and Windows accepts the registered protocol
- **THEN** SuperDesktop records requested and accepted trace events and the built-in capture overlay becomes observable

#### Scenario: Protocol activation rejected
- **WHEN** Windows rejects or cannot resolve the fixed screen-clipping protocol
- **THEN** SuperDesktop cleans up any request-owned Explorer broker, prints a scoped console error, remains alive, and performs no capture-program fallback launch

### Requirement: Bounded verified Explorer broker
When Explorer is absent, SuperDesktop SHALL use only the verified inbox Explorer as a temporary broker while the built-in overlay is visible and SHALL close only a broker owned by that request after the overlay disappears.

#### Scenario: Owned shell starts without Explorer
- **WHEN** `Win+Shift+S` is dispatched with no verified Explorer shell present
- **THEN** SuperDesktop launches the signed canonical inbox Explorer, waits for the built-in overlay, and keeps the broker only until the overlay is dismissed

#### Scenario: Overlay finishes or is cancelled
- **WHEN** `SnipOverlayRootWindow` disappears after capture or Escape
- **THEN** SuperDesktop validates current session and canonical Explorer identity, closes the request-owned broker, and records accepted only after cleanup succeeds

#### Scenario: Explorer pre-existed the request
- **WHEN** a verified Explorer shell was already present before dispatch
- **THEN** SuperDesktop uses Windows' existing native support and does not close that pre-existing Explorer as request-owned

### Requirement: Explorer-compatible delegation
SuperDesktop SHALL install no shell shortcut hook in Explorer-compatible preview mode, leaving `Win+Shift+S` to Windows without duplicate activation.

#### Scenario: Explorer remains the shell
- **WHEN** SuperDesktop runs without owned-shell mode
- **THEN** no SuperDesktop screen-snipping action is enqueued for the chord

### Requirement: Physical shortcut release evidence
The release candidate SHALL pass a headful owned-shell test that starts with Explorer absent, sends the physical chord, observes the temporary verified broker plus built-in overlay and both admission traces, dismisses the overlay with Escape, verifies Explorer is absent again, and verifies SuperDesktop survival without storing screen-content screenshots.

#### Scenario: Two clean headful runs
- **WHEN** the focused test runs twice from clean owned-shell launches
- **THEN** both reports pass with matching candidate hash, real overlay/process observation, clean dismissal, and no panic/error signature
