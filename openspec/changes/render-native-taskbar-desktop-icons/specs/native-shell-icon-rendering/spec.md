## ADDED Requirements

### Requirement: Native icon pixels must be safely owned
The platform layer SHALL convert Windows icons into validated owned RGBA buffers and SHALL release only handles and GDI objects owned by SuperDesktop.

#### Scenario: Shell icon succeeds
- **WHEN** Windows returns an owned Shell icon for an existing item
- **THEN** the platform returns nonempty RGBA pixels and destroys the native icon after conversion

#### Scenario: Borrowed window icon succeeds
- **WHEN** a window or class returns a borrowed icon handle
- **THEN** the platform converts it without destroying the borrowed handle

#### Scenario: Conversion fails
- **WHEN** native allocation, drawing, or pixel validation fails
- **THEN** every resource acquired by SuperDesktop is released and no malformed buffer crosses the platform boundary

### Requirement: Taskbar buttons must show application icons
Each visible taskbar task SHALL render a native application icon resolved from its live window or executable while retaining the readable label and existing action target.

#### Scenario: Window supplies an icon
- **WHEN** `WM_GETICON` returns a usable icon within the timeout
- **THEN** the taskbar renders that icon before the task label

#### Scenario: Window icon is unavailable
- **WHEN** window and class icon lookup produce no usable icon
- **THEN** the taskbar attempts the executable Shell icon and retains a generic fallback if it also fails

#### Scenario: Refresh repeats
- **WHEN** unchanged tasks are reconciled repeatedly
- **THEN** cached owned pixels are reused and cache entries absent from the live identity set are pruned

### Requirement: Desktop items must show Shell icons
Each visible desktop item SHALL render the Windows Shell icon associated with its canonical path while preserving its label, selection, and file operation behavior.

#### Scenario: File folder or shortcut is visible
- **WHEN** a desktop namespace entry has an accessible canonical path
- **THEN** the desktop renders the Shell-provided icon centered above its label

#### Scenario: Item changes or disappears
- **WHEN** desktop reconciliation observes a changed identity set
- **THEN** new icons are resolved as needed and stale cache entries are removed

#### Scenario: Icon resolution fails
- **WHEN** an item path is inaccessible or its icon cannot be converted
- **THEN** the item remains visible and interactive with a stable generic fallback

### Requirement: Native icon parity must be release-gated
The production build SHALL pass automated lifetime/model/rendering checks and headful Windows evidence before packaging is accepted.

#### Scenario: Repeated extraction validation
- **WHEN** native icons are extracted repeatedly in a validation run
- **THEN** process GDI-object usage returns to a stable level after owned resources are released

#### Scenario: Headful validation
- **WHEN** production SuperDesktop is captured on the active Windows host
- **THEN** recognizable non-placeholder icons appear for taskbar applications and desktop Shell items at the host DPI

### Requirement: Icon color and GPU compression must preserve visible identity
The UI SHALL convert owned RGBA pixels to the GPUI Windows BGRA upload contract without red/blue channel bias and SHALL use bounded direct BC7 icon uploads only when the active adapter supports them.

#### Scenario: BGRA fallback is used
- **WHEN** BC7 sampling is unavailable or encoding is rejected
- **THEN** a red RGBA source reaches the atlas as BGRA bytes and remains visibly red

#### Scenario: BC7 hardware path is used
- **WHEN** the active adapter reports BC7 texture sampling support
- **THEN** each distinct icon is encoded to complete 4x4 block rows, uploaded directly, cached within the icon budget, and observed in production telemetry

#### Scenario: Production colors are inspected
- **WHEN** known blue and orange application icons are captured
- **THEN** Discord and Steam remain blue while Firefox retains its orange and blue colors without channel inversion or unmatched sRGB darkening
