# Windows Shell Icon Rendering Design

## Outcome

SuperDesktop renders the actual Windows application icon for each taskbar button and the actual Shell icon for each desktop item. Missing or inaccessible icons degrade to a stable generic visual without hiding labels or blocking interaction.

## Architecture

`platform-win::common::icon` owns all Win32 icon handles and converts them immediately into owned `shell_provider_protocol::IconData` RGBA pixels. Taskbar composition requests a window icon using the bounded order `WM_GETICON` (small, then big), class icon, then executable Shell icon. Desktop composition requests the Shell icon for each canonical item path. UI crates receive only owned pixels and never receive Win32 handles.

The app maintains bounded caches keyed by stable window/application identity for taskbar icons and canonical path for desktop icons. Refresh loops reuse cached pixels and prune stale entries so the existing 50 ms reconciliation does not repeatedly call Shell/GDI APIs.

## Win32 ownership and failure policy

- `SHGetFileInfoW` icons are owned by the caller and always destroyed after conversion.
- `WM_GETICON` and class icons are borrowed and are never destroyed by SuperDesktop.
- RGBA conversion uses a top-down 32-bit DIB, restores the prior selected object, and deletes the bitmap and memory DC on every path.
- Window messaging uses `SendMessageTimeoutW` so a hung application cannot stall the shell.
- Invalid handles, missing paths, conversion failures, and malformed pixel buffers return `None`; the UI keeps a truthful generic fallback.
- Legacy icons whose drawn RGB is visible but alpha is uniformly zero receive opaque alpha for nonzero pixels.

## UI behavior

Taskbar buttons display a 24-logical-pixel icon before the existing readable label. Desktop tiles display a 48-logical-pixel icon centered above the existing label. Image buffers are validated before constructing GPUI images, use nearest available source size, and preserve aspect ratio. The platform boundary remains RGBA, while GPUI's Windows `RenderImage` boundary is explicitly converted to BGRA. On adapters that support BC7 sampling, icons are encoded once, retained in a bounded render cache, and uploaded as direct BC7 block rows; unsupported adapters use the corrected BGRA atlas.

## Verification

Unit tests cover pixel validation, fallback ordering contracts, cache pruning, and model propagation. Windows integration tests extract icons from a real executable, directory, and file and verify nonempty alpha. Headful evidence must show real, non-placeholder taskbar and desktop icons at the host DPI. Repeated extraction is checked for stable process GDI object counts. Formatting, clippy, targeted tests, locked offline workspace checks, strict OpenSpec validation, and installer rebuild are blocking.

## Rollback

The change is source-only and adds no persisted schema or registry mutation. Reverting the icon module, model fields, composition cache, and UI rendering restores the prior placeholders.
