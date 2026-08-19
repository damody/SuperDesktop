## Why

SuperDesktop owns the taskbar but does not provide Windows' far-right Show desktop corner, leaving a common taskbar interaction unavailable when Explorer is removed. The replacement shell needs an owned, reversible implementation that matches Windows presentation and never delegates to Explorer or a synthetic Win+D shortcut.

## What Changes

- Add a Windows 11-style full-height Show desktop corner at the taskbar's far-right monitor edge.
- Add a pure reversible session model that distinguishes first-click minimize from second-click restore.
- Minimize only eligible windows that were not already minimized and restore only fresh exact-identity matches.
- Add pointer, Enter, Space, UIA, localization, light/dark/high-contrast, multi-row, and 175% DPI behavior.
- Add Explorer-absent headful, safety, traceability, release, and standalone/combined installer evidence without archiving the change.

## Capabilities

### New Capabilities

- `owned-show-desktop-corner`: Defines owned window admission, reversible behavior, far-edge presentation, accessibility, safety, and Explorer-free evidence.

### Modified Capabilities

None.

## Impact

Changes `taskbar-ui`, `platform-win`, `superdesktop-app`, focused capture scripts, OpenSpec evidence, and packaged binaries. It adds no wire protocol, settings schema, registry mutation, undocumented Windows API, Explorer UI, shell URI, or SuperExplorer source dependency.
