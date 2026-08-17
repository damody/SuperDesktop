## Context

The active release program has two conflicting platform contracts. UI and capability evidence is frozen to Windows 11 build `26200.8875` with ExplorerPatcher `26100.8457.70.3`, while final lifecycle and installer gates require Windows 10 build 19045. The user approved a material C-level correction: make the existing Windows 11 ExplorerPatcher profile mandatory and stop claiming Windows 10 compatibility.

The migration crosses active OpenSpec changes, PowerShell collectors, JSON schemas, evidence filenames, task state, and release-candidate lineage. No admitted Windows 10 pass artifact exists, so active evidence identifiers can change without rewriting historical archives.

## Goals / Non-Goals

**Goals:**

- Establish one canonical, hash-bound Windows 11 ExplorerPatcher release profile.
- Reuse the existing lifecycle, recovery, installer, UI, and safety thresholds without weakening them.
- Make every active requirement, task, collector, schema, and roll-up derive the same platform decision.
- Reject old Windows 10 evidence and all profile/candidate drift before mutation.
- Leave physical mixed-DPI and independent review mandatory.

**Non-Goals:**

- Support arbitrary Windows 11 builds or stock Windows 11 taskbar visuals.
- Claim Windows 10 compatibility.
- Replace physical display or independent-review evidence with local simulation.
- Execute installer mutation, reboot, or archive operations as part of the migration.
- Rewrite archived OpenSpec evidence.

## Decisions

### 1. The frozen profile contract is the single platform authority

Collectors read `validate-superdesktop-windows-platform/evidence/artifacts/1.1/frozen-profile-contract.json` and verify its referenced hashes. They also verify live Windows build `26200`, UBR `8875`, ExplorerPatcher version `26100.8457.70.3`, ExplorerPatcher binary hashes, settings/allowlist lineage, and reference image hash. Duplicated constants in user-facing messages are allowed only for diagnostics; admission derives from the contract.

Accepting any Windows 11 build was rejected because Windows updates can change Shell Hook, AppBar, ExplorerPatcher, and rendering behavior. Keeping Windows 10 as an optional active evidence kind was rejected because no current compatibility commitment requires the extra schema branch.

### 2. Evidence identifiers become platform-neutral

The active external kind becomes `reference-profile-lifecycle-installer`. Active M0 and completion artifacts use `reference-profile` names and schemas. Old `windows10-*` inputs are rejected, not automatically converted. Archived artifacts remain untouched.

### 3. Existing safety gates keep their meaning

The reference-profile lifecycle collector still requires preview zero-mutation, normal-exit restoration, ten forced-crash recoveries within ten seconds, authenticated guardian completion, exact Registry/Explorer/work-area restoration, and attributable operator identity. The installer sequence still requires `DryRun`, `Enable`, `AfterReboot`, and `Rollback`, one binary set, one operator, one rollback path, explicit opt-in, and exact plan fingerprint.

### 4. UI evidence uses the existing ExplorerPatcher reference

The taskbar reference hash remains `48B5F990B9E155C5C2719D8F8B41D88ED4420A46C3B6018278511F9C349B387E`; masked SSIM remains at least `0.95`; exact control-state, geometry, pointer, keyboard, and UIA assertions remain required. ExplorerPatcher is a comparison environment, not a SuperDesktop runtime dependency.

### 5. Active parent artifacts are corrected together

The foundation, M0 verification, shell-completion verification, and completion parent are an active unarchived lineage. Their design/spec/tasks/evidence are updated in one migration so no downstream roll-up can still require or admit Windows 10. Previously completed local tasks remain completed only when their evidence is unaffected; platform-specific pending tasks are renamed without being marked passed.

### 6. Adjustment classification and evidence invalidation

This is approved adjustment `C-W11-REFERENCE-001`. Task mechanics can later receive A-level refinements. Any correction to hashes, filenames, or validation details within the approved profile is B-level and reopens affected validation/evidence. Any change to platform, build/UBR, ExplorerPatcher version, visual threshold, mandatory display/review gates, mutation permissions, or release semantics is C-level and requires new user approval.

## Component and data flow

1. The profile contract supplies immutable OS, ExplorerPatcher, settings, image, and toolchain lineage.
2. The release-candidate manifest supplies the immutable source revision.
3. Lifecycle and installer collectors validate both authorities before effects and emit raw reference-profile evidence.
4. The completion finalizer validates cross-phase identity and normalizes the lifecycle/installer artifact.
5. The completion collector validates the normalized artifact plus physical mixed-DPI and independent-review artifacts.
6. M0, completion, and parent roll-ups derive blockers and `release_allowed`; they do not accept operator assertions without rehashable source records.

## Risks / Trade-offs

- **[Windows or ExplorerPatcher updates invalidate the workstation]** → Fail before mutation and require a separately approved profile rebaseline.
- **[A partial rename admits old evidence]** → Change schema enums, collector lookup names, finalizer output, README, and tests atomically; scan active paths for Windows 10 identifiers.
- **[Existing evidence becomes ambiguous]** → Preserve old files only as rejected lineage or superseded records; never relabel their schema/kind.
- **[Reboot sequence leaves Shell state changed]** → Keep explicit mutation authority, immutable rollback record, phase continuity, guardian recovery, and exact rollback verification.
- **[Removing Windows 10 is mistaken for broader native parity]** → Record Windows 10 as not claimed and retain limitations for legacy Explorer protocols and undocumented virtual-desktop operations.
- **[Current single monitor cannot close release]** → Keep `G-DPI-MONITOR-PHYSICAL` external pending; this platform correction does not reduce that gate.

## Migration Plan

1. Add this change's contract and task plan.
2. Revise active OpenSpec foundation/M0/completion/parent language and pending task names; preserve task IDs and record `C-W11-REFERENCE-001`.
3. Rename and revise collectors, finalizers, templates, schemas, and active evidence paths.
4. Add fail-closed profile-validation helpers and negative tests/probes.
5. Recompute local evidence and roll-ups; ensure obsolete Windows 10 kinds are rejected.
6. Run parser, workspace, task, and strict OpenSpec validation.
7. Freeze a new candidate only after the migrated harness is committed and green.

Rollback consists of reverting the migration commits before any new reference-profile external evidence is admitted. It performs no Registry or filesystem rollback outside the repository.

## Open Questions

None. The user selected the exact existing Windows 11 ExplorerPatcher profile and approved removal of the mandatory Windows 10 gate.
