## Why

Virtual desktop switching and window movement are common Windows workflows, but the available APIs vary by OS build and must not compromise shell stability.

## What Changes

- Add runtime capability probing and a typed virtual-desktop service.
- Add desktop enumeration, switching, creation/removal where supported, and moving windows between desktops.
- Add task-view UI, keyboard navigation, stale-state reconciliation, and explicit unavailable states.

## Capabilities

### New Capabilities

- `virtual-desktop-control`: Capability-gated desktop enumeration, switching, mutation, window movement, and fallback behavior.

### Modified Capabilities

None.

## Impact

- Affects Windows capability probes, shell-core state, taskbar UI, window tracking, tests, and evidence.
