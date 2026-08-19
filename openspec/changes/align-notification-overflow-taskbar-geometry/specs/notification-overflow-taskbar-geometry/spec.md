## ADDED Requirements

### Requirement: Mode-aware overflow placement
Owned hidden-icons overflow SHALL sit once above the owned taskbar without overlap or double reservation.

#### Scenario: Preview and shell modes
- **WHEN** overflow opens in either runtime mode
- **THEN** it uses the matching work-area or bounds taskbar anchor, row height and 8 DIP gap

### Requirement: Windows overflow proportions
Overflow SHALL prefer 344 DIP width, six 48 DIP cells per row, 24 DIP vertical padding and at most six rows.

#### Scenario: DPI/icon matrix
- **WHEN** 1–36+ icons are laid out at 96/144/168/216 DPI and 1–3 taskbar rows
- **THEN** rows grow deterministically, clamp at six and remain inside the monitor

#### Scenario: Negative constrained monitor
- **WHEN** available space is smaller or has a negative origin
- **THEN** dimensions stay positive, contained and non-overlapping

### Requirement: Explorer-free forced-overflow UTIT
UTIT SHALL create 20 documented NotifyIcons, open the owned panel and record actual geometry, UIA, callbacks and recovery.

#### Scenario: Runtime overflow
- **WHEN** the shell-parity case runs
- **THEN** width is within 16 DIP of 344, gap is 2–16 DIP, panel is contained/owned, hidden buttons exist, callbacks complete and Explorer recovers

### Requirement: Final admission
The change SHALL pass focused/workspace, Explorer-free, governance, release, strict/detailed, traceability and no-launch package gates while unarchived.

#### Scenario: Completion
- **WHEN** implementation completes
- **THEN** 18 unique task records and both installer hashes are present
