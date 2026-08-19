## ADDED Requirements

### Requirement: Windows grouped Jump List presentation
Each non-empty Jump List group SHALL render an accessible 24 DIP heading followed by 32 DIP MenuItems with a semantic 16 DIP glyph column.

#### Scenario: Grouped provider results
- **WHEN** Recent, Frequent, Tasks or Local commands exist
- **THEN** their headings read Recent, Frequent, Tasks or Actions and commands remain ordered/actionable

#### Scenario: Keyboard navigation
- **WHEN** Up/Down/Enter/Escape are used
- **THEN** focus traverses commands only and typed effects remain exactly once

### Requirement: Automated presentation admission
UTIT SHALL retain a screenshot and prove headings, MenuItems, dimensions, hashes and no generic bullet/delegation source.

#### Scenario: Runtime/source gate
- **WHEN** the taskbar Jump List case runs
- **THEN** the owned grouped presentation passes UIA and source checks
