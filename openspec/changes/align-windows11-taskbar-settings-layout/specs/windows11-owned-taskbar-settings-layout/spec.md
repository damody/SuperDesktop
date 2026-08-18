## ADDED Requirements

### Requirement: Settings window uses logical monitor geometry once
The owned taskbar settings window SHALL convert monitor work-area bounds to GPUI logical pixels exactly once, center within the current work area, and clamp to a 640×480 logical minimum only when the work area can contain it.

#### Scenario: 175 percent primary display
- **WHEN** the monitor is 3840×2160 at 168 DPI with a bounded work area
- **THEN** WindowOptions contains logical dimensions and Windows performs the single physical scaling

#### Scenario: Negative-origin small display
- **WHEN** a secondary monitor has a negative origin and less than the preferred logical size
- **THEN** the window remains within that monitor and no width or height becomes negative

### Requirement: Page fills the window with centered responsive content
The settings root SHALL fill the window, own one vertical scroll surface, center a content column capped at 1000 logical pixels, and use 32px normal or 16px compact padding without a persistent right-side dead canvas.

#### Scenario: Wide work area
- **WHEN** the logical window is wider than the maximum content width plus normal padding
- **THEN** equal left and right margins center the bounded content column

#### Scenario: Compact work area
- **WHEN** the logical window approaches the 640px floor
- **THEN** padding reduces, cards use available width, and horizontal clipping does not occur

### Requirement: Windows 11 settings visual metrics are consistent
The page SHALL use Windows 11 light/dark/high-contrast tokens, 8px card radii, 1px borders, 64px section headers, minimum 56px rows, 44×24 switches, 18px switch thumbs, wrapped secondary text, and visible focus treatment.

#### Scenario: Theme matrix
- **WHEN** light, dark, and high-contrast themes render the same section
- **THEN** geometry remains identical and every state remains distinguishable without color or opacity alone

### Requirement: Every control remains reachable and truthful
All expanded sections, including Automatically hide the taskbar and related settings, SHALL be reachable by scrolling, keyboard, and UIA at 100–225% DPI while preserving authoritative state and save-error feedback.

#### Scenario: Bottom control at 175 percent
- **WHEN** the page initially shows its top and the user scrolls or navigates to the bottom
- **THEN** the auto-hide and related rows become fully visible and remain operable

#### Scenario: Save failure
- **WHEN** an owned setting action fails
- **THEN** the error remains accessible in the page flow and the authoritative value/revision stay unchanged

### Requirement: Settings presentation never delegates to Windows Shell UI
The page SHALL render and operate entirely inside SuperDesktop and MUST NOT invoke Explorer, `Shell_TrayWnd`, `ms-settings`, SystemSettings, or inbox taskbar Settings UI.

#### Scenario: Related row is unavailable to the owned router
- **WHEN** a related control cannot be served by an owned surface
- **THEN** it remains truthful and no system Settings process or URI is launched
