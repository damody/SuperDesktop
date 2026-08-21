# Built-in screen snip clipboard completion

## Goal

When SuperDesktop owns the shell and handles `Win+Shift+S`, Windows' built-in Snipping Tool shall finish publishing the selected image to the clipboard before SuperDesktop removes its temporary Explorer protocol broker.

## Root cause

The current implementation treats disappearance of `SnipOverlayRootWindow` as completion and immediately shuts down the temporary Explorer process. The overlay disappears when selection ends, but Snipping Tool publishes the image to the clipboard asynchronously afterward. Removing the broker at that boundary can interrupt the post-capture path, while the existing UTIT only presses Escape and never verifies a real captured image.

## Design

Before protocol activation, SuperDesktop records `GetClipboardSequenceNumber`. It launches the fixed `ms-screenclip:///?source=HotKey` URI through the existing verified temporary Explorer broker and waits for the built-in overlay. After the overlay disappears, it polls for a changed clipboard sequence, at least one Windows image format (`CF_BITMAP`, `CF_DIB`, or `CF_DIBV5`), and a non-empty `GetClipboardData` handle for up to ten seconds. The handle check is required because Snipping Tool advertises delayed-rendering formats before their payload is usable.

If an image appears, SuperDesktop records successful clipboard completion, then removes the temporary Explorer broker. If the overlay was cancelled and the clipboard remains unchanged, it uses a short cancellation grace period and cleans up without reporting a false capture success. If the sequence changes without an image or no image arrives within the bounded completion deadline after a selection, the command returns a precise console error and still cleans up the broker. Existing Explorer processes are never stopped.

The clipboard check opens the clipboard only long enough to ask Windows for a non-empty delayed-rendering handle, then closes it immediately. SuperDesktop does not lock the handle, read pixels, replace, transform, or retain the payload.

## Verification

- Unit tests cover unchanged clipboard, unrelated sequence changes, bitmap/DIB/DIBV5 availability, cancellation, completion, and timeout state transitions.
- Physical UTIT suppresses Explorer, triggers the real shortcut, drags a deterministic rectangle, and verifies that clipboard sequence changes to an image payload with non-zero dimensions.
- A second physical run verifies Escape cancellation leaves the clipboard unchanged and still removes the temporary broker.
- Both runs verify the Microsoft Store Snipping Tool identity, fixed URI, SuperDesktop survival, Explorer absence after completion, and absence of panic/error signatures.
- Workspace tests, Clippy with warnings denied, release build, strict OpenSpec validation, and installer hash comparison remain blocking.

## Non-goals

The change does not implement a custom screenshot engine, force-enable user Snipping Tool settings, read or persist captured pixels as evidence, or terminate an Explorer process that existed before the shortcut.
