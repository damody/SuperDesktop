## Why

Desktop operations, search, taskbar extensions, tray icons, and virtual desktops need stable cross-crate contracts and fault isolation before they can be implemented independently.

## What Changes

- Add versioned provider request, response, capability, cancellation, timeout, and health contracts.
- Add shared DTOs for shell items, commands, search results, notifications, task previews, and virtual desktops.
- Add an out-of-process provider host with bounded IPC, crash containment, and deterministic fallback behavior.
- Add contract fixtures, compatibility checks, and telemetry correlation.

## Capabilities

### New Capabilities

- `shell-provider-contracts`: Versioned provider protocol and shared shell DTO requirements.
- `shell-provider-host`: Isolated provider execution, lifecycle, timeout, cancellation, and fault-containment requirements.

### Modified Capabilities

None.

## Impact

- Adds shared protocol and provider-host crates/binaries.
- Affects shell-core integration, serialization dependencies, process supervision, tests, and evidence schemas.
