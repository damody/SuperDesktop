## ADDED Requirements

### Requirement: Owned right-click popup native topmost state
Every independent SuperDesktop right-click popup SHALL be promoted above ordinary non-topmost application windows before SuperDesktop reports the popup as opened.

#### Scenario: Task Jump List opens
- **WHEN** the user right-clicks a running task button
- **THEN** the Jump List native window has topmost extended style while visible
- **AND** promotion does not activate an unrelated application window

#### Scenario: Taskbar background menu opens
- **WHEN** the user right-clicks taskbar background
- **THEN** the taskbar context native window has topmost extended style while visible

#### Scenario: System-control menu opens
- **WHEN** the user right-clicks the input-method or volume control
- **THEN** the corresponding system-control context native window has topmost extended style while visible

### Requirement: Popup promotion failure is fail closed
SuperDesktop MUST NOT retain or report an independent right-click popup whose owned HWND cannot be promoted to topmost.

#### Scenario: HWND or promotion is rejected
- **WHEN** HWND extraction or the native topmost adapter fails
- **THEN** SuperDesktop removes the popup window
- **AND** clears its popup slot
- **AND** writes the popup kind and failure to console and action trace
- **AND** does not emit the popup-opened success trace

### Requirement: Topmost popup preserves context-menu lifetime
Topmost promotion SHALL occur once per popup creation and SHALL preserve normal focus-loss dismissal.

#### Scenario: Focus moves away
- **WHEN** a topmost right-click popup loses activation to another window
- **THEN** the popup closes and clears its slot

#### Scenario: Popup remains open
- **WHEN** the popup is visible and focused
- **THEN** SuperDesktop does not start a polling or recurring z-order worker for that popup
