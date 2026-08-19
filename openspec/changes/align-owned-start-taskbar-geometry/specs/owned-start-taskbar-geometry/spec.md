## ADDED Requirements

### Requirement: Mode-aware Start anchoring
Owned Start SHALL end above the owned taskbar with the Windows gap and SHALL never overlap it.

#### Scenario: Preview mode
- **WHEN** SuperDesktop runs beside Explorer
- **THEN** Start uses the taskbar anchored to `work_area.bottom`

#### Scenario: Replacement shell
- **WHEN** SuperDesktop runs as shell with a stale work area
- **THEN** Start uses the taskbar anchored to `bounds.bottom` without double or missing reservation

### Requirement: Windows Start proportions
Owned Start SHALL prefer 640×720 DIP and clamp inside the monitor above one to three taskbar rows.

#### Scenario: DPI matrix
- **WHEN** geometry is computed at 96, 144, 168 or 216 DPI in either mode
- **THEN** dimensions convert once, center horizontally and remain bounded

#### Scenario: Constrained negative monitor
- **WHEN** available space is smaller than preferred size
- **THEN** Start remains at least one logical pixel and does not overlap the taskbar

### Requirement: Explorer-free UTIT admission
UTIT SHALL record actual Start/taskbar geometry and prove owned behavior with Explorer absent and recovered.

#### Scenario: Runtime geometry
- **WHEN** `gui-start` opens home, all apps and power in shell mode
- **THEN** width is within 16 DIP of 640, gap is 4–20 DIP, rectangles do not overlap, Start is contained, and screenshots/hashes are recorded

#### Scenario: No system Start host
- **WHEN** owned Start opens
- **THEN** no new StartMenuExperienceHost, SearchHost or ShellExperienceHost PID appears

### Requirement: Release admission
The change SHALL pass focused/workspace, Explorer-free, governance, release, strict/detailed, traceability and no-launch package gates while unarchived.

#### Scenario: Final gate
- **WHEN** implementation completes
- **THEN** all 18 atomic tasks have unique passing evidence and both NSIS packages are hash-bound
