## ADDED Requirements

### Requirement: Tasks without real icons must keep readable labels
The taskbar SHALL display the complete task name, including group count when applicable, whenever a task has no real renderable icon. It MUST NOT substitute the first character of the name as a pseudo-icon.

#### Scenario: Stored labels-disabled setting without icon
- **WHEN** `show_labels=false` is loaded and a task has no renderable icon
- **THEN** the task displays its full readable name rather than its first character

#### Scenario: Traditional Chinese title
- **WHEN** a task without an icon has a Traditional Chinese title
- **THEN** the visible fallback preserves the full title for layout instead of selecting one Unicode scalar

#### Scenario: Grouped task
- **WHEN** a task group contains more than one window
- **THEN** the readable label includes the group count suffix

### Requirement: Task labels must truncate inside their own bounded region
Each visible task label SHALL own a shrinkable single-line region and SHALL use an ellipsis when its shaped text exceeds the available region. Badges and progress overlays MUST NOT collapse the label to an arbitrary single character.

#### Scenario: Long title in a narrow slot
- **WHEN** a long task title exceeds the task button's available label width
- **THEN** GPUI renders a single-line ellipsis while retaining more than an invented first-character placeholder whenever space permits

#### Scenario: Badge shares task button
- **WHEN** a task contains a badge
- **THEN** the badge remains visible and the label shrinks within its own minimum-width-zero region

### Requirement: New and partial settings must default to readable labels
`TaskbarSettings::default()` and JSON decoding with a missing `show_labels` field SHALL set `show_labels=true`; an explicitly stored boolean SHALL remain round-trippable.

#### Scenario: New settings
- **WHEN** no settings file exists
- **THEN** the generated taskbar settings enable readable labels

#### Scenario: Legacy partial taskbar object
- **WHEN** a settings document contains a taskbar object without `show_labels`
- **THEN** decoding enables readable labels without reporting structural failure

#### Scenario: Explicit false remains compatible
- **WHEN** a settings document explicitly contains `show_labels=false`
- **THEN** decoding and encoding preserve the boolean while the no-icon render fallback remains readable
