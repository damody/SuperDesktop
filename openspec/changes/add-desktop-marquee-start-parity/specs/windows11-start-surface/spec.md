## ADDED Requirements

### Requirement: Start home must expose Windows 11 sections
The owned Start home SHALL render Search, a bounded Pinned grid, Recommended rows, Account, Settings, and one collapsed Power control with truthful accessible roles and labels.

#### Scenario: Start opens with a catalog
- **WHEN** owned Start opens without a query
- **THEN** up to 12 pins appear in a six-column grid and up to six recent items appear under Recommended

#### Scenario: Catalog item has a Shell path
- **WHEN** a pin or recommendation activates an existing path
- **THEN** its native Shell icon is rendered through the shared BC7-capable image path

#### Scenario: Icon is unavailable
- **WHEN** native icon resolution fails
- **THEN** a semantic fallback remains visible with the complete readable label

### Requirement: All apps and search must be distinct deterministic modes
Start SHALL expose an All apps action that renders installed apps alphabetically, while nonempty search text SHALL render ranked provider results with title, subtitle, and icon.

#### Scenario: User chooses All apps
- **WHEN** the All apps action is invoked
- **THEN** Start renders a back action and a bounded alphabetical application list

#### Scenario: User types a query
- **WHEN** committed search text is nonempty
- **THEN** ranked results replace the home/all-apps content without changing persisted pins

#### Scenario: User clears the query
- **WHEN** search text becomes empty
- **THEN** the previously selected home or All apps page is shown again

### Requirement: Start footer and power actions must be safely structured
Start SHALL show Account and Settings in the footer and SHALL expose sign-out/restart/shutdown only after the Power button opens a flyout; admitted power actions MUST retain confirmation.

#### Scenario: Power is collapsed
- **WHEN** Start first opens
- **THEN** no destructive power action is directly exposed in the footer

#### Scenario: Power is opened and dismissed
- **WHEN** the user opens Power and then presses Escape
- **THEN** the flyout closes without executing an action or closing Start

#### Scenario: Power action is selected
- **WHEN** a flyout action is invoked
- **THEN** the existing confirmation adapter decides execution and reports accepted, cancelled, or failed

### Requirement: Start placement and input must remain accessible
Start SHALL open centered above the bottom work-area edge, clamp to small monitors, focus Search, and preserve Escape, arrows, Enter, IME, and UI Automation navigation.

#### Scenario: Normal monitor
- **WHEN** Start opens on a monitor wider than its preferred width
- **THEN** horizontal margins are equal and a 12-logical-pixel bottom gap is retained

#### Scenario: Small monitor
- **WHEN** the work area is smaller than preferred Start dimensions
- **THEN** width and height clamp without leaving the work area or hiding the footer

#### Scenario: Keyboard opens and navigates Start
- **WHEN** Start receives keyboard or accessibility input
- **THEN** focus begins at Search and navigation reaches Pinned/All apps/Recommended/Account/Settings/Power with stable names
