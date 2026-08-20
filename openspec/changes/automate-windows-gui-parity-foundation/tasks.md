## 1. Build the parity authority

### 1.1 Compiled surface manifest

**目的：** Establish closed first-wave owned-surface coverage.
**輸入：** Approved design, current UTIT catalog, production surface inventory.
**產出：** Typed manifest model, first-wave entries, and closure tests.
**依賴：** None.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** G-MANIFEST; `evidence/manifest-tests.log`.
**完成門檻：** Duplicate/missing/orphan surfaces fail and every first-wave entry maps bidirectionally to catalog coverage.

- [x] 1.1.1 Add typed GUI surface, variant, geometry-rule, artifact, and Explorer-policy models.
- [x] 1.1.2 Populate first-wave entries for taskbar, Start, system flyouts, overflow, contexts, Jump Lists, previews, task view, and Alt-Tab.
- [x] 1.1.3 Add manifest uniqueness, rule-validity, and manifest-to-catalog closure tests.

### 1.2 Normalized measurement validator

**目的：** Make geometry and ratio results machine authoritative.
**輸入：** Manifest types and existing headful JSON reports.
**產出：** `gui-parity-measurement/v1` model, legacy adapter, validator, and diagnostics.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** G-MEASUREMENT; `evidence/measurement-tests.log`.
**完成門檻：** One physical-to-DIP conversion evaluates bounds, ranges, ratios, containment, overlap, targets, and malformed/stale failures.

- [x] 1.2.1 Implement normalized physical/DPI/region/control/action report types.
- [x] 1.2.2 Implement absolute, range, ratio, containment, non-overlap, and minimum-hit-target evaluation.
- [x] 1.2.3 Add deterministic diagnostics and malformed, stale, boundary, and high-DPI tests.

### 1.3 Explorer-free policy

**目的：** Prevent all normal product routes from depending on Explorer.
**輸入：** Production/recovery module inventory.
**產出：** Path-aware source auditor and negative fixtures.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** G-EXPLORER-FREE; `evidence/explorer-policy.log`.
**完成門檻：** Only guardian recovery, installer rollback, test watchdog, and explicit Return to default Explorer references are admitted.

- [x] 1.3.1 Implement allowed-path/action classification and forbidden-token scanning.
- [x] 1.3.2 Add negative fixtures for composition, provider, Settings, and SuperExplorer launch delegation.
- [x] 1.3.3 Add the auditor as a mandatory UTIT smoke case.

## 2. Convert and correct first-wave shell chrome

### 2.1 Shared Windows metrics

**目的：** Remove duplicated first-wave geometry constants from views/composition/scripts.
**輸入：** Manifest reference rules and current formulas.
**產出：** Shared `WindowsGuiMetrics` constants/formulas consumed by production.
**依賴：** 1.1 and 1.2.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** G-METRICS; `evidence/metrics-tests.log`.
**完成門檻：** Canonical row/target/width/padding/radius/gap values have one source and matrix tests pass at supported DPI/rows.

- [x] 2.1.1 Add shared taskbar, popup, flyout, context, overflow, preview, Start, task-view, and Alt-Tab metrics.
- [x] 2.1.2 Replace duplicate production constants/formulas with shared metrics without changing authority boundaries.
- [x] 2.1.3 Add geometry matrix tests for 96/120/144/168/192 DPI, rows 1–3, and negative monitor origins.

### 2.2 Normalize headful adapters

**目的：** Convert first-wave scripts to one report schema and eliminate local magic thresholds.
**輸入：** Validator and existing headful adapters.
**產出：** Normalized reports and manifest-derived expected rules.
**依賴：** 1.2 and 2.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** G-ADAPTERS; `evidence/adapter-validation.log`.
**完成門檻：** Every converted case reports physical bounds/DPI/named regions/actions/Explorer state and passes schema validation.

- [ ] 2.2.1 Convert taskbar, Start, and taskbar context/settings adapters.
- [ ] 2.2.2 Convert system flyout, notification overflow/center, and calendar adapters.
- [x] 2.2.3 Convert Jump List, hover preview, task-view, and Alt-Tab adapters or record explicit follow-up manifest dispositions.

### 2.3 First automated correction loop

**目的：** Fix first-wave geometry and interaction failures produced by the new authority.
**輸入：** Converted reports and release binaries.
**產出：** Corrected production geometry/layout plus before/after evidence.
**依賴：** 2.1 and 2.2.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** G-GUI-PARITY; `evidence/gui-parity/`.
**完成門檻：** Current-host first-wave mandatory entries pass or have valid hardware/capacity dispositions, with no hidden uncovered surface.

- [x] 2.3.1 Run the full first-wave matrix and index every failing manifest rule.
- [x] 2.3.2 Correct taskbar/Start/context/overflow proportions and popup anchors from measured deltas.
- [x] 2.3.3 Correct system flyout/notification/preview/Jump List proportions and popup lifecycles from measured deltas.
- [ ] 2.3.4 Re-run the matrix and retain normalized reports, screenshots, traces, hashes, and recovery results.
- [x] 2.3.5 Remove the implicit SuperExplorer taskbar entry and verify that only explicit pins or running windows use a normal task slot.

## 3. Close integration gates

### 3.1 Quality and evidence

**目的：** Prove the foundation is safe, complete, and reusable by later waves.
**輸入：** Completed implementation and GUI evidence.
**產出：** Passing quality logs, strict OpenSpec validation, and evidence index.
**依賴：** 2.3.
**Owner／Wave：** Primary agent / Wave 4.
**Gate／Evidence：** G-FMT, G-CHECK, G-TEST, G-CLIPPY, G-OPENSPEC; `evidence/quality/` and `evidence/evidence-index.json`.
**完成門檻：** All automated gates pass, every task has unique evidence/subcheck, conditional gaps remain explicit, and unrelated worktree changes are preserved.

- [x] 3.1.1 Run formatting, focused tests, locked offline workspace check/test, and Clippy warnings-as-errors.
- [x] 3.1.2 Run strict OpenSpec validation, placeholder/contradiction scan, and report/hash validation.
- [x] 3.1.3 Review Explorer allowances, manifest closure, final diff, and evidence lineage; resolve all P0/P1 findings.
- [x] 3.1.4 Write the evidence index and mark only passed or evidence-backed conditional leaves complete.
