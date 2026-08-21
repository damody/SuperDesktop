## Why

The owned Start menu currently has a fixed 640×720 DIP window and explicitly disables native resizing. Users cannot adapt it to their screen or content, and no Start geometry is retained between openings or application restarts.

## What Changes

- Enable native edge resizing for the titlebar-free owned Start popup.
- Add optional logical width and height to persisted Start settings with backward-compatible defaults.
- Clamp restored and live sizes to usable per-monitor work-area limits across DPI, taskbar rows, preview, and owned-shell modes.
- Debounce resize persistence and flush the latest size on deactivation and explicit close paths.
- Reopen Start at the saved size while deriving position from the current left/center taskbar alignment.
- Add settings, geometry, UI lifecycle, physical resize/reopen/restart, release, and installer evidence.

## Capabilities

### New Capabilities

- `owned-start-menu-sizing`: Defines native resize behavior, logical size persistence, monitor/DPI clamping, close/deactivation flush, and physical restart recovery.

### Modified Capabilities

None.

## Impact

The change affects settings-store Start schema encoding, taskbar-ui Start view lifecycle, superdesktop-app Start geometry/window creation and persistence callbacks, focused UTIT scripts, release binaries, and installer evidence. It does not persist Start position or change taskbar alignment semantics.
