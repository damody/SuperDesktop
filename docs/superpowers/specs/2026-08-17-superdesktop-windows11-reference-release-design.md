# SuperDesktop Windows 11 ExplorerPatcher Release Baseline

## Context

SuperDesktop already uses a frozen Windows 11 and ExplorerPatcher environment as its UI and interaction reference, but the final completion program still requires Windows 10 22H2 build 19045 evidence. The available reference workstation is Windows 11 build `26200.9168` with ExplorerPatcher `26100.8457.70.3`. The Windows 10 requirement prevents the completed implementation from entering final lifecycle and installer verification even though Windows 10 is no longer a required product target.

This design replaces the mandatory Windows 10 compatibility gate with a revision-bound Windows 11 ExplorerPatcher reference-profile gate. It does not weaken the lifecycle, recovery, installer rollback, physical mixed-DPI, accessibility, security, or independent-review requirements.

## Goals

- Make the frozen Windows 11 build `26200.9168` and ExplorerPatcher `26100.8457.70.3` profile the mandatory release environment.
- Use the existing ExplorerPatcher taskbar and Explorer UI reference image as the visual and interaction baseline.
- Run Shell takeover, normal exit, forced-crash guardian recovery, installer enable/reboot/rollback, and UI verification against one immutable release candidate and one exact reference profile.
- Remove Windows 10 build 19045 from mandatory blockers, artifact schemas, scripts, tasks, and release language.
- Preserve fail-closed evidence admission and truthful unsupported-capability reporting.

## Non-goals

- Supporting arbitrary Windows 11 builds or arbitrary ExplorerPatcher configurations.
- Changing the UI target to the stock Windows 11 taskbar.
- Claiming Windows 10 compatibility.
- Replacing physical mixed-DPI confirmation with virtual topology evidence.
- Removing the independent architecture, security, accessibility, and lineage review.
- Automatically mutating the login Shell, rebooting the workstation, or archiving OpenSpec changes.

## Considered approaches

### A. Exact frozen Windows 11 ExplorerPatcher profile

Require the existing OS build, UBR, ExplorerPatcher binaries/settings, and reference image hashes. This is the selected approach because it is reproducible and reuses the established UI baseline.

### B. Any Windows 11 workstation

Accept a range of Windows 11 builds and compare only functional behavior. This reduces setup friction but makes visual, AppBar, Shell Hook, and ExplorerPatcher behavior non-deterministic after updates.

### C. Windows 11 mandatory with optional Windows 10 compatibility

Keep a non-blocking Windows 10 evidence path. This adds schema and maintenance cost without affecting the current release decision. Windows 10 can be proposed later as a separate compatibility change if required.

## Selected baseline

The mandatory reference profile is identified by all of the following:

- Windows edition: interactive Windows 11 workstation.
- OS build and UBR: `26200.9168`.
- ExplorerPatcher version: `26100.8457.70.3`.
- ExplorerPatcher binary hashes from the frozen profile contract.
- ExplorerPatcher and Explorer UI-affecting settings hash: `020E3E000A3A91B837923722A1081FADA78AF8518F1FBDF3F451878A3665BD6D`.
- Reference taskbar image hash: `48B5F990B9E155C5C2719D8F8B41D88ED4420A46C3B6018278511F9C349B387E`.
- Existing visual threshold: masked SSIM at least `0.95`, exact control-state assertions, and scaled geometry tolerance.

The canonical source is the frozen profile contract under `validate-superdesktop-windows-platform`. Collectors must read and hash that contract rather than duplicate unbound constants. Any profile mismatch is a blocking stale-baseline result, not a warning.

## Evidence architecture

### Reference-profile lifecycle collector

The Windows 10-specific lifecycle collector becomes a reference-profile collector. Before any Shell action it must:

1. Bind the full Git release-candidate revision and prove that production sources and dependencies have not drifted.
2. Validate the exact OS build/UBR and interactive workstation session.
3. Validate ExplorerPatcher binary version and hashes, the settings snapshot/allowlist, and the reference image lineage.
4. Require an attributable operator for mutation-bearing lifecycle evidence.

