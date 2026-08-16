# External completion evidence

This directory accepts three normalized artifacts bound to the immutable revision in `../release-candidate.json` only:

- `windows10-lifecycle-installer.json`, produced by `scripts/finalize-shell-completion-windows10-evidence.ps1` after the M0 Windows 10 collector and all installer phases pass.
- `physical-mixed-dpi.json`, produced by `scripts/finalize-shell-completion-physical-mixed-dpi-evidence.ps1` after the M0 physical collector and completion feature confirmation pass.
- `independent-review.json`, produced by `scripts/capture-shell-completion-independent-review-evidence.ps1` from an attributable independent review.

Use `scripts/m0-physical-mixed-dpi-confirmation.example.json`, `scripts/shell-completion-physical-confirmation.example.json`, and `scripts/shell-completion-independent-review.example.json` as the three manual-input templates. Copy them to non-example paths inside the repository and replace every placeholder before capture.

All raw M0 artifacts, installer phase files, confirmations, photos, and screenshots referenced by these normalized artifacts must remain inside the repository. Finalizers record repository-relative paths and SHA-256 values; the collector reopens and rehashes every referenced file.

Capture installer evidence in this order on Windows 10 build 19045, using one unchanged release build and one rollback-record path:

1. `DryRun` without `-Apply`; this emits the Shell/metadata non-mutation proof.
2. `Enable -Apply -ExplicitOptIn -ConfirmPlan <fingerprint>`.
3. Reboot, then capture `AfterReboot`.
4. `Rollback -Apply -ExplicitOptIn -ConfirmPlan <fingerprint>`.

Supply the same attributable `-OperatorName` and `-OperatorOrganization` to every installer phase. The Windows 10 M0 collector also requires those two operator fields.

After all three normalized artifacts exist for the frozen candidate revision, run:

```powershell
powershell -NoProfile -File scripts/collect-shell-completion-evidence.ps1 `
  -ExternalEvidenceDirectory openspec/changes/verify-superdesktop-shell-completion/evidence/external `
  -OutputPath openspec/changes/verify-superdesktop-shell-completion/evidence/current-rollup.json
```

The collector rejects wrong hosts, revisions other than the frozen candidate, production drift after the candidate, partial evidence, source/artifact hash drift, unresolved P0/P1 findings, incomplete interaction matrices, recovery slower than ten seconds, and inexact installer rollback. Evidence commits may descend from the candidate without invalidating the reviewed revision. Example confirmations are templates only and are rejected until every placeholder is replaced by attributable data.
