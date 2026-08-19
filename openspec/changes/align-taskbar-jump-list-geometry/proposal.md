## Why

Owned task Jump Lists are fixed 360×480 DIP and screen-centered, producing large empty panels disconnected from the source task and incorrect taskbar spacing.

## What Changes

- Anchor Jump Lists to the source pointer/task and matching preview/shell taskbar.
- Size height from actual entries/groups with a 480 DIP cap.
- Add DPI/edge/row/mode tests and real taskbar UTIT geometry evidence.
- Keep all behavior owned and the change unarchived.

## Capabilities

### New Capabilities

- `taskbar-jump-list-geometry`: Source anchoring, content height, containment and automated admission.

### Modified Capabilities

None.

## Impact

Changes app composition/geometry, taskbar UTIT capture and evidence. No Explorer/system Jump List call, protocol, privilege or persistence change.
