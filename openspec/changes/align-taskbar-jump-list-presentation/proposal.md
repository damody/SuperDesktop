## Why

Owned Jump Lists render every ordinary command as a bullet and omit group headings, unlike Windows grouped Recent/Frequent/Tasks/Actions presentation.

## What Changes

- Add 24 DIP accessible headings for each non-empty group.
- Replace generic bullets with typed semantic glyphs.
- Include headings in content-sized geometry while preserving keyboard/action behavior.
- Extend UTIT/source/screenshot gates; keep unarchived.
- Reserve the dynamic notification-area width so one-row task buttons never overlap tray controls.

## Capabilities

### New Capabilities

- `taskbar-jump-list-presentation`: Grouped headings, semantic glyph column and automated presentation admission.

### Modified Capabilities

None.

## Impact

Changes taskbar UI, Jump List geometry tests and UTIT assertions only. No command, protocol, privilege, persistence or Explorer dependency changes.
