## ADDED Requirements

### Requirement: Physical Windows-key chord admission
SuperDesktop SHALL recognize owned-shell Windows-key chords from tracked physical left/right Windows and modifier state, enqueue at most one action per initial chord press, consume matching repeats/releases, and open Start on Windows-key release only when no chord consumed that gesture.

#### Scenario: Win+D initial press
- **WHEN** either Windows key is tracked down and D receives its initial key-down without unsupported modifiers
- **THEN** exactly one Show Desktop action is enqueued and the standalone Start gesture is cancelled

#### Scenario: Win+Shift+S initial press
- **WHEN** Windows and Shift are tracked down and S receives its initial key-down without Control or Alt
- **THEN** exactly one built-in screen-snipping action is enqueued

#### Scenario: Repeat and release
- **WHEN** an admitted chord key repeats or receives its matching key-up
- **THEN** no duplicate action is enqueued and the active-key fence is cleared on release

### Requirement: Reversible exact-window Show Desktop session
The first Win+D SHALL minimize only eligible visible non-minimized task windows and record only exact identities that succeeded. The next Win+D SHALL restore only those exact still-minimized windows, including request-owned windows hidden by the minimized shelf, then clear the session.

#### Scenario: First cycle
- **WHEN** Show Desktop is inactive and eligible visible task windows exist
- **THEN** each exact window is minimized once and only successful targets enter the restore set

#### Scenario: Restore hidden minimized windows
- **WHEN** Show Desktop is active and its exact targets are minimized and hidden by the shelf
- **THEN** the shelf-merged snapshot admits those targets for restore and the session clears after the attempt

#### Scenario: Stale or new window
- **WHEN** an HWND is reused, process/stable identity changes, or a new minimized window was not in the session
- **THEN** that window is not restored by the Show Desktop cycle

### Requirement: Built-in screen-snipping route remains bounded
Win+Shift+S SHALL invoke only the fixed Windows screen-clipping protocol through the verified owned-shell broker lifecycle and SHALL leave no request-owned Explorer broker after overlay dismissal or failure.

#### Scenario: Overlay accepted and dismissed
- **WHEN** Windows accepts the fixed `ms-screenclip:///?source=HotKey` route and the overlay is dismissed
- **THEN** the built-in overlay is observed, the request-owned broker is cleaned up, and SuperDesktop remains alive

#### Scenario: Protocol failure
- **WHEN** the fixed protocol cannot be activated
- **THEN** SuperDesktop logs a scoped error, performs no fallback launch, cleans request-owned state, and remains alive

