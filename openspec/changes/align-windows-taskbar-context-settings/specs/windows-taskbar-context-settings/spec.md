## ADDED Requirements

### Requirement: Empty taskbar context menu
The system SHALL open an owned Windows 11-style context menu when the user invokes context on empty taskbar space, SHALL expose Task Manager and Taskbar settings commands, and SHALL NOT invoke Explorer or Windows Settings.

#### Scenario: Pointer opens one clamped menu
- **WHEN** the user right-clicks taskbar background on any live monitor
- **THEN** exactly one owned menu opens near the pointer and remains inside that monitor's work area

#### Scenario: Task input does not bubble to background
- **WHEN** the user right-clicks an application task button
- **THEN** only the application command surface opens and the empty-taskbar menu remains closed

#### Scenario: Keyboard and dismissal parity
- **WHEN** the menu is operated with arrows, Enter, Escape, focus loss, or pointer activation
- **THEN** the same typed command or dismissal occurs and focus returns to the originating taskbar when it remains live

### Requirement: Validated taskbar commands
The composition root SHALL execute taskbar context commands only after validation and SHALL report failure without changing unrelated taskbar state.

#### Scenario: Task Manager launch
- **WHEN** the user activates Task Manager
- **THEN** the system resolves and launches the inbox executable without shell expansion, environment substitution, or Explorer delegation

#### Scenario: Command failure
- **WHEN** command validation or launch fails
- **THEN** the menu closes or reports a bounded accessible error, records a typed trace, and leaves settings and task membership unchanged

### Requirement: Windows 11 application command presentation
Application Jump Lists SHALL preserve typed provider/local commands while rendering Windows 11 command-row geometry, section separation, hover, pressed, focus, disabled, and destructive states.

#### Scenario: Pin and close commands remain actionable
- **WHEN** an application task has live windows and the Jump List opens
- **THEN** pin/unpin and close-all commands use the existing typed effects and expose accurate enabled/risk state

#### Scenario: Provider failure remains bounded
- **WHEN** a Jump List provider fails or returns malformed data
- **THEN** only validated local commands remain and the surface does not execute unvalidated provider content

### Requirement: Owned taskbar settings hierarchy
The system SHALL provide an owned **Personalization > Taskbar** window with Windows 11-style grouped cards for taskbar items, system tray icons, other system tray icons, taskbar behavior, and related settings.

#### Scenario: Supported controls are interactive
- **WHEN** the settings window renders saved settings
- **THEN** Search mode, Task View, labels, grouping, previews, monitor policy, rows, and alignment expose accurate values and pointer/keyboard/UIA activation

#### Scenario: Unsupported controls are truthful
- **WHEN** Widgets, pen menu, touch keyboard ownership, or auto-hide is displayed without an owned implementation
- **THEN** the row is disabled, includes an unavailable explanation, and emits no mutation

#### Scenario: Singleton settings lifecycle
- **WHEN** Taskbar settings is invoked repeatedly or from another monitor
- **THEN** one settings window remains, is activated or repositioned safely, and closes during product teardown

### Requirement: Bounded persistent settings
`TaskbarSettings` SHALL persist bounded Search visibility, Task View visibility, and alignment fields while preserving existing taskbar settings and additive decoding behavior.

#### Scenario: Missing fields use stable defaults
- **WHEN** an existing settings document omits the new fields
- **THEN** Search is hidden, Task View is visible, alignment is left, and every existing explicit field remains unchanged

#### Scenario: Invalid field is isolated
- **WHEN** one new enum field contains an unknown value
- **THEN** only that field falls back to its default and valid sibling fields round-trip unchanged

#### Scenario: Candidate validation
- **WHEN** a settings model proposes a row count outside one through three or an unrecognized enum
- **THEN** the candidate is rejected before persistence and the saved document remains unchanged

### Requirement: Atomic live settings application
The system SHALL publish taskbar setting changes only after an atomic settings-store save succeeds and SHALL update every live taskbar consistently.

