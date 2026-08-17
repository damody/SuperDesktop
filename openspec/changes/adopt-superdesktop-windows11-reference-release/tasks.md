# Adopt SuperDesktop Windows 11 Reference Release

## 1. Contract and active-lineage migration

### 1.1 Freeze the approved platform and adjustment contract

**目的：** Record the approved C-level platform correction and one canonical exact reference profile.
**輸入：** Approved design, frozen platform contract, current active change lineage, and release-candidate manifest.
**產出：** Adjustment record, profile inventory, and traceability map under this change's evidence directory.
**依賴：** Approved design commit `4c26aa61` and proposal/design/spec artifacts.
**Owner／Wave：** Primary integrator / Wave 1.
**Gate／Evidence：** `G-ARCH`, `G-TRACE`; `evidence/contract-migration.json`.
**完成門檻：** Every approved constant, unchanged gate, affected active artifact, and superseded Windows 10 identifier is explicit and hash-bound.

- [x] 1.1.1 Add `C-W11-REFERENCE-001` with approval, scope, unchanged thresholds, and evidence invalidation rules.
- [x] 1.1.2 Inventory every active non-archive Windows 10 requirement, task, script, schema, template, and evidence identifier.
- [x] 1.1.3 Bind the canonical frozen-profile contract, settings, allowlist, ExplorerPatcher binaries, and reference image hashes.
- [x] 1.1.4 Map each new requirement scenario to its implementation file, validation command, gate, and evidence subcheck.

### 1.2 Correct all active OpenSpec release contracts

**目的：** Make every active parent and verifier describe the same Windows 11 ExplorerPatcher release target.
**輸入：** 1.1 outputs and active foundation, M0, completion-verification, and completion-program artifacts.
**產出：** Revised proposals, designs, specs, tasks, program ledger, and execution guidance with stable task IDs.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 1.
**Gate／Evidence：** `G-ARCH`, `G-TRACE`; `evidence/active-artifact-migration.json`.
**完成門檻：** No active contract requires Windows 10, Windows 10 is not claimed, and physical mixed-DPI plus independent review remain mandatory.

- [x] 1.2.1 Revise shell-foundation proposal, design, specs, tasks, and execution guidance without modifying archives.
- [x] 1.2.2 Revise M0 verification proposal, design, specs, and pending platform task descriptions while preserving IDs.
- [x] 1.2.3 Revise shell-completion proposal, design, spec, and pending task descriptions while preserving IDs.
- [x] 1.2.4 Revise completion-program proposal, design, spec, tasks, and `PROGRAM.md` release language.
- [x] 1.2.5 Add Windows 10 `not-claimed` classification and retain legacy tray and undocumented virtual-desktop limitations.
- [x] 1.2.6 Produce an active-path scan proving no mandatory Windows 10 release statement remains.

## 2. Reference-profile admission and collectors

### 2.1 Implement reusable fail-closed profile admission

**目的：** Provide one reusable verifier for candidate lineage and the exact live Windows 11 ExplorerPatcher profile.
**輸入：** Frozen-profile contract and release-candidate manifest.
**產出：** PowerShell reference-profile admission module and machine-readable observation.
**依賴：** 1.1.
**Owner／Wave：** Verification harness owner / Wave 2.
**Gate／Evidence：** `G-SAFETY`, `G-TRACE`; `evidence/profile-admission-verification.json`.
**完成門檻：** Exact live profile passes; wrong build, UBR, version, hash, settings, image, candidate, or source state fails before effects.

- [x] 2.1.1 Implement canonical contract loading and repository-contained referenced-file hash verification.
- [x] 2.1.2 Implement live OS build/UBR, workstation/session, ExplorerPatcher version, binary, and settings observation.
- [x] 2.1.3 Implement release-candidate ancestry plus committed, staged, and working-tree production drift rejection.
- [x] 2.1.4 Emit a bounded owned observation containing profile fingerprint, candidate revision, and rehashable sources.
- [x] 2.1.5 Add negative fixture tests for every independently failing admission field and obsolete Windows 10 input.
- [x] 2.1.6 Run a live zero-mutation admission probe on the frozen workstation.

### 2.2 Migrate lifecycle and guardian evidence collection

