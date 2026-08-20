## Why

SuperDesktop owns the shell but currently ignores a standalone Windows logo key, so users cannot open or close the owned Start menu with the primary Windows keyboard gesture. The fix is required now to restore the Windows-documented Start toggle while preserving every existing Win-key chord.

## What Changes

- Recognize standalone left and right Windows-key gestures in the shell-scoped low-level keyboard hook.
- Emit one Start toggle only when the candidate Windows key is released without another key having been pressed.
- Cancel the standalone gesture when a chord, repeat, second Windows key, or mismatched release makes it ineligible.
- Route the action through the same owned Start callback used by the taskbar button, including open-on-first-gesture and close-on-second-gesture behavior.
- Add reducer, routing-contract, and headful UTIT coverage with shell/Explorer recovery evidence.

## Capabilities

### New Capabilities

- `owned-win-key-start-toggle`: Defines standalone Windows-key recognition and owned Start open/close behavior in SuperDesktop shell mode.

### Modified Capabilities

None.

## Impact

The change affects the Windows low-level shell hotkey reducer and bounded action queue in `platform-win`, the GPUI shell action router in `superdesktop-app`, Windows-only tests, UTIT automation, and release/installer verification. It adds no dependency, registry migration, public API, or Explorer-hosted fallback.