It then verifies preview zero-mutation, explicit Shell takeover, normal-exit restoration, ten forced-crash guardian recoveries within the existing deadline, and exact final restoration of Explorer, Registry, and work-area state.

### Installer reboot sequence

The installer physical collector retains four phases: `DryRun`, `Enable`, `AfterReboot`, and `Rollback`. Mutation remains gated by `-Apply`, explicit opt-in, and an exact plan fingerprint. Every phase must bind the same candidate revision, product binary hashes, operator identity, rollback-record path, and exact Windows 11 ExplorerPatcher profile. Rollback must restore exact prior Registry presence/value and remove metadata only after verification.

### Completion normalization

The external evidence kind changes from `windows10-lifecycle-installer` to `reference-profile-lifecycle-installer`. The normalized artifact proves the same gates:

- `G-SHELL-TAKEOVER`
- `G-GUARDIAN-RECOVERY`
- `G-INSTALL-ROLLBACK`

The physical mixed-DPI artifact remains separate and mandatory. The independent-review artifact remains separate and mandatory.

### UI reference behavior

Windows 11 ExplorerPatcher is a reference profile, not a runtime dependency injected into SuperDesktop. SuperDesktop continues to own its GPUI desktop, taskbar, Start surface, providers, lifecycle, and recovery. ExplorerPatcher supplies the frozen comparison UI and interaction expectations only. Unsupported undocumented Explorer internals remain unavailable or not claimed.

## Naming and migration

Active specifications, tasks, schemas, scripts, and evidence use platform-neutral reference-profile names. Existing unarchived Windows 10-specific evidence filenames may be renamed because no admitted external pass artifact exists. Historical archived evidence is immutable and is not rewritten.

The migration updates:

- the foundation design/spec/task release target;
- M0 verification tasks, collectors, evidence indexes, and blocker names;
- shell-completion proposal/design/spec/tasks, schema, collector, finalizer, templates, and README;
- completion-program design/spec/tasks, program ledger, and roll-ups;
- release-candidate purpose and external-harness readiness evidence.

Task IDs remain stable where the observable gate is unchanged. Evidence kinds, schema identifiers, and artifact paths change atomically so old Windows 10 artifacts cannot be admitted accidentally.

## Failure handling and safety

- Build, UBR, ExplorerPatcher version, binary hash, settings hash, reference image hash, candidate revision, or product binary drift fails before Shell mutation.
- Missing operator/reviewer attribution fails before evidence output is accepted.
- A partial installer phase cannot be normalized as passed.
- A failed takeover or recovery run triggers the existing guardian restoration path and keeps the gate failed.
- Existing external artifacts with an obsolete Windows 10 kind are rejected by the revised schema and collector.
- No script may silently translate a Windows 10 artifact into the new reference-profile kind.

## Verification

Implementation validation requires:

- PowerShell parser validation for every revised collector/finalizer.
- Negative probes for wrong OS build, wrong UBR, ExplorerPatcher drift, candidate drift, missing attribution, incomplete reboot phases, binary drift, and rollback mismatch.
- A zero-mutation dry-run on the frozen workstation.
- Workspace format, tests, clippy, and strict OpenSpec validation.
- Recomputed M0, shell-completion, and parent program roll-ups.
- Physical mixed-DPI confirmation and independent review remain blocking until their attributable artifacts exist.

## Rollout and rollback

The specification and harness migration lands before any new external capture. After validation, a new immutable release candidate is frozen. External evidence must reference only that candidate. Rolling back this design restores the Windows 10-specific active artifacts and blocker, but does not modify Registry state or archived evidence.

## Completion criteria

The migration is complete when no active requirement, task, collector, schema, roll-up, or release document requires Windows 10 build 19045; the exact Windows 11 ExplorerPatcher reference profile is enforced fail-closed; all local validation passes; and the remaining release blockers truthfully represent only evidence not yet captured under the revised plan.
