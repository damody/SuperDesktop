## 1. Baseline and evidence contract

### 1.1 Freeze minimized-window and repository baseline

**目的：** Record current revisions, dirty boundaries, minimize actions, task snapshot flow, and native placement assumptions.
**輸入：** Approved design, parent/nested repositories, Win32 docs, existing taskbar tests.
**產出：** Baseline report, evidence schema, and append-only index.
**依賴：** None.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** BASE-01; tasks 1.1.1–1.1.3 in `evidence/index.jsonl`.
**完成門檻：** Exact revisions, preserved untracked roots, current behavior, native contract, and evidence fields are recorded.

- [x] 1.1.1 Record parent/nested revisions and preserve every pre-existing unrelated dirty path.
- [x] 1.1.2 Record minimize, restore, snapshot, grouping, and owned-shell mode call paths.
- [x] 1.1.3 Create and validate the unique evidence schema and append-only index contract.

## 2. Platform shelf implementation

### 2.1 Implement identity-safe minimized placement

**目的：** Hide only the iconic representation of an exact eligible window while retaining its owned taskbar model.
**輸入：** `owned-minimized-window-shelf` spec, `taskbar.rs`, Win32 placement contract.
**產出：** Placement builder, exact-identity adapter, typed outcomes, and platform tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** PLATFORM-01; tasks 2.1.1–2.1.4.
**完成門檻：** Adapter preserves normal/max placement and iconic state, rejects every ineligible/stale fixture, and is idempotent.

- [x] 2.1.1 Add exact iconic-window `ShowWindowAsync(SW_HIDE)` admission without placement or style mutation.
- [x] 2.1.2 Implement HWND/PID/stable-identity and eligibility revalidation before asynchronous hide.
- [x] 2.1.3 Integrate immediate shelving into the exact-identity Minimize action without altering other window actions.
- [x] 2.1.4 Add platform unit/source tests for placement preservation, exclusions, stale identity, and forbidden geometry/style paths.

### 2.2 Implement bounded shelf reconciliation

**目的：** Cover application-originated minimization without repeated placement or console flooding.
**輸入：** Platform adapter, owned task snapshots, shell-mode boundary.
**產出：** Cache reducer, runtime reconciliation, contextual diagnostics, and reducer tests.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** RUNTIME-01; tasks 2.2.1–2.2.4.
**完成門檻：** New minimized identities shelve once, restored/retired identities prune, failures log once per episode, and preview never mutates.

- [x] 2.2.1 Implement pure cache planning for new, retained, failed, restored, retired, and reused identities.
- [x] 2.2.2 Reconcile the existing 50 ms owned-shell task snapshot before grouping while retaining minimized task models.
- [x] 2.2.3 Emit identity-scoped one-shot console errors and clear suppression after an eligibility transition.
- [x] 2.2.4 Add reducer and composition tests for idempotence, retry, cached taskbar retention, preview isolation, and asynchronous hide.

## 3. Physical GUI verification

### 3.1 Add an ordinary-window minimize fixture and UTIT case

**目的：** Prove taskbar-owned and application-owned minimization hide the desktop tile and restore exact geometry.
**輸入：** Release candidate, taskbar fixture patterns, UI Automation, Win32 placement observation.
**產出：** Fixture binary, headful script, catalog/manifest entry, reports, screenshots, and recovery records.
**依賴：** 2.2 and source compilation.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** GUI-01; tasks 3.1.1–3.1.4.
**完成門檻：** The fixture exercises both minimize origins, stays iconic/hidden/taskbar-present, restores exact bounds, and cleans up on every exit.

- [x] 3.1.1 Add a deterministic ordinary top-level fixture with external minimize/restore controls and known normal bounds.
- [x] 3.1.2 Implement bounded headful observation for iconic hidden state, taskbar presence, placement invariants, restore bounds, and process survival.
- [x] 3.1.3 Implement `finally` recovery for fixture, SuperDesktop, Explorer, environment, and Winlogon Shell state.
- [x] 3.1.4 Register the mandatory case and add catalog, parser, manifest, and artifact contract tests.

### 3.2 Execute the final-candidate physical gate twice

**目的：** Exclude transient success and stale-binary evidence.
**輸入：** Built final candidate and registered focused case.
**產出：** Two clean reports with identical candidate hash and complete geometry/lifecycle assertions.
**依賴：** 3.1 and release candidate build.
**Owner／Wave：** Primary agent / Wave 4.
**Gate／Evidence：** GUI-02; tasks 3.2.1–3.2.2.
**完成門檻：** Two consecutive runs pass every mandatory assertion with the same final candidate hash.

- [x] 3.2.1 Run and index the first clean owned-shell minimized-window result.
- [x] 3.2.2 Run and index the second clean owned-shell minimized-window result.

## 4. Source and package gates

### 4.1 Run deterministic source quality gates

**目的：** Exclude regressions across the SuperDesktop workspace.
**輸入：** Completed platform/runtime/UTIT implementation.
**產出：** Formatting, parser, focused, workspace, Clippy, and release results.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / Wave 4.
**Gate／Evidence：** SRC-01; tasks 4.1.1–4.1.3.
**完成門檻：** Every command exits zero and Clippy reports no denied warning.

- [x] 4.1.1 Run Rust formatting, PowerShell parser checks, and focused shelf/runtime/UTIT tests.
- [x] 4.1.2 Run the full workspace test suite and all-target Clippy with warnings denied.
- [x] 4.1.3 Build the exact final SuperDesktop release candidate and record its hash.

### 4.2 Build and verify the integrated installer

**目的：** Package the GUI-verified candidate without changing unrelated work.
**輸入：** Passing source/GUI gates, committed nested source, staged parent gitlink.
**產出：** Installer, extracted SuperDesktop payload, hashes, provenance, and restored dirty paths.
**依賴：** 3.2 and 4.1.
**Owner／Wave：** Primary agent / Wave 5.
**Gate／Evidence：** REL-01; tasks 4.2.1–4.2.3.
**完成門檻：** Installer build passes, embedded hash equals the GUI candidate, and unrelated paths are restored unchanged.

- [x] 4.2.1 Commit intended nested changes and synchronize only the SuperDesktop gitlink in the parent.
- [x] 4.2.2 Build the all-component installer without launch and record its hash.
- [x] 4.2.3 Extract the packaged SuperDesktop binary and prove candidate, GUI, and embedded hashes are identical.

## 5. Final traceability and integration

### 5.1 Close requirements, evidence, and repository state

**目的：** Deliver an auditable complete change with no unresolved blocking finding.
**輸入：** All platform, runtime, GUI, source, release, package, and repository evidence.
**產出：** Traceability, final review, completed tasks, strict validation, and final parent/nested commits.
**依賴：** 4.2.
**Owner／Wave：** Primary agent / Wave 6.
**Gate／Evidence：** REVIEW-01; tasks 5.1.1–5.1.4.
**完成門檻：** Every leaf has unique passed evidence, all scenarios map to gates, OpenSpec validates strictly, zero P0/P1 remains, and tracked repositories are clean.

- [x] 5.1.1 Map every proposal outcome, decision, requirement, scenario, task, gate, command, and artifact.
- [x] 5.1.2 Review identity safety, placement preservation, cache bounds, console behavior, preview isolation, GUI cleanup, and unrelated diffs.
- [x] 5.1.3 Run strict OpenSpec, detailed-task, evidence-schema, formatting, and final repository validation.
- [x] 5.1.4 Commit completed nested evidence and synchronize the final parent gitlink.
