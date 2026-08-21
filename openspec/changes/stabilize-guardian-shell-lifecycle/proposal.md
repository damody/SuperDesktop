## Why

SuperDesktop can reject its own recovery guardian when Windows reports an equivalent executable through a different path spelling, then misreport the rejection as an acceptance timeout. If the rollback record is also missing, failure recovery and the explicit return-to-Explorer command cannot restore the default shell, while an expected AppBar geometry fallback produces a misleading warning.

## What Changes

- Preserve immutable guardian file-identity validation while accepting equivalent normalized Windows path spellings.
- Make guardian acceptance bounded, observable, and able to distinguish early child rejection from a real timeout.
- Reconstruct a default-Explorer rollback record only for an exact current SuperDesktop shell registration; continue refusing unknown shell values.
- Make Explorer restoration idempotent when Explorer is already the registered shell.
- Retain owned monitor geometry when AppBar registration is unavailable without emitting a console warning for the expected degraded mode.
- Add lifecycle unit, integration, release-fixture, console-signature, and packaging evidence.

## Capabilities

### New Capabilities

- `owned-shell-recovery-lifecycle`: Defines guardian identity/acceptance, rollback reconstruction, Explorer restoration, and expected AppBar fallback behavior for an owned Windows shell.

### Modified Capabilities

None.

## Impact

The change affects `platform-win` guardian lease handling, `superdesktop-guardian`, `superdesktop-app` shell registration and taskbar startup, shell-installer rollback records, lifecycle test scripts, release binaries, and the parent installer/submodule integration. It does not weaken file identity checks, overwrite third-party shell registrations, or reintroduce Explorer-owned taskbar geometry.
