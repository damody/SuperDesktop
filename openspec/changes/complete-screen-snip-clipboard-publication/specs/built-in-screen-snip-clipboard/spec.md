## ADDED Requirements

### Requirement: Built-in screen snip completes through the Windows clipboard
The system SHALL activate the Windows-registered built-in image-snipping overlay through the fixed `ms-screenclip:///?source=HotKey` protocol. A completed capture SHALL be successful only after the clipboard sequence changes, at least one native image format is available, and Windows returns a non-empty clipboard data handle. The system MUST close the clipboard immediately and MUST NOT lock or read the payload.

#### Scenario: Rectangular selection publishes an image
- **WHEN** the user invokes Win+Shift+S and completes a rectangular selection
- **THEN** the clipboard sequence changes, `CF_BITMAP`, `CF_DIB`, or `CF_DIBV5` becomes available with a non-empty data handle, and the command reports a captured terminal

#### Scenario: Delayed format is advertised before payload materialization
- **WHEN** Snipping Tool advertises an image format but `GetClipboardData` still returns an empty handle
- **THEN** the system retains its temporary broker and continues waiting instead of reporting capture success

#### Scenario: Clipboard changes to non-image data
- **WHEN** the overlay closes after capture intent but the clipboard sequence changes only to non-image data
- **THEN** the system continues waiting for Snipping Tool's image until the bounded deadline and does not report capture success from the unrelated update

### Requirement: Temporary Explorer broker remains until Snipping Tool reaches a terminal
When Explorer was absent before the shortcut, the system SHALL retain the trusted temporary Explorer protocol broker until capture, explicit cancellation, or bounded failure is determined. It SHALL clean up only that temporary broker after the terminal and MUST NOT terminate a pre-existing Explorer process.

#### Scenario: Explorer-free capture completes
- **WHEN** SuperDesktop creates a temporary Explorer broker and Snipping Tool publishes an image
- **THEN** the broker remains present through clipboard publication and is removed afterward

#### Scenario: Explorer existed before invocation
- **WHEN** trusted Explorer was already present before Win+Shift+S
- **THEN** the system performs no Explorer cleanup after capture or cancellation

### Requirement: Cancellation and publication failure remain distinct
The system SHALL report explicit Escape cancellation separately from capture. A selection attempt or ambiguous overlay dismissal that does not publish an image within ten seconds SHALL return a console-visible publication error, and every terminal SHALL perform bounded broker cleanup.

#### Scenario: User presses Escape
- **WHEN** Escape dismisses the overlay before capture intent and the clipboard remains unchanged during the cancellation grace
- **THEN** the command reports cancelled, leaves the clipboard unchanged, and removes only its temporary broker

#### Scenario: Selection does not publish an image
- **WHEN** capture intent is observed but no changed image clipboard payload appears within ten seconds
- **THEN** the command reports a clipboard publication timeout and still removes its temporary broker

### Requirement: Physical evidence verifies clipboard outcome without retaining pixels
The focused physical test SHALL perform a deterministic real selection and verify clipboard sequence, image format, and non-zero image dimensions. Evidence MUST contain only result metadata and content-independent hashes and MUST NOT persist or embed captured pixels.

#### Scenario: Two clean physical capture runs
- **WHEN** the admitted release candidate runs the Explorer-free Win+Shift+S case twice
- **THEN** both runs verify a non-zero clipboard image, built-in Microsoft package identity, broker cleanup, Explorer recovery, SuperDesktop survival, and absence of crash/error signatures without saving the screenshot

#### Scenario: Physical cancellation run
- **WHEN** the same candidate invokes the overlay and presses Escape
- **THEN** the clipboard sequence remains unchanged, cancellation is traced, broker cleanup completes, and no screenshot artifact is written
