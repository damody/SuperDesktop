## ADDED Requirements

### Requirement: Taskbar always exposes show-all tray control
Every owned taskbar SHALL reserve and render a 32 DIP up-chevron control in the notification area regardless of icon count or placement.

#### Scenario: No registered tray icons
- **WHEN** the notification snapshot contains no registered icons or the provider is unavailable
- **THEN** the up-chevron remains visible, enabled, and UIA-reachable at the same right-cluster position

#### Scenario: All icons fit visible capacity
- **WHEN** every registered icon is placed visibly on the taskbar
- **THEN** the up-chevron remains visible and task width reservation includes its 32 DIP slot

#### Scenario: Overflow icons exist
- **WHEN** one or more registered icons are placed in overflow
- **THEN** exactly one up-chevron is rendered with unchanged pointer, keyboard, hover, pressed, and focus behavior

### Requirement: Show-all popup receives complete current icon snapshot
Pointer and keyboard activation SHALL open or toggle the owned popup with every current notification-area node exactly once, independent of visible/overflow placement.

#### Scenario: Mixed visible and overflow icons
- **WHEN** the taskbar has both visible and overflow notification icons and the user activates the chevron
- **THEN** the popup exposes the union of both placements without duplicates and each icon retains typed activate/context actions

#### Scenario: Snapshot changes before next activation
- **WHEN** icons register, retire, or change placement after a popup cycle
- **THEN** the next activation uses one fresh `accessible_nodes` snapshot and does not reuse stale nodes

#### Scenario: Empty snapshot activation
- **WHEN** the user activates the chevron with no current notification icons
- **THEN** the owned popup opens at minimum geometry and displays a localized no-icons empty state

### Requirement: Show-all control remains accessible and theme-visible
The chevron and popup SHALL retain owned Button/Dialog semantics, localized names, Escape/focus-loss dismissal, and visible light/dark/high-contrast rendering without Explorer delegation.

#### Scenario: Dark and high contrast
- **WHEN** the taskbar uses dark or high-contrast theme
- **THEN** the chevron uses the current taskbar text token and remains distinguishable from its background and interaction states

#### Scenario: Pointer and keyboard parity
- **WHEN** the control is activated by click, Enter, or Space
- **THEN** each route forwards the same complete snapshot and opens/toggles the same owned popup
