## Why

When Explorer is absent, Windows exposes minimized top-level windows as legacy iconic title tiles at the lower-left desktop edge. SuperDesktop must provide Explorer-equivalent taskbar ownership: the minimized application stays in the taskbar but no separate desktop tile remains visible.

## What Changes

- Add an owned-shell minimized-window shelf that asynchronously hides only the already-iconic representation while caching its taskbar model and preserving normal restore bounds.
- Apply the shelf immediately to SuperDesktop-originated minimize commands and reconcile application-originated minimization from the existing live task snapshot.
- Revalidate HWND, PID, stable identity, visibility, minimized state, and task eligibility before every asynchronous hide.
- Prune restored or retired identities and deduplicate contextual console errors without permanent failure suppression.
- Add unit, fixture, and headful UTIT coverage for hide, taskbar retention, exact restore, application-owned minimize, failure cleanup, and package provenance.

## Capabilities

### New Capabilities

- `owned-minimized-window-shelf`: Defines Explorer-equivalent hidden iconic-window handling, cached taskbar representation, and restoration while SuperDesktop owns the shell.

### Modified Capabilities

None.

## Impact

The change affects the Windows taskbar adapter, shell runtime task reconciliation, taskbar action paths, Windows-only UTIT fixtures, release verification, and installer evidence. It adds no dependency, persistent setting, public API, window-style mutation, Explorer fallback, or migration.
