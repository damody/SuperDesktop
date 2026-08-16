# External completion evidence

This directory accepts three normalized, revision-bound artifacts only:

- `windows10-lifecycle-installer.json`, produced by `scripts/finalize-shell-completion-windows10-evidence.ps1` after the M0 Windows 10 collector and all installer phases pass.
- `physical-mixed-dpi.json`, produced by `scripts/finalize-shell-completion-physical-mixed-dpi-evidence.ps1` after the M0 physical collector and completion feature confirmation pass.
- `independent-review.json`, produced by `scripts/capture-shell-completion-independent-review-evidence.ps1` from an attributable independent review.

After all three exist for the current full Git revision, run:

```powershell
powershell -NoProfile -File scripts/collect-shell-completion-evidence.ps1 `
  -ExternalEvidenceDirectory openspec/changes/verify-superdesktop-shell-completion/evidence/external `
  -OutputPath openspec/changes/verify-superdesktop-shell-completion/evidence/current-rollup.json
```

The collector rejects wrong hosts, stale revisions, partial evidence, unresolved P0/P1 findings, incomplete interaction matrices, recovery slower than ten seconds, and inexact installer rollback. Example confirmations are templates only and are rejected until every placeholder is replaced by attributable data.
