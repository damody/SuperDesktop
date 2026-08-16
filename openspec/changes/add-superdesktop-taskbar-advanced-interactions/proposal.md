## Why

Basic task switching is complete, but common Windows taskbar workflows such as previews, grouped windows, Jump Lists, and persisted pinning remain absent.

## What Changes

- Add live thumbnails, grouped-window flyouts, progress/attention overlays, and richer activation rules.
- Add Jump Lists and recent/frequent destinations through safe provider contracts.
- Persist pin order and taskbar preferences with reconciliation after external window changes.

## Capabilities

### New Capabilities

- `taskbar-advanced-interactions`: Preview, grouping, Jump List, overlay, and persistence behavior.

### Modified Capabilities

None.

## Impact

- Affects taskbar reducers, GPUI overlays, DWM/window adapters, provider host, settings, accessibility, tests, and evidence.
