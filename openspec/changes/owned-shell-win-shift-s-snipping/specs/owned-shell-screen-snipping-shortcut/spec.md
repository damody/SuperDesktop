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
SuperDesktop SHALL activate the Windows-registered built-in image-snipping overlay using only the fixed `ms-screenclip:` protocol and SHALL NOT resolve or launch Explorer, a path-discovered executable, or a third-party capture program.

#### Scenario: Protocol activation accepted
- **WHEN** the queued screen-snipping action is dispatched and Windows accepts the registered protocol
- **THEN** SuperDesktop records requested and accepted trace events and the built-in capture overlay becomes observable

#### Scenario: Protocol activation rejected
- **WHEN** Windows rejects or cannot resolve the fixed screen-clipping protocol
- **THEN** SuperDesktop prints a scoped console error, remains alive, and performs no fallback launch

### Requirement: Explorer-compatible delegation
SuperDesktop SHALL install no shell shortcut hook in Explorer-compatible preview mode, leaving `Win+Shift+S` to Windows without duplicate activation.

#### Scenario: Explorer remains the shell
- **WHEN** SuperDesktop runs without owned-shell mode
- **THEN** no SuperDesktop screen-snipping action is enqueued for the chord

### Requirement: Physical shortcut release evidence
The release candidate SHALL pass a headful Explorer-free test that sends the physical chord, observes the built-in overlay and both admission traces, dismisses the overlay with Escape, and verifies SuperDesktop survival without storing screen-content screenshots.

#### Scenario: Two clean headful runs
- **WHEN** the focused test runs twice from clean owned-shell launches
- **THEN** both reports pass with matching candidate hash, real overlay/process observation, clean dismissal, and no panic/error signature
