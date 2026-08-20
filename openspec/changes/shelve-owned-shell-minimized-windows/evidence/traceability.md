# Traceability

| Requirement / scenario | Implementation | Task / gate | Evidence |
|---|---|---|---|
| Taskbar minimize hides legacy tile | exact action + immediate shelf reconcile | 2.1.3, 3.2.1–3.2.2 / PLATFORM-01, GUI-02 | gui-run-8/9 iconic+hidden |
| Application minimizes itself | 50 ms authoritative snapshot reconciliation | 2.2.2, 3.1.1–3.2.2 / RUNTIME-01, GUI-02 | fixture WM_APP minimize and gui-run-8/9 |
| Multiple minimized apps | per-stable-identity bounded episode map | 2.2.1 / RUNTIME-01 | reducer matrix and startup multi-window shelf traces |
| Preview retains Explorer ownership | `shell.then_some(...)` boundary | 2.2.2, 2.2.4 / RUNTIME-01 | runtime source contract |
| Taskbar restore preserves exact bounds | cached task model + existing SW_RESTORE | 2.2.2, 3.1.2 / GUI-01 | gui-run-8/9 taskbar_restore_exact |
| Application restore preserves exact bounds | hidden iconic live HWND + fixture SW_RESTORE | 3.1.1–3.2.2 / GUI-02 | gui-run-8/9 application_restore_exact |
| Alt+Tab retains/restores hidden iconic HWND | ordinary snapshot retains hidden HWND; existing restore action | 2.2.2, 5.1.2 / RUNTIME-01, REVIEW-01 | source review and workspace tests |
| Retired/reused HWND rejected | immediate PID/stable identity re-snapshot | 2.1.2, 2.1.4 / PLATFORM-01 | platform exact-identity tests |
| Hidden/restored/tool/cloaked/transient excluded | `minimized_shelf_eligible` | 2.1.2, 2.1.4 / PLATFORM-01 | eligibility matrix |
| Repeated hidden snapshots idempotent | Shelved cached episode retained only hidden+iconic | 2.2.1, 2.2.4 / RUNTIME-01 | reducer idempotence test |
| Transition allows retry | cache pruned on restore/retire/visible rewrite | 2.2.1 / RUNTIME-01 | prune/retry reducer tests |
| Observation failure reports once | Failed episode plus contextual `report_error` | 2.2.3 / RUNTIME-01 | failure episode reducer/source contract |
| Physical final candidate passes twice | focused UTIT with PMv2 geometry observer | 3.2.1–3.2.2 / GUI-02 | gui-run-8/9 |
| Cleanup restores host | `finally`, watchdog, shell snapshot | 3.1.3 / GUI-01 | recovery_verified and shell/explorer true in all runs |
| Package equals tested candidate | NSIS extraction and SHA256 equality | 4.1.3, 4.2.2–4.2.3 / SRC-01, REL-01 | package-gates.json |

## Correction lineage

- B-001 preserves failed runs 1–5. Run 1 corrected the taskbar active-state precondition. Runs 2–3 proved `SetWindowPlacement` clamps off-screen minimized workspace coordinates to `(-2,-2)`. Runs 4–5 exposed DPI observer virtualization and demonstrated that geometry mutation is unnecessary and risky.
- The final design uses no placement mutation: asynchronous hide plus an exact-identity cached task model. PMv2 observer runs 6–7 passed before commit.
- A-002 invalidated pre-commit binary provenance after the build embedded the nested commit identity. Final packaged-candidate runs 8–9 re-proved every GUI assertion on the exact extracted hash.
