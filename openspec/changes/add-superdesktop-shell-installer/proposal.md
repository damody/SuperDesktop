## Why

The shell can take over safely at runtime, but users need a deliberate, recoverable way to install, enable, disable, repair, and uninstall it for logon use.

## What Changes

- Add explicit opt-in install, enable, disable, repair, and uninstall commands.
- Add preflight checks, signed transaction records, backup, verification, rollback, and guardian handoff.
- Add non-destructive dry-run and unsupported-session rejection.

## Capabilities

### New Capabilities

- `shell-installer`: Transactional installation, opt-in activation, repair, rollback, uninstall, and audit behavior.

### Modified Capabilities

None.

## Impact

- Adds installer CLI/tooling and affects packaging, registry adapters, guardian recovery, release scripts, tests, and evidence.
