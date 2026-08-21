## Context

SuperDesktop launches the fixed Windows `ms-screenclip:///?source=HotKey` protocol. When Explorer is absent, it temporarily starts a trusted Explorer shell so Windows can resolve the protocol, waits for `SnipOverlayRootWindow`, then removes that Explorer as soon as the overlay disappears. Snipping Tool performs clipboard publication asynchronously after selection, so overlay disappearance is not a valid success terminal. The current focused test verifies only Escape dismissal and therefore cannot detect the missing screenshot.

The implementation must keep using Microsoft's registered Snipping Tool, preserve pre-existing Explorer processes, avoid reading or persisting captured pixels, and remain bounded if Snipping Tool or the clipboard stalls.

## Goals / Non-Goals

**Goals:**

- Treat a changed clipboard sequence plus an available native image format as successful capture completion.
- Keep only the SuperDesktop-created protocol broker alive until capture completion, cancellation, or timeout.
- Distinguish captured, explicitly cancelled, unrelated clipboard update, and failed publication outcomes.
- Verify a real rectangular selection reaches the clipboard in Explorer-free mode.
- Keep physical evidence privacy-safe by recording metadata and hashes only, never screenshot pixels.

**Non-Goals:**

- Implementing a custom screenshot or clipboard writer.
- Enabling or changing Snipping Tool user settings.
- Reading, decoding, saving, or uploading the captured image in product code or evidence.
- Terminating Explorer when it existed before the shortcut.

## Decisions

### Use clipboard sequence and materialized image payload as the completion fence

The worker snapshots `GetClipboardSequenceNumber` before activation. After the overlay closes, capture succeeds only when the sequence differs, `IsClipboardFormatAvailable` reports `CF_BITMAP`, `CF_DIB`, or `CF_DIBV5`, and a brief `OpenClipboard` / `GetClipboardData` probe returns a non-empty handle. Physical evidence showed that Snipping Tool advertises delayed-rendering formats before the payload is usable. SuperDesktop closes the clipboard immediately and never locks or reads the handle. This is preferred over a fixed delay, which cannot prove completion.

### Track capture intent and explicit cancellation while the overlay is visible

The overlay polling loop records left-button activity and Escape activity. Explicit Escape without capture intent yields a cancelled outcome after a short race grace. A selection attempt or ambiguous dismissal waits up to ten seconds for an image publication. The API returns `Captured` or `Cancelled`; publication timeout remains an error. This prevents a ten-second delay for ordinary cancellation without treating an unfinished selection as success.

### Cleanup follows the outcome fence

The temporary Explorer broker is cleaned after captured/cancelled/error terminal determination. Every error path attempts cleanup and combines cleanup failure with the primary error. A pre-existing Explorer bypasses broker creation and cleanup but still uses the clipboard completion fence.

### Trace outcomes separately

SuperDesktop writes `shell-hotkey:screen-snip-captured` only after clipboard image completion and `shell-hotkey:screen-snip-cancelled` only for explicit cancellation. The legacy accepted trace is retained only as an admission marker and cannot substitute for the captured trace in UTIT.

### Evidence adjustment classes

A-level changes may refine test mechanics without changing the completion fence. B-level corrections include clipboard formats, deadline, cancellation classification, trace semantics, or broker order and require artifact revalidation. Replacing the built-in tool, reading/storing pixels, weakening privacy, or extending external permissions is C-level and requires user approval.

## Risks / Trade-offs

- **Risk: unrelated image clipboard write is mistaken for capture** → Require capture intent or an overlay completion boundary in addition to sequence/image checks and keep the observation window bounded.
- **Risk: Escape polling misses a very short key press** → Treat unclassified dismissal as ambiguous and wait for publication rather than falsely report cancellation.
- **Risk: Snipping Tool publishes slowly** → Use a ten-second completion deadline and keep broker cleanup in all terminals.
- **Risk: clipboard delayed rendering is disturbed** → Open only for the shortest bounded handle probe, always close immediately, and never lock or read the image.

## Migration Plan

Ship the platform helper, app trace handling, physical UTIT, and installer together. No persistent state migration is required. Rollback restores the prior overlay-only behavior but does not alter clipboard contents or Snipping Tool settings.

## Open Questions

None. The Microsoft built-in tool remains the sole producer of the screenshot and clipboard payload.
