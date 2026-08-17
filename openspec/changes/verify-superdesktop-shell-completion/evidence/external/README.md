# External completion evidence

This directory accepts three normalized artifacts bound to the immutable revision in `../release-candidate.json` only:

- `reference-profile-lifecycle-installer.json`, produced by `scripts/finalize-shell-completion-reference-profile-evidence.ps1` after the exact Windows 11 ExplorerPatcher M0 collector and all installer phases pass.
- `physical-mixed-dpi.json`, produced by `scripts/finalize-shell-completion-physical-mixed-dpi-evidence.ps1` after the M0 physical collector and completion feature confirmation pass.
- `independent-review.json`, produced by `scripts/capture-shell-completion-independent-review-evidence.ps1` from an attributable independent review.

Use `scripts/m0-physical-mixed-dpi-confirmation.example.json`, `scripts/shell-completion-physical-confirmation.example.json`, and `scripts/shell-completion-independent-review.example.json` as the three manual-input templates. Copy them to non-example paths inside the repository and replace every placeholder before capture.

All raw M0 artifacts, installer phase files, confirmations, photos, and screenshots referenced by these normalized artifacts must remain inside the repository. Finalizers record repository-relative paths and SHA-256 values; the collector reopens and rehashes every referenced file.

Capture installer evidence in this order on the exact Windows 11 build 26200.9168＋ExplorerPatcher 26100.8457.70.3 profile, using one unchanged release build and one rollback-record path:

1. `DryRun` without `-Apply`; this emits the Shell/metadata non-mutation proof.
2. `Enable -Apply -ExplicitOptIn -ConfirmPlan <fingerprint>`.
3. Reboot, then capture `AfterReboot`.
4. `Rollback -Apply -ExplicitOptIn -ConfirmPlan <fingerprint>`.

Supply the same attributable `-OperatorName` and `-OperatorOrganization` to every installer phase. The reference-profile M0 collector also requires those two operator fields. Windows 10 artifacts are obsolete and rejected rather than translated.

Run the collector after each normalized artifact is added, or after all three exist:

```powershell
powershell -NoProfile -File scripts/collect-shell-completion-evidence.ps1 `
  -ExternalEvidenceDirectory openspec/changes/verify-superdesktop-shell-completion/evidence/external `
  -OutputPath openspec/changes/verify-superdesktop-shell-completion/evidence/current-rollup.json
```

Each admitted artifact changes only its mapped gates to `passed`; missing artifacts remain `external_pending`, so incremental recomputation never weakens the final gate. The collector rejects unknown JSON artifacts, wrong hosts, revisions other than the frozen candidate, production drift after the candidate, malformed or internally partial evidence, source/artifact hash drift, unresolved P0/P1 findings, incomplete interaction matrices, recovery slower than ten seconds, and inexact installer rollback. Evidence commits may descend from the candidate without invalidating the reviewed revision. Example confirmations are templates only and are rejected until every placeholder is replaced by attributable data.
