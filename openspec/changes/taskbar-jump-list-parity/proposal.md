## Why

SuperDesktop currently allows a task hover preview to remain visible when the user opens the task's right-click Jump List, and it labels the contents of the global Windows Recent folder as application-specific history. This produces overlapping owned popups and unrelated entries that diverge from File Explorer's native taskbar behavior.

## What Changes

- Make task preview and task Jump List surfaces mutually exclusive.
- Cancel pending hover timers and close an existing preview before opening a Jump List.
- Prevent a stale hover timer from reopening a preview while the Jump List is displayed.
- Stop exposing unscoped global Recent/Frequent items as if they belonged to the selected application.
- Keep provider tasks only when they are valid for the selected application.
- Place required taskbar commands at the bottom without a synthetic `Actions` heading.
- Always expose the applicable pin/unpin command and exactly one single-window or grouped close command.
- Preserve exact-identity minimize, maximize, pin, and close execution.
- Extend headful UTIT coverage for popup exclusivity, command presence/order, stale-timer rejection, and exact window actions.

## Capabilities

### New Capabilities

- `taskbar-jump-list-parity`: Defines Explorer-aligned task right-click popup exclusivity, application-scoped Jump List content, required bottom commands, and verification behavior.

### Modified Capabilities

None.

## Impact

- `taskbar-ui`: hover-controller cancellation, Jump List grouping and rendering, and unit tests.
- `superdesktop-app`: task-context popup coordination, application-specific composition, command ordering, and action tracing.
- `platform-win` / `shell-provider-host`: removal or fail-closed suppression of the incorrect global Recent-folder enumeration path.
- `superdesktop-utit` and PowerShell capture scripts: physical right-click and delayed-preview regression evidence.
- No public protocol break is required; existing response fields may remain empty when application ownership cannot be proven.
