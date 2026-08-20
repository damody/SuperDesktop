## Why

SuperDesktop's GUI parity checks are fragmented across scripts and do not prove complete surface coverage or consistent Windows geometry. The final shell will contain only SuperDesktop and SuperExplorer, so missing UI coverage, proportion drift, and normal-path Explorer dependencies must become automatic release failures now.

## What Changes

- Add a compiled manifest for every owned GUI surface and its Windows reference geometry, variants, controls, interactions, artifacts, and Explorer-free policy.
- Add a normalized GUI measurement/report schema with exact physical-to-DIP conversion and named region/ratio checks.
- Make UTIT reject uncovered manifest surfaces, unmanifested GUI cases, script-local geometry constants, malformed/stale reports, and unsupported pass claims.
- Add a product source gate that permits Explorer only in guardian recovery, installer rollback, test watchdogs, and explicit Return to default Explorer behavior.
- Tag and execute a first-wave GUI parity matrix for taskbar, Start, system flyouts, notification overflow, Jump Lists, hover previews, context menus, task view, and Alt-Tab.
- Centralize Windows shell chrome metrics and correct first-wave size/proportion drift found by the new matrix.
- Preserve hardware/reboot/external-review limitations as blocked or evidence-backed not-applicable.

## Capabilities

### New Capabilities

- `windows-gui-parity-automation`: Complete owned-surface inventory, normalized Windows geometry measurement, automated parity gating, and Explorer-free source/runtime enforcement.

### Modified Capabilities

None. Earlier shell GUI changes are still unarchived; this foundation adds one integrating gate without weakening their requirements.

## Impact

Affected components are `superdesktop-utit`, headful PowerShell adapters, `taskbar-ui`, `desktop-ui`, `superdesktop-app` composition, and platform recovery/source-policy tests. The normalized report is test-only and introduces no production IPC or dependency. Later desktop/file-manager and release-closure waves consume this manifest rather than redefining parity thresholds.
