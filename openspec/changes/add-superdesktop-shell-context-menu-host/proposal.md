## Why

Windows shell context menus may load third-party extensions, so SuperDesktop needs compatible commands without allowing unstable extension code into the GPUI process.

## What Changes

- Add an out-of-process context-menu provider with capability probing and bounded execution.
- Sanitize menu descriptors before GPUI rendering and preserve stable command identity.
- Add timeouts, cancellation, crash recovery, elevation markers, and built-in fallback commands.

## Capabilities

### New Capabilities

- `shell-context-menu-host`: Safe enumeration, rendering descriptors, invocation, isolation, and fallback behavior.

### Modified Capabilities

None.

## Impact

- Affects provider protocol, Windows shell adapters, desktop/taskbar menus, GPUI overlays, supervision, tests, and evidence.
