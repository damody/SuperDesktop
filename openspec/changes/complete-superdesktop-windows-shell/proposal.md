## Why

SuperDesktop M0 proves the shell lifecycle, desktop, taskbar, Explorer bridge, and recovery foundation, but it does not yet cover the everyday Windows shell workflows users expect. This program coordinates the remaining product changes and verification without weakening the existing fail-closed safety model.

## What Changes

- Extend shared shell contracts and add an isolated provider host.
- Add desktop file operations and a safe shell context-menu host.
- Add Start/search, advanced taskbar interactions, notification-area hosting, and virtual-desktop controls.
- Add an explicit opt-in shell installer with transactional rollback.
- Verify the completed shell against functional, accessibility, DPI, performance, safety, recovery, and traceability gates.

## Capabilities

### New Capabilities

- `windows-shell-completion-program`: Defines dependency ordering, ownership, release gates, and evidence roll-up for the completion program.

### Modified Capabilities

None.

## Impact

- Coordinates nine child OpenSpec changes under `openspec/changes/`.
- Affects the Rust workspace, Windows platform adapters, GPUI surfaces, test fixtures, installer tooling, and evidence automation.
- Preserves explicit opt-in takeover, guardian recovery, and fail-closed release rules.
