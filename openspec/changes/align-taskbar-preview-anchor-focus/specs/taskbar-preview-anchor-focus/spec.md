## ADDED Requirements

### Requirement: Source-anchored preview geometry
SuperDesktop SHALL center an owned task preview over its source task anchor and clamp the complete popup inside the source monitor work area.

#### Scenario: Interior task
- **WHEN** a preview is admitted from a task away from monitor edges
- **THEN** the popup horizontal center matches the source anchor within two physical pixels

#### Scenario: Edge and DPI placement
- **WHEN** the source is near either edge on a scaled or negative-origin monitor
- **THEN** the popup is clamped inside that monitor without cross-monitor drift

#### Scenario: Pointer unavailable
- **WHEN** the physical source anchor cannot be read
- **THEN** deterministic monitor-center placement is used without delegation

### Requirement: Hover preserves foreground focus
A hover-opened preview SHALL neither activate its HWND nor move GPUI keyboard focus from the foreground application.

#### Scenario: Ordinary hover
- **WHEN** a preview opens after the hover delay
- **THEN** the foreground HWND before and after admission is identical

#### Scenario: Pointer action
- **WHEN** a pointer activates or closes a card in a hover preview
- **THEN** the existing typed action runs exactly once

### Requirement: Click preserves keyboard interaction
A click-opened grouped preview SHALL activate and assign its internal focus handle.

#### Scenario: Keyboard selection
- **WHEN** a grouped preview is opened by click
- **THEN** Left, Right, Enter, Delete, and Escape remain available through the focused owned popup

### Requirement: Explorer-free automated admission
The anchor and focus behavior SHALL be admitted by UTIT with Explorer absent and safely recovered.

#### Scenario: UTIT anchor-focus matrix
- **WHEN** the controlled hover case runs
- **THEN** foreground preservation, source/popup bounds, expected anchor, clamp, containment, Explorer absence/recovery, screenshot, and binary hashes are recorded

#### Scenario: Release admission
- **WHEN** the change is complete
- **THEN** focused/workspace, Clippy, architecture/source, release, strict/detailed, UTIT shell-parity, traceability, and both package gates pass while the change remains unarchived
