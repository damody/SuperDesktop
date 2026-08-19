## ADDED Requirements

### Requirement: Labeled task buttons share adaptive geometry
The taskbar SHALL count the owned fixed SuperExplorer entry as a labeled task slot and SHALL assign it the same 44-160 DIP adaptive width as ordinary labeled task buttons without changing its independent activation route.

#### Scenario: Crowded one-row taskbar
- **WHEN** a one-row taskbar contains enough labeled entries that 160 DIP per entry would cross the reserved system-control boundary
- **THEN** the fixed entry and every ordinary labeled task shrink to a common bounded width before the boundary and remain in stable visual order

#### Scenario: Spacious or multi-row taskbar
- **WHEN** the available task area can accommodate the labeled slots at the Windows maximum width
- **THEN** the allocator caps each labeled slot at 160 DIP rather than stretching it

#### Scenario: Severely constrained taskbar
- **WHEN** the available task area divided by visible task columns is below 44 DIP
- **THEN** each adaptive labeled width remains 44 DIP and the renderer uses clipping or ellipsis without negative geometry

### Requirement: Fixed running indicator follows its button
The fixed entry SHALL render its long running indicator from the adaptive button width with an 8 DIP inset on both sides and SHALL retain a visible bounded indicator at minimum width.

#### Scenario: Width changes under crowding
- **WHEN** the adaptive fixed-entry width changes
- **THEN** its hit target, label container, and indicator update from the same width during that render

### Requirement: UTIT rejects unified-geometry regressions
The live taskbar UTIT case MUST record the fixed-entry bounds, ordered ordinary task measurements, DPI-derived logical widths, and reserved right-control boundary, and MUST fail on missing bounds, invalid width, fixed/task mismatch above one physical pixel, unstable order, or overlap.

#### Scenario: Production taskbar measurement passes
- **WHEN** the release app renders the isolated crowded one-row labeled profile
- **THEN** the report contains the ordered measurements, observes adaptive shrink, proves fixed/task width parity, and proves zero right-control overlap

#### Scenario: Obsolete fixed geometry returns
- **WHEN** the fixed entry is again hard-coded to 160 DIP while ordinary tasks shrink
- **THEN** focused UTIT and the non-self-referential source contract fail

### Requirement: Product path remains Explorer independent
The unified geometry implementation SHALL NOT launch, invoke, or delegate taskbar rendering or fixed-entry activation to `explorer.exe`.

#### Scenario: Explorer is absent
- **WHEN** Explorer-free shell-parity cases exercise the owned taskbar and recovery boundaries
- **THEN** taskbar geometry and fixed-entry rendering remain owned by SuperDesktop and the UTIT report records successful recovery independently
