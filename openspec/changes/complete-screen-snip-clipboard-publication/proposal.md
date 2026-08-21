## Why

SuperDesktop currently removes the temporary Explorer protocol broker as soon as the built-in Snipping Tool overlay disappears. A completed selection publishes its image asynchronously after that point, so the screenshot can be lost before it reaches the clipboard even though the overlay appeared correctly.

## What Changes

- Observe the clipboard sequence and native image formats around the fixed built-in screen-snipping activation.
- Keep a SuperDesktop-created Explorer broker alive until Snipping Tool publishes the selected image or a bounded completion deadline expires.
- Distinguish successful capture, user cancellation, unrelated clipboard updates, and failed image publication.
- Preserve pre-existing Explorer processes and always clean up only the temporary broker owned by SuperDesktop.
- Upgrade physical UTIT from Escape-only coverage to deterministic rectangle selection and clipboard-image verification without persisting captured pixels.
- Retain the Microsoft Store Snipping Tool identity, fixed URI, no custom capture fallback, and console-visible failures.

## Capabilities

### New Capabilities

- `built-in-screen-snip-clipboard`: Defines built-in overlay activation, clipboard image completion, cancellation, timeout, broker ownership, and privacy-safe physical evidence.

### Modified Capabilities

None.

## Impact

The change affects `platform-win` screen-snipping lifecycle code, the SuperDesktop hotkey worker trace/error contract, the focused Win+Shift+S capture script, UTIT assertions, release binaries, and installer evidence. It adds no custom screenshot engine and does not store clipboard image content in committed evidence.
