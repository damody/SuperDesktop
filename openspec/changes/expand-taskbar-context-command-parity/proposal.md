## Why

The owned taskbar context menu exposes only three commands, omitting common Windows controls for Search presentation, the Task View button, and Show desktop. Existing UTIT checks only the lock row, so incomplete command sets and incorrect menu geometry can pass unnoticed.

## What Changes

- Expand the owned context menu to six ordered, keyboard-accessible command rows.
- Add typed commands for cycling Search presentation, toggling Task View, and invoking the owned Show desktop cycle.
- Persist Search, Task View, and Lock mutations through the existing revisioned settings store without partial live-state updates.
- Render current checked state and localized labels while retaining Windows-style 220 DIP geometry.
- Extend Explorer-free UTIT to verify ordered menu items, checked accessibility names, popup bounds, content fit, and the existing lock action.
- Keep every UI path independent of `explorer.exe` and leave the change unarchived.

## Capabilities

### New Capabilities

- `taskbar-context-command-parity`: Defines the owned taskbar context menu command set, behavior, geometry, accessibility state, persistence, and automated evidence.

### Modified Capabilities

None.

## Impact

Affected areas are `taskbar-ui` context menu model/view, `superdesktop-app` composition and settings routing, the Explorer-free resize/lock capture script, tests, and evidence. No dependency, public protocol, installer, or persisted schema change is introduced.
