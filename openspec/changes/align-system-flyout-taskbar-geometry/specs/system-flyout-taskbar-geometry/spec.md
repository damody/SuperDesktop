## ADDED Requirements

### Requirement: Mode-aware taskbar anchoring
SuperDesktop SHALL place each owned system flyout exactly once above the owned taskbar according to the runtime taskbar anchor.

#### Scenario: Preview taskbar
- **WHEN** SuperDesktop runs beside Explorer in preview mode
- **THEN** the popup bottom is one owned-taskbar height plus the Windows gap above `work_area.bottom`

#### Scenario: Replacement shell taskbar
- **WHEN** SuperDesktop runs as the replacement shell
- **THEN** the popup bottom is one owned-taskbar height plus the Windows gap above `bounds.bottom`, independent of stale work-area reservation

### Requirement: Windows logical proportions across DPI
Owned system flyouts SHALL retain their preferred Windows 11 logical sizes and remain inside the selected monitor across supported DPI, row counts, and monitor origins.

#### Scenario: DPI and row matrix
- **WHEN** input, network/power, volume, or calendar geometry is computed at 96, 144, 168, or 216 DPI with one to three taskbar rows
- **THEN** physical coordinates convert once, preferred dimensions remain logical, and the popup does not overlap the taskbar or leave the monitor

#### Scenario: Constrained negative-origin monitor
- **WHEN** preferred dimensions exceed a small negative-origin monitor
- **THEN** dimensions clamp to at least one logical pixel and all edges remain bounded

### Requirement: Runtime geometry admission
UTIT SHALL treat actual taskbar/flyout rectangles, logical dimensions, gap, containment, replacement, Explorer absence, and recovery as authoritative evidence.

#### Scenario: Owned flyout matrix
- **WHEN** the Explorer-free system-status case opens all four flyout kinds
- **THEN** each record has type, physical rectangles, DPI, logical dimensions, a 2–16 DIP taskbar gap, containment, and the release app hash

#### Scenario: Mutual replacement
- **WHEN** successive system controls are invoked
- **THEN** one owned flyout replaces the prior flyout without opening Explorer or a system flyout host

### Requirement: Release admission
The geometry correction SHALL pass local quality, traceability, Explorer-free, and packaging gates while remaining unarchived.

#### Scenario: Final admission
- **WHEN** implementation is complete
- **THEN** focused/workspace tests, Clippy, architecture/source boundary, release, strict/detailed OpenSpec, shell-parity, screenshot inspection, 18-leaf traceability, and both no-launch NSIS builds pass
