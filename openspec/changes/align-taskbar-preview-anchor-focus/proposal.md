## Why

Owned taskbar previews currently open at the monitor center and always activate themselves. Windows anchors previews above the source task and ordinary hover does not interrupt the foreground application, so both placement and focus are conspicuous parity gaps.

## What Changes

- Distinguish hover-opened and click-opened task previews with a typed source policy.
- Anchor content-sized previews to the source pointer/task position and clamp them to the source monitor across DPI and negative origins.
- Keep hover previews non-activating and non-focusing while retaining pointer interaction.
- Preserve click and keyboard activation/focus behavior.
- Extend the Explorer-free UTIT case with foreground preservation and exact placement evidence.
- Run full source, test, release, traceability, and installer gates without archiving.

## Capabilities

### New Capabilities

- `taskbar-preview-anchor-focus`: Defines source-relative geometry, focus policy, fallback behavior, and automated admission.

### Modified Capabilities

None.

## Impact

Changes `superdesktop-app`, `taskbar-ui`, the existing hover-preview UTIT capture, OpenSpec evidence, and packages. It adds no protocol, persistence, privilege, undocumented API, or Explorer dependency.
