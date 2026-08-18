## ADDED Requirements

### Requirement: Preview and Shell use distinct bottom anchors
SuperDesktop SHALL place the taskbar at the current Explorer work-area bottom in Preview mode and at the physical monitor bottom in Shell mode.

#### Scenario: Explorer remains present
- **WHEN** SuperDesktop runs in Preview while Explorer owns the system work area
- **THEN** the owned taskbar stays immediately above that work-area bottom without covering Explorer’s taskbar

#### Scenario: Explorer is absent
- **WHEN** SuperDesktop runs in admitted Shell mode
- **THEN** the owned taskbar occupies the monitor bottom and reserves exactly its physical row height

### Requirement: Taskbar lock state is persisted and accessible
SuperDesktop SHALL persist whether the taskbar is locked, SHALL default legacy settings to locked, and SHALL expose the same checked state through the owned context menu and settings surface.

#### Scenario: Toggle from context menu
- **WHEN** the user invokes “Lock the taskbar”
- **THEN** exactly one atomic save toggles the authoritative value and every owned taskbar reconciles to it

#### Scenario: Save fails
- **WHEN** lock persistence fails
- **THEN** the authoritative lock value and native resize style remain unchanged

### Requirement: Unlocked top edge selects one through three rows
An unlocked taskbar SHALL expose native top-edge resizing, quantize its logical height to one, two, or three 40px rows, persist a changed row, and snap its HWND without moving the bottom edge.

#### Scenario: Drag across row thresholds
- **WHEN** the user drags the unlocked top edge through one-, two-, and three-row heights
- **THEN** each distinct row count is saved once and the window snaps to the exact DPI-scaled height

#### Scenario: Taskbar is locked
- **WHEN** the user attempts the same drag while locked
- **THEN** no resize strip or native thick frame is active and row state does not change

### Requirement: Shell AppBar follows runtime row changes
Shell mode SHALL update the controlled AppBar reservation on the same owned HWND whenever rows change and SHALL leave Preview without AppBar registration.

#### Scenario: Shell grows from one to three rows
- **WHEN** an unlocked Shell taskbar changes from one row to three
- **THEN** the work area and owned HWND reserve exactly 120 logical pixels converted at monitor DPI

### Requirement: Multi-row chrome is continuous
The taskbar SHALL draw one outer top border and MUST NOT draw horizontal separators between rows.

#### Scenario: Two or three rows render
- **WHEN** the taskbar displays multiple rows in light, dark, or high contrast
- **THEN** no full-width line appears between adjacent rows and task indicators remain visible

### Requirement: Resize never targets Explorer
SuperDesktop MUST validate the caller-owned HWND before changing native resize style and MUST NOT find, invoke, resize, or restyle Explorer or `Shell_TrayWnd`.

#### Scenario: Foreign HWND is supplied
- **WHEN** the style adapter receives a valid HWND owned by another process
- **THEN** it rejects the request without changing that window

### Requirement: Resize completion is auditable and packaged
The change SHALL provide automated, headful, Explorer-free, accessibility, traceability, release, and installer evidence with a unique link for every atomic task.

#### Scenario: Mandatory evidence is missing
- **WHEN** a placement, drag, lock, AppBar, chrome, process, hash, or unique task record is missing
- **THEN** the corresponding blocking gate remains failed