#### Scenario: Successful save updates every taskbar
- **WHEN** a supported control produces a valid candidate and persistence succeeds
- **THEN** every live taskbar updates Search, Task View, alignment, labels, grouping, previews, monitor policy, and rows from the returned saved revision

#### Scenario: Save failure preserves authoritative state
- **WHEN** persistence fails
- **THEN** all taskbars retain the previous saved settings and the settings window exposes a bounded accessible error

#### Scenario: Stale window cannot overwrite newer settings
- **WHEN** an older settings-window candidate targets a superseded revision
- **THEN** it is rejected or reconciled against the latest revision without resurrecting stale fields

### Requirement: Search, Task View, and alignment behavior
The rendered taskbar SHALL apply the saved Search mode, Task View visibility, and left/center alignment without changing the edge-anchored system-status area.

#### Scenario: Search modes
- **WHEN** Search is hidden, icon, or box
- **THEN** the control is respectively absent, rendered as a Windows-style icon, or rendered as a labeled search box, and visible modes activate owned Start search

#### Scenario: Task View disabled
- **WHEN** Task View visibility is off
- **THEN** the control is absent from rendering, keyboard order, hit testing, and UIA

#### Scenario: Alignment and overflow
- **WHEN** alignment changes with one to three rows, grouped tasks, or constrained width
- **THEN** the task cluster is left- or center-aligned as a bounded unit, preserves task order, and never overlaps status controls

### Requirement: Theme, DPI, localization, and accessibility parity
All new surfaces SHALL remain usable in light, dark, high-contrast, reduced-motion, localized, and 100–500% scale conditions.

#### Scenario: Visual geometry matrix
- **WHEN** context/settings surfaces render across supported DPI and themes
- **THEN** row heights, corner radii, padding, focus indicators, clipping, and hit targets satisfy the committed Windows 11 geometry matrix

#### Scenario: UIA and keyboard inventory
- **WHEN** an accessibility client enumerates the surfaces
- **THEN** dialogs, menus, buttons, switches, dropdowns, disabled reasons, current values, and errors have accurate names, roles, states, and supported patterns

#### Scenario: Long localized text
- **WHEN** Traditional Chinese or English labels and explanations reach their bounded maximum
- **THEN** supporting text wraps without overlapping controls or leaving the monitor work area

### Requirement: Labeled task running indicator geometry
Running indicators SHALL match the configured task-button presentation instead of using one fixed icon-only length.

#### Scenario: Labeled wide task button
- **WHEN** readable labels are enabled for a task button
- **THEN** its bottom running indicator extends across the bounded button content width with Windows-reference horizontal insets

#### Scenario: Icon-only task button
- **WHEN** labels are hidden and the task has a real icon
- **THEN** the indicator uses the short icon-only Windows geometry

#### Scenario: State layers retain common width
- **WHEN** a labeled task becomes active, grouped, minimized, progresses, or flashes for attention
- **THEN** all indicator layers preserve the labeled-button width basis and do not collapse to the icon-only length

### Requirement: Mode independence and packaging
The owned context and settings behavior SHALL be available in Preview and controlled Shell modes, SHALL create no Explorer-exclusive identity, and SHALL ship through standalone and combined installers.

#### Scenario: Preview non-interference
- **WHEN** the feature runs in Preview while Explorer is present
- **THEN** Explorer remains running and SuperDesktop does not register or mutate an Explorer-owned menu/settings identity

#### Scenario: Explorer-free operation
- **WHEN** controlled Shell mode runs without Explorer
- **THEN** context menus, settings persistence, and supported commands remain functional through owned surfaces

#### Scenario: Installer contents and cleanup
- **WHEN** standalone and combined installers are built and their uninstall manifests are inspected
- **THEN** no extra provider is required, all modified binaries are packaged, and uninstall leaves no feature-specific file or registration residue
