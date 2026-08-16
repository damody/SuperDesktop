## Why

The GPUI desktop can display and activate items but lacks the normal file-management operations required for daily desktop use.

## What Changes

- Add refresh, rename, recycle/delete, sort, align, and position persistence.
- Add copy/move drag-and-drop with progress, cancellation, collision handling, and reconciliation.
- Route filesystem effects through typed commands and report partial failures without corrupting UI state.

## Capabilities

### New Capabilities

- `desktop-file-operations`: Desktop mutation, layout, transfer, cancellation, and reconciliation behavior.

### Modified Capabilities

None.

## Impact

- Affects desktop reducers, GPUI input/render paths, filesystem adapters, recycle-bin integration, settings, tests, and evidence.
