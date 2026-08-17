# Complete SuperDesktop Windows Shell

## 1. Program contract and lineage

### 1.1 Freeze dependency order, ownership, and claims

**目的：** Define the exact completion program without broad or ambiguous Windows parity claims.
**輸入：** Parent proposal, approved completion design, nine child changes, and M0 safety contracts.
**產出：** Program design/spec, ordered child manifest, ownership, capability ledger, and limitation ledger.
**依賴：** Existing M0 foundation and approved design document.
**Owner／Wave：** Primary integrator / Program Wave P1.
**Gate／Evidence：** `G-ARCH`, `G-TRACE`, `G-SAFETY`; `PROGRAM.md` and program roll-up.
**完成門檻：** Every child appears exactly once in dependency order and every claim is classified truthfully.

- [x] 1.1.1 Add complete parent design, delta specification, and detailed tasks.
- [x] 1.1.2 Add the ordered nine-child program manifest and ownership/dependency table.
- [x] 1.1.3 Add implemented, owned-protocol, unavailable, and not-claimed capability classifications.
- [x] 1.1.4 Record invariant safety contracts that no child may weaken.

## 2. Production implementation integration

### 2.1 Roll up every locally implemented child

**目的：** Prove all production functionality and local verification are integrated and attributable.
**輸入：** Eight implementation changes, verification local roll-up, commits, and workspace tests.
**產出：** Machine-readable program roll-up with separate implementation and release fields.
**依賴：** 1.1 and completion child changes.
**Owner／Wave：** Primary integrator / Program Wave P2.
**Gate／Evidence：** Domain gates, `G-PERF`, `G-A11Y-I18N`, `G-TRACE`; `evidence/program-rollup.json`.
**完成門檻：** All eight production children are complete, verification local tasks pass, and workspace/OpenSpec validation is green.

- [x] 2.1.1 Verify contracts, desktop operations, and context-menu children are complete.
- [x] 2.1.2 Verify Start/search, advanced taskbar, and notification-area children are complete.
- [x] 2.1.3 Verify documented virtual-desktop and transactional-installer children are complete.
- [x] 2.1.4 Verify completion local roll-up/schema and all workspace commands pass.
- [x] 2.1.5 Emit `implementation_complete=true`, `release_allowed=false`, exact blockers, and no archive mutation.

## 3. Physical verification and release

### 3.1 Complete external gates and final disposition

**目的：** Convert the locally complete implementation into an evidence-approved release candidate.
**輸入：** Exact Windows 11 ExplorerPatcher profile, mixed-DPI hardware, reboot/rollback runs, and independent reviewer.
**產出：** Final verification child, recomputed program roll-up, and release disposition.
**依賴：** 2.1 and verification tasks 3.1.1–3.1.3.
**Owner／Wave：** Physical operator, independent reviewer, primary integrator / Program Wave P3.
**Gate／Evidence：** `G-SHELL-TAKEOVER`, `G-GUARDIAN-RECOVERY`, `G-INSTALL-ROLLBACK`, `G-DPI-MONITOR-PHYSICAL`, `G-REVIEW`.
**完成門檻：** All external gates pass and both child and parent derived roll-ups report release allowed.

- [ ] 3.1.1 Complete exact reference-profile lifecycle/installer reboot rollback and physical mixed-DPI evidence.
- [ ] 3.1.2 Complete independent architecture/security/accessibility review.
- [ ] 3.1.3 Recompute verification and parent roll-ups with `release_allowed=true`.
- [ ] 3.1.4 Run final strict/task validation while leaving changes unarchived.
