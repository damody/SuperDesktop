## Why

The notification-area up-chevron is conditional on the overflow partition being non-empty, so it disappears whenever current icons fit the five visible slots. The requested stable entry point for viewing all current tray icons is therefore missing in common taskbar states.

## What Changes

- Always reserve and render the localized up-chevron in the notification area.
- Forward a single snapshot of all current notification-area nodes to the owned popup for pointer and keyboard activation.
- Open a truthful empty-state popup when no icons are registered instead of silently doing nothing.
- Use theme tokens for the chevron and add focused/headful evidence for empty, populated, dark, and high-contrast states.

## Capabilities

### New Capabilities

- `taskbar-show-all-tray-icons`: Defines the stable notification-area chevron, complete current-icon popup snapshot, empty state, accessibility, and theme behavior.

### Modified Capabilities

None. Existing overflow geometry/interaction changes remain historical; no archived base capability currently owns the chevron admission contract.

## Impact

Affected areas are `taskbar-ui` taskbar/overflow rendering, `superdesktop-app` popup admission, focused tests, headful fixtures, and evidence. Notification ingestion, placement policy, settings, system-status controls, and public APIs remain unchanged.
