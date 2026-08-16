## ADDED Requirements

### Requirement: Installation is explicit and dry-run by default
The installer SHALL perform no registry or filesystem mutation unless both apply and explicit-opt-in authority are present and match a current preflight plan.

#### Scenario: Install is invoked without apply
- **WHEN** a user requests install or enable without `--apply`
- **THEN** the installer emits the exact plan and performs no mutation

### Requirement: Shell changes are transactional
The installer SHALL persist the exact prior shell state before compare-before-write mutation and verify the resulting value.

#### Scenario: Registry state drifts after planning
- **WHEN** the observed Shell value no longer matches the plan precondition
- **THEN** the transaction aborts without overwriting the external change

### Requirement: Rollback is exact and always reachable
The installer SHALL restore either the exact prior value or the exact prior absence and verify Explorer recovery.

#### Scenario: Enable verification fails
- **WHEN** the written shell value cannot be read back exactly
- **THEN** the installer immediately attempts rollback and returns a failed terminal record

### Requirement: Binary and guardian preflight is fail closed
The installer SHALL require canonical absolute regular non-reparse app/guardian paths, supported session/policy, and admitted guardian recovery before enable.

#### Scenario: Guardian is unavailable
- **WHEN** guardian recovery admission fails
- **THEN** enable is rejected before registry mutation

### Requirement: Uninstall restores before removal
The installer SHALL disable/rollback shell activation before removing SuperDesktop-owned installer metadata.

#### Scenario: Restore cannot be verified
- **WHEN** uninstall cannot verify the prior shell state
- **THEN** it preserves rollback metadata and reports uninstall incomplete

### Requirement: Every operation is auditable
The installer SHALL emit a machine-readable record containing operation, plan fingerprint, pre/post observations, affected targets, timestamps, and terminal disposition without secrets.

#### Scenario: Dry-run completes
- **WHEN** any command runs without mutation authority
- **THEN** its audit record reports `dry_run` and the exact targets that would be affected
