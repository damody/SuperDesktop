## ADDED Requirements

### Requirement: Start footer matches Windows 11 composition
The owned Start footer SHALL render the account control on the left and exactly one Power action on the right, and SHALL NOT render or expose a footer Settings control.

#### Scenario: Start home opens
- **WHEN** the owned Start home is visible
- **THEN** the footer action group contains exactly one accessible Power button and no Settings button

#### Scenario: Settings remains discoverable
- **WHEN** the footer Settings control is absent
- **THEN** Settings remains available through the owned application/search catalog or taskbar settings without delegating to Explorer

### Requirement: Power geometry and behavior remain Windows-like
The Power button SHALL have a 40 by 40 DIP hit target, SHALL be right-aligned within the 52 DIP footer, and SHALL open the existing owned power menu.

#### Scenario: Power is activated
- **WHEN** the user invokes Power by pointer or keyboard
- **THEN** the owned Sign out, Restart, and Shut down menu appears without launching or invoking `explorer.exe`

### Requirement: UTIT rejects footer regressions
The Explorer-free Start UTIT case MUST record footer action count, Power physical/logical bounds and right inset, Settings absence, power-menu actions, Explorer absence, and recovery.

#### Scenario: Live Start footer passes
- **WHEN** the release app renders Start at the host DPI
- **THEN** UTIT proves one footer action, 38-42 DIP Power dimensions, a 20-36 DIP outer-window right inset, no Settings footer button, complete power actions, and successful recovery

#### Scenario: Gear or geometry returns
- **WHEN** a Settings footer control appears or Power leaves its size/alignment bounds
- **THEN** the focused Start case fails and emits no passed evidence
