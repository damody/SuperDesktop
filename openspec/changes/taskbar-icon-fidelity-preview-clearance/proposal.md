## Why

SuperDesktop currently upscales low-resolution and BC7-compressed task icons, making them visibly softer than Explorer, and positions hover previews from the Windows work area without reserving the visible SuperDesktop taskbar. The result diverges from the native taskbar and, when Explorer is present, allows the preview to cover the taskbar.

## What Changes

- Select a DPI-appropriate 32–64 px task icon source and prefer size-matched or large Windows icon resources before small fallbacks.
- Upload small taskbar icons as lossless BGRA pixels while retaining the 24 DIP displayed size.
- Position task previews above the effective SuperDesktop taskbar top in both owned-shell and Explorer-compatible preview modes.
- Carry taskbar row count and mode through delayed and immediate preview-opening paths.
- Add deterministic unit, headful GUI, release, and installer gates for icon fidelity and preview clearance.

## Capabilities

### New Capabilities

- `taskbar-icon-fidelity`: DPI-aware Windows task icon acquisition and lossless small-icon rendering behavior.
- `task-preview-taskbar-clearance`: Task preview placement that respects the visible SuperDesktop taskbar across shell modes, row counts, DPI, and monitor origins.

### Modified Capabilities

None.

## Impact

The change affects `platform-win` icon acquisition, `taskbar-ui` image upload, `superdesktop-app` task discovery and popup geometry, focused GUI automation/evidence, and the normal Windows release/installer verification pipeline. It introduces no settings migration, public API break, additional privilege, or persistent-data change.
