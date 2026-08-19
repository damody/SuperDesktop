## ADDED Requirements

### Requirement: Owned reversible show-desktop session
SuperDesktop SHALL implement Show desktop from its own authoritative window snapshots and SHALL NOT invoke Explorer, shell UI, Win+D, shell COM automation, or undocumented interfaces.

#### Scenario: First activation minimizes admitted windows
- **WHEN** the corner is activated while no Show desktop session is active
- **THEN** the system minimizes only visible eligible top-level windows that were not already minimized and records only successfully minimized exact identities

#### Scenario: Second activation restores exact admitted windows
- **WHEN** the corner is activated while a Show desktop session is active
- **THEN** the system restores without activation only still-live windows whose HWND, process, and stable identity match the original admitted set, then clears the session

#### Scenario: Pre-minimized and new windows are preserved
- **WHEN** a window was minimized before first activation or appears after first activation
- **THEN** neither window is added to the restore set or restored by the second activation

#### Scenario: Stale identity fails closed
- **WHEN** an admitted window is destroyed, replaced, cloaked, made transient, or its identity no longer matches
- **THEN** the restore pass skips it without widening the target set or delegating elsewhere

### Requirement: Windows 11 far-edge presentation
The owned taskbar SHALL render a full-height Show desktop Button flush with the far-right monitor edge using Windows 11 geometry and state treatment.

#### Scenario: One-row 175 percent taskbar
- **WHEN** the taskbar is rendered at 175% DPI with one row
- **THEN** the button occupies the final 8 logical pixels, spans the complete taskbar client height, and has no gap beyond its right edge

#### Scenario: Multi-row taskbar
- **WHEN** the taskbar is rendered with two or three rows
- **THEN** the single button spans the complete multi-row height without horizontal separators between rows

#### Scenario: Visual states and themes
- **WHEN** the pointer hovers, presses, or keyboard-focuses the button in light, dark, or high-contrast presentation
- **THEN** the surface exposes distinguishable Windows-style fill plus edge or focus geometry without relying on color alone

### Requirement: Accessible localized activation
The Show desktop corner SHALL provide equivalent pointer, keyboard, and UI Automation operation with localized accessible naming.

#### Scenario: UI Automation semantics
- **WHEN** an accessibility client inspects the far-right control
- **THEN** it exposes Button role, enabled state, stable automation identity, and localized Show desktop name

#### Scenario: Keyboard activation
- **WHEN** the focused control receives Enter or Space
- **THEN** it invokes exactly the same typed session action as a primary pointer click

#### Scenario: Unsupported input has no effect
- **WHEN** another key or non-primary pointer action reaches the control
- **THEN** no window action or external system UI is invoked

### Requirement: Explorer-absent safety and evidence
The feature SHALL be verified against a controlled production composition with Explorer absent and SHALL leave the host recoverable after the gate.

#### Scenario: Controlled end-to-end cycle
- **WHEN** two visible fixture windows and one pre-minimized fixture window are present with Explorer suppressed
- **THEN** UIA first activation minimizes the two visible windows, second activation restores exactly those two, and the pre-minimized window remains minimized

#### Scenario: No forbidden process or API path
- **WHEN** source and runtime noninterference checks execute
- **THEN** no Explorer, ShellExperienceHost, SystemSettings, Win+D injection, shell URI, or shell automation path is used

#### Scenario: Release admission
- **WHEN** implementation is marked complete
- **THEN** detailed task and strict OpenSpec validation, automated tests, Clippy, release build, headful evidence, hashes, and standalone plus combined NSIS installers have passed while the change remains unarchived
