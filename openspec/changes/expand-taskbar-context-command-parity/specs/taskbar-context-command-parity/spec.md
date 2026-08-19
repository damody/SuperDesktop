## ADDED Requirements

### Requirement: Owned context menu exposes common Windows commands
The taskbar context menu SHALL expose exactly six owned commands in this order: Search presentation, Show Task View button, Show the desktop, Task Manager, Lock the taskbar, and Taskbar settings.

#### Scenario: Pointer or keyboard opens the menu
- **WHEN** the user opens the owned taskbar context menu
- **THEN** all six rows appear in the normative order with non-empty accessible names and 32 DIP hit targets

#### Scenario: Keyboard navigation wraps
- **WHEN** focus moves above the first or below the last menu row
- **THEN** selection wraps within the same six-command order and activation emits exactly one typed command

### Requirement: Context settings commands are truthful and atomic
Search, Task View, and Lock rows SHALL expose their current value or checked state and SHALL persist exactly one field through the revisioned settings store before updating live state.

#### Scenario: Search presentation activation
- **WHEN** Search is activated repeatedly
- **THEN** its setting cycles Hidden, Search icon only, Search box, then Hidden without modifying unrelated settings

#### Scenario: Task View or Lock activation
- **WHEN** either checked row is activated
- **THEN** only its corresponding boolean is toggled and the next menu open reflects the saved state

#### Scenario: Settings save fails
- **WHEN** the revisioned store rejects a context setting update
- **THEN** the live settings snapshot remains unchanged and the action records a rejected trace

### Requirement: Show desktop remains owned
The Show the desktop row SHALL invoke the existing SuperDesktop ShowDesktopSession cycle and SHALL NOT launch, call, or delegate to `explorer.exe`.

#### Scenario: Explorer is absent
- **WHEN** Show the desktop is invoked from the context menu during shell ownership
- **THEN** eligible windows are minimized or restored through exact owned identities without requiring Explorer

### Requirement: Context menu geometry is measured automatically
Explorer-free UTIT MUST record the six ordered MenuItem names and physical bounds, DPI-derived popup size, checked state, monitor containment, and Lock action result.

#### Scenario: Live context menu passes
- **WHEN** the release app opens the menu at 175 percent scaling
- **THEN** the popup is 200-240 DIP wide, all rows fit without clipping or empty bounds, command order and checked names match the isolated settings, and Lock persists successfully

#### Scenario: Command or geometry regresses
- **WHEN** a row is missing, duplicated, reordered, clipped, outside the monitor, or falsely checked
- **THEN** the focused UTIT case fails and does not emit passed evidence
