## ADDED Requirements

### Requirement: Every owned GUI surface has closed manifest coverage
UTIT SHALL maintain a unique compiled manifest entry for every owned GUI surface, declaring reference family, owner, variants, geometry rules, required controls/actions/artifacts, and Explorer policy. Every GUI catalog case MUST map to at least one entry and every mandatory entry MUST map to an executable or evidence-backed conditional case.

#### Scenario: Missing or duplicate coverage
- **WHEN** a surface is missing, duplicated, or maps only one direction between manifest and catalog
- **THEN** catalog validation fails before any GUI process starts

#### Scenario: Conditional hardware surface
- **WHEN** a manifest entry requires unavailable physical hardware
- **THEN** UTIT records blocked or evidence-backed not-applicable and never passed

### Requirement: GUI measurements use one normalized schema
Each first-wave headful case SHALL emit `gui-parity-measurement/v1` with physical window/content/region/control rectangles, window DPI, variant identity, popup owner/lifecycle, action terminals, Explorer state, screenshots, and artifact hashes. UTIT MUST derive DIP once and reject invalid, stale, missing, or contradictory measurements.

#### Scenario: Valid high-DPI measurement
- **WHEN** a physical rectangle and nonzero window DPI are reported
- **THEN** UTIT converts every coordinate and size by `96 / dpi` exactly once and evaluates manifest rules

#### Scenario: Invalid geometry
- **WHEN** DPI is zero, a rectangle is inverted/outside its monitor, required regions overlap, or a hit target is below its minimum
- **THEN** validation fails with surface, variant, rule, expected, actual, and delta

### Requirement: First-wave shell chrome follows shared Windows metrics
Taskbar, Start, system flyouts, notification overflow, system/taskbar context menus, Jump Lists, previews, task view, and Alt-Tab SHALL consume shared Windows GUI metrics for canonical widths, row/target sizes, padding, radii, popup gaps, and monitor clamping. Content formulas MUST remain bounded and text scaling MUST NOT reduce hit targets or create overlap.

#### Scenario: Canonical taskbar and popup geometry
- **WHEN** first-wave surfaces render at supported rows and DPI
- **THEN** taskbar rows are 40 DIP, primary targets are at least 44×40 DIP, status targets at least 36×36 DIP, and taskbar popups use a nominal 8 DIP gap within 2–16 DIP

#### Scenario: Bounded system panels
- **WHEN** input/volume/network/calendar or notification overflow opens
- **THEN** it uses its declared 360/380/344 DIP preferred width, remains monitor-contained, and preserves declared internal proportions

### Requirement: Explorer is absent from normal product behavior
Normal SuperDesktop/SuperExplorer composition, UI callbacks, providers, Settings actions, and launch/focus routes MUST NOT start, invoke, or depend on Explorer. Explorer MAY appear only in guardian recovery, installer rollback, test watchdogs after measurement, or explicit Return to default Explorer behavior.

#### Scenario: Forbidden source reference
- **WHEN** a production source outside an allowed recovery/rollback module contains an Explorer executable or delegation route
- **THEN** the Explorer policy gate fails with the file and forbidden token

#### Scenario: Explorer-free headful run
- **WHEN** an Explorer-free case measures a surface
- **THEN** Explorer remains absent for the entire measurement and is recovered only after the terminal report or failure watchdog

### Requirement: GUI parity failures are actionable and blocking
UTIT SHALL make mandatory first-wave geometry, interaction, accessibility, locale/theme, popup ownership, and Explorer-free failures block shell-parity completion. Reports MUST retain normalized deltas, screenshots, logs, recovery state, and hashes.

#### Scenario: One region drifts
- **WHEN** any mandatory named region violates its rule
- **THEN** the case fails even if the process exits zero or the screenshot exists

#### Scenario: Complete first-wave run
- **WHEN** all current-host first-wave cases satisfy their manifest entries and conditional gates are truthfully disposed
- **THEN** UTIT reports the selected GUI parity matrix passed or partial only because of explicit filtering, not hidden uncovered surfaces
