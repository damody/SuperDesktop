## Why

SuperDesktop's implemented UI already targets a frozen Windows 11 and ExplorerPatcher reference profile, but the active release program still requires unavailable Windows 10 build 19045 evidence. The release contract must match the approved product target while retaining fail-closed lifecycle, rollback, display, and independent-review gates.

## What Changes

- **BREAKING** Replace the mandatory Windows 10 build 19045 evidence kind and admission rules with an exact Windows 11 build `26200.9168` and ExplorerPatcher `26100.8457.70.3` reference-profile contract.
- Bind lifecycle, Shell takeover, guardian recovery, installer reboot/rollback, and UI evidence to the same immutable candidate and frozen profile hashes.
- Rename active Windows 10-specific collectors, schemas, evidence kinds, blocker records, and instructions to platform-neutral reference-profile names.
- Update the active foundation, M0 verification, shell-completion verification, and parent program artifacts so their release language and derived gates agree.
- Preserve the physical mixed-DPI and attributable independent-review gates as mandatory release blockers.
- Treat Windows 10 compatibility as not claimed rather than a release prerequisite.

## Capabilities

### New Capabilities

- `windows11-reference-release-baseline`: Defines exact reference-profile admission, candidate lineage, lifecycle and installer evidence normalization, UI comparison requirements, migration behavior, and release-gate derivation.

### Modified Capabilities

None. Existing active, unarchived change artifacts are migrated by implementation tasks; no archived or globally published capability is rewritten.

## Impact

- Revises active OpenSpec artifacts under the shell foundation, M0 verification, shell-completion verification, and completion-program changes.
- Revises PowerShell lifecycle, installer, normalization, roll-up, and evidence-capture scripts.
- Revises JSON schemas, evidence templates, readiness records, and release-candidate lineage.
- Does not change product runtime APIs, mutate the login Shell automatically, reboot the workstation, modify archived evidence, or archive any change.
