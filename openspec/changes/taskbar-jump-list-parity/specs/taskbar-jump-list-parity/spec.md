## ADDED Requirements

### Requirement: Task context popup exclusivity
SuperDesktop SHALL make an owned task Jump List mutually exclusive with any pending or visible task preview.

#### Scenario: Right-click while preview is visible
- **WHEN** the user right-clicks a task button while its preview surface is visible
- **THEN** SuperDesktop removes the preview before exposing the Jump List
- **AND** no preview surface remains visible concurrently with the Jump List

#### Scenario: Right-click while hover open is pending
- **WHEN** the user right-clicks a task button before the hover delay expires
- **THEN** SuperDesktop invalidates the pending preview generation
- **AND** the preview does not appear after the Jump List opens

#### Scenario: Context resolution fails
- **WHEN** the task snapshot, selected identity, or provider request cannot be resolved
- **THEN** SuperDesktop still cancels and removes the preview before returning

### Requirement: Application-owned Jump List destinations
SuperDesktop MUST NOT label global or unverified Windows Recent/Frequent entries as destinations of the selected application.

#### Scenario: No verified application destination source
- **WHEN** SuperDesktop cannot prove that Recent or Frequent destinations belong to the selected application
- **THEN** the Jump List omits the corresponding groups
- **AND** unrelated URI, shortcut, or document labels are not displayed

#### Scenario: Provider is unavailable
- **WHEN** the Jump List provider fails or times out
- **THEN** SuperDesktop displays the locally composed taskbar actions
- **AND** does not synthesize Recent or Frequent entries

### Requirement: Explorer-aligned required commands
Every running task Jump List SHALL expose the applicable pin state and exactly one terminal close action in the bottom command area.

#### Scenario: Unpinned single-window application
- **WHEN** an unpinned application has one eligible task window
- **THEN** the bottom commands include `Pin to taskbar` followed by `Close window`
- **AND** `Close all windows` is absent

#### Scenario: Pinned grouped application
- **WHEN** a pinned application has multiple eligible task windows
- **THEN** the bottom commands include `Unpin from taskbar` followed by `Close all windows`
- **AND** `Close window` is absent

#### Scenario: Local action presentation
- **WHEN** the Jump List renders its local taskbar commands
- **THEN** it does not render a synthetic `Actions` heading
- **AND** the pin/unpin command precedes the final close command

### Requirement: Exact command execution
Task Jump List commands SHALL mutate only the captured eligible window identity or captured application window set and SHALL report failures without claiming success.

#### Scenario: Single-window close
- **WHEN** the user invokes `Close window`
- **THEN** SuperDesktop validates the captured HWND, process ID, and window identity before posting close

#### Scenario: Group close
- **WHEN** the user invokes `Close all windows`
- **THEN** SuperDesktop applies close to the captured application window set
- **AND** reports rejection if any required close action fails

#### Scenario: Pin state mutation
- **WHEN** the user invokes pin or unpin
- **THEN** SuperDesktop changes only the selected application identity in persisted taskbar pins
- **AND** reports rejection if persistence fails

### Requirement: Headful regression evidence
The focused UTIT case SHALL prove popup exclusivity, required commands, stale-timer rejection, and exact window actions using an interactive Windows session.

#### Scenario: Focused UTIT passes
- **WHEN** the `gui-taskbar-window-actions` case completes
- **THEN** its artifact records no preview during the Jump List's lifetime
- **AND** records the applicable pin and close commands
- **AND** records successful minimize, maximize, and close observations
- **AND** `validate-report` accepts the run report
