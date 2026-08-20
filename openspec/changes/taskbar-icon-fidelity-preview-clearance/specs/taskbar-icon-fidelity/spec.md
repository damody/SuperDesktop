## ADDED Requirements

### Requirement: DPI-appropriate task icon source
SuperDesktop SHALL request a task icon source sized for a 24 DIP icon at the highest active monitor DPI, rounded up and clamped to 32–64 physical pixels.

#### Scenario: Standard DPI
- **WHEN** all active monitors report 96 DPI
- **THEN** SuperDesktop requests a 32 px source and renders it at 24 DIP

#### Scenario: High DPI
- **WHEN** an active monitor requires more than 32 physical pixels for 24 DIP
- **THEN** SuperDesktop requests at least the rounded-up physical requirement without exceeding 64 px

#### Scenario: Invalid DPI
- **WHEN** monitor DPI is zero or unavailable
- **THEN** SuperDesktop uses the 96 DPI fallback without panicking

### Requirement: Highest-quality Windows icon fallback order
SuperDesktop SHALL prefer a size-matched executable resource and large window/class icons before accepting small icon variants, while preserving recoverable shell fallbacks.

#### Scenario: Size-matched executable resource exists
- **WHEN** the executable exposes an icon at or near the requested edge
- **THEN** SuperDesktop converts that owned resource and releases its `HICON` after conversion

#### Scenario: Executable extraction fails
- **WHEN** the path is unrepresentable, protected, stale, or exposes no requested resource
- **THEN** SuperDesktop continues through large, small, class, and shell fallbacks without hiding the task or panicking

### Requirement: Lossless small task icon upload
SuperDesktop SHALL upload task icons no larger than 64 px per dimension as lossless BGRA data rather than BC7-compressed data.

#### Scenario: Detailed small icon
- **WHEN** a valid 32–64 px task icon contains thin strokes or partial alpha
- **THEN** the render image preserves the source BGRA bytes and dimensions before normal compositor scaling

#### Scenario: Invalid icon payload
- **WHEN** icon dimensions and pixel length disagree
- **THEN** SuperDesktop rejects that payload through the existing recoverable placeholder path without panicking
