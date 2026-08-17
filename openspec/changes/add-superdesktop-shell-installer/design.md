## Context

M0 can take over and recover Explorer for a single run, but has no packaging-time workflow for logon activation. Changing the Windows logon shell is high impact. Installation must be an explicit, auditable, reversible transaction and must never occur as a side effect of launching SuperDesktop.

## Goals / Non-Goals

**Goals:** Add dry-run, install, enable, disable, repair, uninstall, preflight, immutable backup record, compare-before-write, verification, rollback, guardian checks, and machine-readable audit output.

**Non-Goals:** Run the installer automatically, bypass UAC/policy, modify security desktops, replace Winlogon, or apply any mutation during tests in this environment.

## Decisions

1. Add a separate `shell-installer` CLI; the app binary has no installer mutation API.
2. Default every command to dry-run. Registry writes require `--apply --explicit-opt-in` plus an exact preflight plan fingerprint.
3. Use the per-user Winlogon `Shell` value only. The transaction records whether the value was absent or its exact previous UTF-16 value and restores that exact state.
4. Write and fsync a rollback record before registry mutation, compare the observed registry state immediately before write, then read back and verify.
5. Enable only when app and guardian paths are absolute, canonical, regular non-reparse files and runtime recovery admission succeeds. Disable/rollback remains available even if new binaries fail validation.
6. Uninstall first disables/restores the shell value, verifies Explorer restoration, then removes only SuperDesktop-owned metadata.

## Risks / Trade-offs

- [Power loss between registry write and verification] → Guardian and prewritten rollback record restore on next recovery.
- [External registry edit races] → Compare-before-write and refuse state drift.
- [Binary path changes] → Repair produces a new transaction; never silently edits an existing record.
- [Policy blocks per-user shell] → Preflight reports unsupported and performs no mutation.

## Migration Plan

Ship the CLI disabled-by-default, validate dry-run and memory-registry transactions, then perform exact Windows 11 ExplorerPatcher reference-profile enable/reboot/rollback evidence only in final verification. Existing run-scoped shell mode remains available independently; Windows 10 is not claimed.

## Open Questions

None.