**目的：** Replace the Windows 10 lifecycle collector with reference-profile lifecycle evidence while retaining all recovery controls.
**輸入：** 2.1 module, release binaries, guardian contracts, and existing lifecycle collector.
**產出：** Reference-profile lifecycle collector and raw artifact schema.
**依賴：** 2.1.
**Owner／Wave：** Lifecycle verification owner / Wave 2.
**Gate／Evidence：** `G-SHELL-TAKEOVER`, `G-GUARDIAN-RECOVERY`, `G-SAFETY`; `evidence/lifecycle-collector-verification.json`.
**完成門檻：** Parser and zero-effect negative probes pass; mutation-bearing phases remain explicit and unexecuted during migration.

- [x] 2.2.1 Rename the active lifecycle collector and output schema/path to reference-profile terminology.
- [x] 2.2.2 Invoke shared admission before builds, Shell actions, operator acceptance, or evidence output.
- [x] 2.2.3 Preserve preview zero-mutation, normal-exit, ten forced-crash, authenticated terminal, deadline, and final-baseline checks.
- [x] 2.2.4 Replace Windows 10 host metadata with exact profile fingerprint and source hashes.
- [x] 2.2.5 Add parser and wrong-profile fail-closed probes proving no output or Shell mutation.

### 2.3 Migrate installer reboot and rollback collection

**目的：** Bind every installer phase to the exact reference profile without changing mutation authority or rollback semantics.
**輸入：** 2.1 module, packaged binaries, current installer phase collector, and rollback contract.
**產出：** Revised installer phase collector and phase records.
**依賴：** 2.1.
**Owner／Wave：** Installer verification owner / Wave 2.
**Gate／Evidence：** `G-INSTALL-ROLLBACK`, `G-SAFETY`; `evidence/installer-collector-verification.json`.
**完成門檻：** DryRun proves zero mutation; Apply remains gated; all phase records carry identical candidate/profile/binary/operator/rollback identity.

- [x] 2.3.1 Require shared profile admission before every `DryRun`, `Enable`, `AfterReboot`, and `Rollback` phase.
- [x] 2.3.2 Replace the build-19045 mutation check with exact frozen-profile fingerprint admission.
- [x] 2.3.3 Record profile sources and fingerprint in every installer phase artifact.
- [x] 2.3.4 Preserve apply, explicit-opt-in, exact fingerprint, rollback metadata, and exact-absence restoration controls.
- [x] 2.3.5 Run packaged installer dry-run and verify Registry value presence/value and rollback-record absence are unchanged.
- [x] 2.3.6 Add negative probes for phase profile, candidate, binary, operator, rollback path, and fingerprint mismatch.

### 2.4 Migrate normalization, schema, and external instructions

**目的：** Admit only complete reference-profile lifecycle/installer evidence in the completion roll-up.
**輸入：** 2.2 and 2.3 artifact contracts, existing finalizer, collector, schema, templates, and README.
**產出：** Renamed finalizer/template/external kind, revised schema/collector, and operator instructions.
**依賴：** 2.2 and 2.3.
**Owner／Wave：** Completion evidence owner / Wave 2.
**Gate／Evidence：** `G-TRACE`, `G-SHELL-TAKEOVER`, `G-GUARDIAN-RECOVERY`, `G-INSTALL-ROLLBACK`; `evidence/normalization-verification.json`.
**完成門檻：** Complete new artifacts normalize; obsolete, partial, stale, mismatched, or translated Windows 10 artifacts fail.

- [x] 2.4.1 Rename the completion finalizer, parameters, schemas, kind, and output to `reference-profile-lifecycle-installer`.
- [x] 2.4.2 Revise the roll-up JSON schema and collector external-source admission enum and validation.
- [x] 2.4.3 Revise operator README, confirmation templates, readiness record, and release-candidate purpose.
- [x] 2.4.4 Add fixtures proving old Windows 10 kinds and partial/mismatched phase sets are rejected.
- [x] 2.4.5 Validate normalized gate derivation keeps physical mixed-DPI and independent review external pending.

## 3. M0 and program evidence integration

### 3.1 Migrate M0 local capture and foundation roll-up

