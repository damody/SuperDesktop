## Why

Applications depend on a notification area for persistent status and commands, but hosting arbitrary tray clients directly would expand the shell process failure boundary.

## What Changes

- Add an isolated notification-area host and versioned icon/event protocol.
- Add visible/overflow layouts, mouse and keyboard activation, tooltips, and DPI-aware icons.
- Add bounded event delivery, client cleanup, restart reconciliation, and observable health states.

## Capabilities

### New Capabilities

- `notification-area-host`: Tray-client compatibility, isolation, layout, interaction, cleanup, and recovery behavior.

### Modified Capabilities

None.

## Impact

- Affects Windows message adapters, taskbar UI, provider protocol, process supervision, accessibility, tests, and evidence.
