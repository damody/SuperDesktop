## Why

SuperDesktop currently renders taskbar applications and desktop items with placeholder glyphs, so users cannot visually identify programs or files as they can in Windows Explorer. The application already owns the correct window and canonical-path identities, making native icon parity an actionable correctness repair.

## What Changes

- Add an owned Windows icon extraction and RGBA conversion boundary.
- Resolve taskbar icons from the live window with executable/Shell fallback and bounded caching.
- Resolve desktop icons from each Shell item path with bounded caching.
- Render DPI-aware icons in taskbar buttons and desktop tiles while preserving labels and actions.
- Correct RGBA/BGRA channel handling and use bounded BC7 GPU icon uploads when supported.
- Add resource-lifetime, integration, headful, and installer evidence.

## Capabilities

### New Capabilities

- `native-shell-icon-rendering`: Defines native taskbar and desktop icon resolution, fallback, rendering, caching, and resource ownership.

### Modified Capabilities

None.

## Impact

- Affects `platform-win`, `taskbar-ui`, `desktop-ui`, and `superdesktop-app`.
- Reuses the existing protocol `IconData`, updates the GPUI fork to its BC7-capable revision, and adds the bounded `intel_tex_2` encoder; no settings schema, registry, or external API change.
- Packaging changes only through rebuilt SuperDesktop and combined installers.