**目的：** Make M0 and foundation evidence derive reference-profile lifecycle state without a Windows 10 blocker.
**輸入：** Revised active contracts and normalized artifact schema.
**產出：** Revised local capture, blocker artifacts, foundation roll-up, and task-state derivation.
**依賴：** 1.2 and 2.4.
**Owner／Wave：** M0 evidence owner / Wave 3.
**Gate／Evidence：** `G-TRACE`, `G-SHELL-TAKEOVER`, `G-GUARDIAN-RECOVERY`; `evidence/m0-rollup-migration.json`.
**完成門檻：** M0 accepts only new evidence, preserves pending physical/review tasks, and reports Windows 10 as not claimed rather than blocked.

- [x] 3.1.1 Replace Windows 10 artifact lookup, schema checks, task derivation, and blocker text in local M0 capture.
- [x] 3.1.2 Replace Windows 10 status and evidence paths in foundation roll-up capture.
- [x] 3.1.3 Preserve five-DPI, physical mixed-DPI, independent-review, archive, and final-disposition dependencies.
- [x] 3.1.4 Regenerate local blocker/evidence artifacts without marking unexecuted external tasks passed.
- [x] 3.1.5 Validate every migrated M0/foundation task ID has unique evidence or a unique shared subcheck.

### 3.2 Recompute completion and program roll-ups

**目的：** Publish internally consistent local roll-ups after the platform migration.
**輸入：** 2.4 and 3.1, eight implementation-child verification records, and current OpenSpec status.
**產出：** Revised completion roll-up, program roll-up, exact blocker set, and migration verification record.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / Wave 3.
**Gate／Evidence：** `G-TRACE`; `evidence/rollup-verification.json`.
**完成門檻：** `implementation_complete=true`; Windows 10 is absent from blockers; release remains false only for genuinely pending revised mandatory gates.

- [x] 3.2.1 Recompute shell-completion local roll-up with the revised schema and external-source names.
- [x] 3.2.2 Recompute completion-program lineage, child counts, commits, limitations, and blockers.
- [x] 3.2.3 Verify old external kinds, missing children, duplicate gates, stale hashes, and contradictory decisions fail.
- [x] 3.2.4 Record the exact remaining blocker set without inventing physical or independent evidence.

## 4. Validation and candidate handoff

### 4.1 Validate the complete migration

**目的：** Prove the migrated contracts and harness are syntactically, structurally, behaviorally, and safely complete.
**輸入：** All prior work packages.
**產出：** Validation logs and `evidence/verification.json` with one subcheck per task.
**依賴：** 1.2, 2.4, and 3.2.
**Owner／Wave：** Primary integrator / Wave 4.
**Gate／Evidence：** `G-ARCH`, `G-SAFETY`, `G-TRACE`; `evidence/verification.json`.
**完成門檻：** All parsers, negative probes, workspace checks, detailed-task validation, strict OpenSpec validation, and active-path scans pass with a clean worktree.

- [x] 4.1.1 Parse every revised PowerShell script with Windows PowerShell 5.1-compatible syntax.
- [x] 4.1.2 Run all reference-profile and obsolete-artifact negative probes.
- [x] 4.1.3 Run `cargo fmt`, workspace tests, and workspace clippy with warnings denied.
- [x] 4.1.4 Run detailed task validation for this change and affected detailed active changes.
- [x] 4.1.5 Run `openspec validate --all --strict` and active-path Windows 10 requirement scan.
- [x] 4.1.6 Verify no Registry, rollback metadata, reboot, archive, or external-service mutation occurred.

### 4.2 Freeze the migrated release candidate

**目的：** Bind future external evidence to the fully migrated and validated harness revision.
**輸入：** Passed 4.1 evidence and committed migration implementation.
**產出：** New immutable candidate manifest, refreshed package smoke, and final program lineage.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / Wave 4.
**Gate／Evidence：** `G-TRACE`, `G-SAFETY`; `evidence/candidate-handoff.json`.
**完成門檻：** Candidate revision contains the migrated harness, package hashes pass, installer dry-run is zero-mutation, and only physical/reviewer or separately authorized reboot evidence remains pending.

- [x] 4.2.1 Commit the migrated implementation and resolve its full immutable revision.
- [x] 4.2.2 Freeze release-candidate purpose and revision to the migrated harness commit.
- [x] 4.2.3 Build a non-overwriting six-binary release package and verify every manifest SHA-256.
- [x] 4.2.4 Run packaged installer dry-run and verify exact zero mutation.
- [x] 4.2.5 Refresh completion/program lineage and publish the truthful post-migration blocker set.
- [x] 4.2.6 Leave all changes active and unarchived.
