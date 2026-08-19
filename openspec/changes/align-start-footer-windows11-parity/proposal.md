## Why

The owned Start footer renders a Settings gear beside Power, while Windows 11 shows only the account control and Power. The current live UTIT incorrectly requires Settings and therefore certifies this visible mismatch.

## What Changes

- Remove the Start footer Settings button while preserving Settings discovery through Start search/pins and owned taskbar settings.
- Keep one 40 by 40 DIP Power action, account control, existing footer height, and owned power popup.
- Change UTIT to reject a footer Settings control, require exactly one footer action, and measure Power size/right alignment.
- Retain Explorer-free power interaction, recovery, accessibility, and full quality gates.
- Keep the change unarchived.

## Capabilities

### New Capabilities

- `start-footer-windows11-parity`: Defines the Windows 11 account-and-Power footer composition and automated geometry evidence.

### Modified Capabilities

None.

## Impact

Affected areas are `taskbar-ui` Start rendering/source contracts and the live Start capture script/evidence. No public protocol, settings schema, dependency, installer, or power behavior changes.
