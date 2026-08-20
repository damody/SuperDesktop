## Why

SuperDesktop does not currently prove or consistently preserve distinct left- and right-click behavior for input language, volume, notification-area icons, and taskbar applications. Child pointer events can fall through to taskbar-background handling, while UTIT accepts incomplete evidence such as any NotifyIcon callback rather than the exact expected callback.

## What Changes

- Define one explicit pointer contract for primary, contextual, keyboard, and UIA activation across the four affected taskbar control families.
- Stop handled child events from opening or replacing the intended popup with the taskbar background menu.
- Add owned, fixed-target context actions for input-language and volume controls.
- Preserve Explorer-like application-button left-click state reduction and right-click Jump List behavior.
- Require exact Activate-versus-Context delivery for visible and overflow notification icons.
- Extend UTIT with real-pointer, Explorer-free headful cases and exact JSON/trace assertions for both mouse buttons.
- Keep popup toggle, exclusivity, focus-loss, Escape, positioning, watchdog, and recovery behavior observable.

## Capabilities

### New Capabilities

- `taskbar-pointer-interaction-parity`: Explorer-aligned left/right pointer routing, popup ownership, notification callback delivery, application-button behavior, and UTIT evidence.

### Modified Capabilities

None. Relevant earlier changes are not yet archived as baseline capabilities; this change introduces one integrated parity contract without weakening them.

## Impact

Affected code includes `taskbar-ui` input rendering and popup views, `superdesktop-app` popup composition/lifetimes and traces, NotifyIcon compatibility assertions, UTIT catalog entries, and headful PowerShell fixtures. No public IPC shape, dependency, installer behavior, or caller-controlled launch surface is added.
