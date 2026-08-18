## ADDED Requirements

### Requirement: Product Start is exclusively owned by SuperDesktop
SuperDesktop SHALL render its GPUI `StartView` for every taskbar Start invocation in preview, Shell and verification modes. Product composition MUST NOT invoke Explorer, ExplorerPatcher, SearchHost or another system Start host to present Start.

#### Scenario: Preview Start invocation
- **WHEN** the user invokes Start during ordinary preview mode
- **THEN** SuperDesktop opens the owned Start window and performs no system Start-host invocation

#### Scenario: Shell Start invocation
- **WHEN** the user invokes Start after SuperDesktop has Shell authority
- **THEN** the same owned Start implementation opens with the same model and actions used by preview

#### Scenario: Historical platform probe remains present
- **WHEN** capability verification directly exercises the platform Start probe outside product composition
- **THEN** the probe may retain its historical behavior but no product Start action calls it

### Requirement: Owned Start exposes complete bounded sections
The owned Start surface SHALL render Search, at most twelve Pinned entries in a six-column grid, at most six Recommended entries, an alphabetical All apps page, Account, Settings and one collapsed Power control.

#### Scenario: Start Home opens
- **WHEN** the owned Start window opens with an empty query
- **THEN** Search, Pinned, Recommended, Account, Settings and collapsed Power are visible with native icons or truthful fallbacks

#### Scenario: All apps opens
- **WHEN** the user invokes All apps
- **THEN** Start replaces Home content with a bounded alphabetical application list and a Back action

#### Scenario: Search provider is unavailable
- **WHEN** the owned search provider fails or refuses a request
- **THEN** Start presents a truthful owned unavailable/error state and does not delegate to a system UI

### Requirement: Start activation and persistence remain owned
Start SHALL activate supported app, file, folder, Settings and confirmed Power actions through typed SuperDesktop/platform commands, and SHALL persist pin/recent snapshots without requiring Explorer's Start process.

#### Scenario: Application activation
- **WHEN** the user invokes an application result by pointer, keyboard or UIA
- **THEN** SuperDesktop sends exactly one owned activation command and dismisses Start according to the existing contract

#### Scenario: Pin snapshot changes
- **WHEN** a Start interaction changes the persisted pin or recent snapshot
- **THEN** the settings store saves the bounded snapshot and a later owned Start instance restores it

#### Scenario: Power menu remains collapsed by default
- **WHEN** Start opens
- **THEN** Sign out, Restart and Shut down actions are absent until the user explicitly opens Power and accepts the existing confirmation flow

### Requirement: Start placement and input are mode-independent
Start SHALL center and clamp above the active monitor work-area edge and SHALL preserve Escape, arrows, Enter, pointer, UIA, IME composition and focus-return behavior in every execution mode.

#### Scenario: High-DPI Start placement
- **WHEN** Start opens at 175% DPI
- **THEN** its bounds remain within the work area with the configured bottom gap and all required controls have actionable UIA bounds

#### Scenario: Repeated Start invocation
- **WHEN** Start is already open and the Start control is invoked again
- **THEN** SuperDesktop closes exactly that owned Start window, clears its stored handle and leaves no second Start window

#### Scenario: IME query composition
- **WHEN** an IME composition updates and then commits a Start query
- **THEN** composition does not dispatch premature search and the committed query starts one bounded owned search generation

### Requirement: Desktop marquee remains independent from Start
The desktop SHALL retain live normal/reverse marquee selection, Ctrl-additive selection, lost-button cancellation and fixed SuperExplorer pointer activation after Start becomes exclusively owned.

#### Scenario: Reverse marquee after closing Start
- **WHEN** the user closes Start and reverse-drags across multiple desktop items at host DPI
- **THEN** the visible marquee selects every intersecting item and no Start window consumes the desktop pointer events

#### Scenario: Fixed entry remains pointer-addressable
- **WHEN** a real desktop item had a persisted position overlapping the reserved SuperExplorer cell
- **THEN** layout reconciliation rehomes the conflicting item and pointer activation launches SuperExplorer rather than the overlapped item

### Requirement: Exclusive ownership is auditable
The change SHALL provide source, automated, headful and packaging evidence proving owned Start and desktop-marquee behavior, and every completed task SHALL map to unique evidence.

#### Scenario: Source delegation guard
- **WHEN** the owned-Start source guard scans product composition
- **THEN** it rejects any system Start-host invocation reachable from the taskbar Start callback

#### Scenario: Evidence or strict validation is incomplete
- **WHEN** a task lacks unique evidence or strict OpenSpec validation fails
- **THEN** `G-TRACE` remains failed and the change cannot be reported complete
