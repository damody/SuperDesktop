## ADDED Requirements

### Requirement: Release evidence uses the exact frozen Windows 11 ExplorerPatcher profile
The release verifier SHALL admit lifecycle, recovery, installer, and UI evidence only from an interactive Windows 11 workstation matching the frozen profile contract: build `26200`, UBR `9168`, ExplorerPatcher `26100.8457.70.3`, and every bound binary, settings, allowlist, and reference-image hash.

#### Scenario: Exact profile is admitted
- **WHEN** the live workstation and every referenced artifact match the frozen profile contract
- **THEN** the collector may proceed to non-mutating probes or separately authorized mutation phases

#### Scenario: Profile field or hash drifts
- **WHEN** the OS build, UBR, ExplorerPatcher version, binary, settings, allowlist, or reference-image hash differs
- **THEN** collection fails before Shell mutation and emits no passed evidence

### Requirement: Release evidence is candidate-bound and production-drift-free
Every reference-profile collector SHALL bind one full Git release-candidate revision and SHALL reject committed, staged, or working-tree production source or dependency drift relative to that candidate.

#### Scenario: Candidate and production tree match
- **WHEN** the candidate exists, is an ancestor of HEAD, and production paths have no drift
- **THEN** evidence records the candidate revision and product binary hashes

#### Scenario: Candidate or production lineage drifts
- **WHEN** the candidate is unavailable, is not an ancestor, or a production path differs
- **THEN** collection fails before lifecycle or installer effects

### Requirement: Reference-profile lifecycle evidence preserves recovery gates
The lifecycle collector SHALL verify preview zero-mutation, explicit Shell takeover, normal-exit restoration, ten authenticated forced-crash recoveries within ten seconds, and final exact Registry, Explorer-process, and work-area restoration.

#### Scenario: Lifecycle sequence passes
- **WHEN** every lifecycle phase uses the admitted candidate/profile and all recovery terminals and baselines pass
- **THEN** the raw artifact may satisfy `G-SHELL-TAKEOVER` and `G-GUARDIAN-RECOVERY`

#### Scenario: A recovery or restoration fails
- **WHEN** any run exceeds the deadline, lacks authenticated guardian completion, or fails to restore a baseline
- **THEN** both affected gates remain failed or pending and no final pass is normalized

### Requirement: Installer reboot and rollback evidence is phase-consistent
The installer verifier SHALL require `DryRun`, `Enable`, `AfterReboot`, and `Rollback` records with one candidate, one exact reference profile, one product binary set, one attributable operator, and one rollback-record path. Mutation SHALL additionally require explicit apply authority, explicit opt-in, and the exact plan fingerprint.

#### Scenario: Complete reboot sequence restores exact prior state
- **WHEN** all four phases pass and rollback restores the prior Registry presence/value and verified metadata disposition
- **THEN** the normalized artifact may satisfy `G-INSTALL-ROLLBACK`

#### Scenario: Phase identity or state differs
- **WHEN** a phase is missing or its candidate, profile, binary, operator, rollback path, fingerprint, or observed state differs
- **THEN** normalization fails and the installer gate remains blocked

### Requirement: ExplorerPatcher UI comparison remains exact and independent of runtime ownership
The UI verifier SHALL retain the frozen reference image, masked SSIM threshold of at least `0.95`, scaled geometry tolerance, exact control states, and pointer, keyboard, and UIA interaction assertions. ExplorerPatcher SHALL remain reference-only and SHALL NOT become a SuperDesktop runtime dependency.

#### Scenario: UI and interactions match the frozen reference contract
- **WHEN** visual, geometry, state, and all input-route checks pass on the admitted profile
- **THEN** the existing UI and accessibility gates retain passed status

#### Scenario: Visual threshold or control state fails
- **WHEN** masked SSIM is below `0.95` or any exact control/input assertion fails
- **THEN** the corresponding gate fails even if lifecycle behavior passes

### Requirement: Windows 10 is not a mandatory or claimed release target
Active release requirements, collectors, schemas, tasks, and roll-ups SHALL NOT require Windows 10 build 19045 and SHALL classify Windows 10 compatibility as not claimed.

#### Scenario: Obsolete Windows 10 artifact is supplied
- **WHEN** an artifact uses a `windows10` schema, kind, or active filename
- **THEN** the revised collector rejects it and does not translate it into reference-profile evidence

#### Scenario: Release blockers are derived after migration
- **WHEN** reference-profile lifecycle/installer evidence passes but physical mixed-DPI or independent review is absent
- **THEN** Windows 10 is not a blocker while the missing mandatory gates remain `external_pending`

### Requirement: Active lineage migrates atomically and archives remain immutable
The migration SHALL update active foundation, M0, completion-verification, and completion-program artifacts together, preserve stable task IDs where gate meaning is unchanged, record adjustment `C-W11-REFERENCE-001`, and SHALL NOT modify archived changes.

#### Scenario: Active migration is complete
- **WHEN** all revised artifacts and scripts validate and active-path scans find no mandatory Windows 10 requirement
- **THEN** new roll-ups use only the reference-profile evidence kind and retain truthful remaining blockers

#### Scenario: Partial migration remains
- **WHEN** any active schema, collector, task, or roll-up still requires or admits Windows 10
- **THEN** strict migration validation fails and no new release candidate is frozen
