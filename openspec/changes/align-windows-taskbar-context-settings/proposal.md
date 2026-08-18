## Why

SuperDesktop currently provides only application-button Jump Lists and has no owned empty-taskbar context menu or Windows-like taskbar settings surface. This leaves a major interaction and visual gap when Explorer is removed, and existing taskbar preferences cannot be discovered or changed through the product UI.

## What Changes

- Add an owned Windows 11-style context menu for empty taskbar space with Task Manager and Taskbar settings commands.
- Restyle application Jump Lists to the same Windows 11 command-surface geometry and interaction states.
- Add an owned **Personalization > Taskbar** settings window with grouped cards, switches, dropdowns, supporting text, keyboard navigation, UIA, theme, high-contrast, and DPI behavior.
- Add bounded taskbar settings for Search visibility, Task View visibility, and left/center alignment; connect existing labels, grouping, previews, monitor, and row settings.
- Apply successful settings saves immediately to every taskbar surface and preserve the prior state on save failure.
- Show unsupported Windows inbox surfaces as disabled with truthful explanations instead of invoking Explorer or Windows Settings.
- Add release/headful verification and package the behavior without adding another binary.

## Capabilities

### New Capabilities

- `windows-taskbar-context-settings`: Owned taskbar context menus, application command styling, taskbar settings contracts, persistence, live composition, accessibility, and Windows 11 visual parity.

### Modified Capabilities

None.

## Impact

Affected components are `settings-store`, `taskbar-ui`, `superdesktop-app`, GPUI window composition, settings serialization/migration tests, headful verification scripts, OpenSpec evidence, and standalone/combined installer verification. The change adds no external dependency and must remain mode-independent except that unsupported Explorer-owned surfaces stay unavailable in both Preview and Shell modes.
