## ADDED Requirements

### Requirement: Child taskbar gestures are exact and isolated
The taskbar SHALL classify left click, right click, keyboard activation, and UI Automation activation at the target control and MUST emit exactly one typed action. A handled child gesture MUST NOT invoke the taskbar-background context action or another child action.

#### Scenario: Primary pointer action
- **WHEN** a user left-clicks an enabled affected control
- **THEN** exactly its primary action is emitted and no contextual or background action is emitted

#### Scenario: Context pointer action
- **WHEN** a user right-clicks an enabled affected control
- **THEN** exactly its contextual action is emitted and no primary or background action is emitted

#### Scenario: Keyboard and UIA activation
- **WHEN** an affected control is invoked by Enter, Space, or UI Automation
- **THEN** it emits the same primary action as one left click exactly once

#### Scenario: Unavailable control
- **WHEN** a provider-backed control is unavailable
- **THEN** it exposes truthful unavailable state and emits no fabricated provider mutation

### Requirement: Input and volume controls provide Explorer-aligned pointer behavior
The input control SHALL toggle the owned input-profile flyout on left click and SHALL open an owned context menu with the fixed Language preferences action on right click. The volume control SHALL toggle the owned volume flyout on left click and SHALL open an owned context menu with compile-time Open volume mixer and Sound settings actions on right click. No context action SHALL accept a caller-controlled executable, URI, verb, argument, or working directory.

#### Scenario: Input left click
- **WHEN** the input-language control is left-clicked while its flyout is closed
- **THEN** the owned input flyout opens, receives focus, and no taskbar background menu opens

#### Scenario: Input right click
- **WHEN** the input-language control is right-clicked
- **THEN** the owned input context menu opens with Language preferences and the input flyout and taskbar background menu remain closed

#### Scenario: Volume left click
- **WHEN** the volume control is left-clicked while its flyout is closed
- **THEN** the owned volume flyout opens, receives focus, and no taskbar background menu opens

#### Scenario: Volume right click
- **WHEN** the volume control is right-clicked
- **THEN** the owned volume context menu opens with Open volume mixer and Sound settings and no volume flyout or taskbar background menu opens

#### Scenario: Fixed context action rejected
- **WHEN** Windows rejects a fixed input or volume context action
- **THEN** the app records a truthful rejected terminal and does not attempt an alternate target

### Requirement: Notification icons deliver exact native intent
Visible and overflow notification icons SHALL emit `Activate` only for left click, Enter, Space, or UIA Invoke and SHALL emit `Context` only for right click. The notification compatibility host MUST translate that event through the icon's negotiated version exactly once and MUST preserve the pointer coordinates required by the negotiated context callback.

#### Scenario: Visible icon left and right clicks
- **WHEN** a registered visible icon is left-clicked and then right-clicked
- **THEN** its owner observes one Activate callback followed by one Context callback and the taskbar background menu remains absent

#### Scenario: Overflow icon left and right clicks
- **WHEN** a registered overflow icon is left-clicked and then right-clicked
- **THEN** its owner observes one Activate callback followed by one Context callback and the overflow surface is not replaced by the taskbar background menu

#### Scenario: Legacy and version-four payloads
- **WHEN** the same intents are delivered to legacy and version-four icons
- **THEN** each payload uses its negotiated identifier layout and the exact native activation/context event for that version

#### Scenario: Dead or stale owner
- **WHEN** an icon owner is dead, stale, or past the callback deadline
- **THEN** delivery is rejected once and no other icon receives the event

### Requirement: Application buttons preserve Explorer state reduction
A taskbar application button SHALL use current observed window state for left-click behavior and SHALL reserve right click for its Jump List. A single inactive window SHALL activate, a single active window SHALL minimize, a minimized window SHALL restore and activate, and a multi-window group SHALL open its owned preview. Right click MUST NOT activate, minimize, restore, or open the taskbar background menu.

#### Scenario: Inactive single window
- **WHEN** an inactive single-window task button is left-clicked
- **THEN** that exact window activates without opening a preview or Jump List

#### Scenario: Active single window
- **WHEN** the active single-window task button is left-clicked
- **THEN** that exact window minimizes

#### Scenario: Minimized single window
- **WHEN** a minimized single-window task button is left-clicked
- **THEN** that exact window restores and activates

#### Scenario: Multi-window group
- **WHEN** a multi-window application button is left-clicked
- **THEN** one owned group preview opens without changing window activation until a preview item is selected

#### Scenario: Application right click
- **WHEN** an application button is right-clicked in any window state
- **THEN** one owned Jump List opens and no primary or taskbar-background action occurs

### Requirement: Owned taskbar popups are exclusive and recoverable
System flyouts, input/volume context menus, notification overflow, group previews, Jump Lists, and the taskbar background menu SHALL form one owned popup domain. Opening one SHALL dismiss conflicting members; reinvoking the same control SHALL toggle its popup closed; Escape and window deactivation SHALL dismiss the popup without allowing stale dismissal to clear a replacement.

#### Scenario: Conflicting popup switch
- **WHEN** one owned taskbar popup is open and a different popup control is invoked
- **THEN** the first popup closes and only the newly requested popup remains visible

#### Scenario: Same-control toggle
- **WHEN** the owner control of an open toggleable popup is invoked again with the same primary gesture
- **THEN** that popup closes and no replacement popup opens

#### Scenario: Escape and deactivation
- **WHEN** an owned popup receives Escape or loses activation
- **THEN** it closes, returns a consistent ownership slot, and the taskbar remains interactive

#### Scenario: Stale dismissal
- **WHEN** an old popup's delayed dismissal runs after a replacement popup opens
- **THEN** the replacement remains owned and visible

### Requirement: UTIT blocks completion on exact pointer parity evidence
UTIT SHALL include mandatory headful cases for input/volume controls, notification icons, and taskbar application buttons. Each case MUST use real pointer input, record the target and mouse button, assert the exact intended terminal, assert absence of unintended taskbar-background behavior, and write a versioned JSON report with hashed artifacts and Explorer recovery state.

#### Scenario: Complete current-host pointer run
- **WHEN** all required controls and fixtures are available
- **THEN** UTIT proves both buttons for every control family and reports passed only when all exact-action, popup, and recovery assertions pass

#### Scenario: Limited input profile host
- **WHEN** fewer than two suitable real input profiles exist
- **THEN** only the profile-switch mutation subcheck is evidence-backed not-applicable while input pointer routing remains mandatory

#### Scenario: Wrong or duplicate action
- **WHEN** a gesture emits the wrong action, both primary and context actions, a duplicate callback, or a taskbar-background popup
- **THEN** the relevant UTIT case fails and preserves trace, UIA state, and recovery evidence

#### Scenario: Timeout or Explorer recovery failure
- **WHEN** a pointer case times out or its Explorer-free recovery is not observed
- **THEN** the case fails and the overall shell-parity decision is incomplete or failed
