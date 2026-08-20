## 1. Baseline and evidence contract

### 1.1 Freeze keyboard and repository baseline

**目的：** Record the current reducer, runtime callback, repository revisions, and dirty-path boundary before editing.
**輸入：** Approved design, current parent/nested repositories, existing hotkey and Start tests.
**產出：** Baseline report and append-only evidence index.
**依賴：** None.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** BASE-01; `evidence/index.jsonl` tasks 1.1.1–1.1.2.
**完成門檻：** Revisions, relevant call paths, preserved dirty paths, and evidence fields are recorded.

- [x] 1.1.1 Record parent/nested revisions, dirty paths, hook reducer, action queue, and Start callback route.
- [x] 1.1.2 Create the evidence directory, baseline report, and unique JSONL evidence records for the baseline gate.

## 2. Standalone Windows-key implementation

### 2.1 Implement the release-time gesture reducer

**目的：** Recognize one standalone left/right Win gesture without regressing chords.
**輸入：** `owned-win-key-start-toggle` spec and `shell_hotkey.rs`.
**產出：** Atomic gesture state, `ToggleStart` queue code, hook integration, and reducer tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** HOTKEY-01; tasks 2.1.1–2.1.3.
**完成門檻：** Left/right, repeat, supported/unsupported chord, dual-Win, and mismatched-release matrices pass.

- [x] 2.1.1 Add the stable `ToggleStart` action/code and resettable standalone-Win state.
- [x] 2.1.2 Integrate candidate, cancellation, matching-release consumption, and one-shot dispatch into the bounded hook.
- [x] 2.1.3 Add focused reducer and queue tests for every input-state scenario.

### 2.2 Route the action through the owned Start callback

**目的：** Share the pointer Start lifecycle and keep dispatch panic-safe.
**輸入：** New queued action and existing `TaskbarCallbacks::start` composition.
**產出：** Runtime match arm, trace/error behavior, and composition contract tests.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** RUNTIME-01; tasks 2.2.1–2.2.2.
**完成門檻：** Dispatch invokes the shared callback outside temporary borrows, traces success, reports absence, and compiles exhaustively.

- [x] 2.2.1 Dispatch `ToggleStart` through the first live taskbar Start callback with scoped trace and console error.
- [x] 2.2.2 Add source-contract tests for shared lifecycle routing, shell-only hook scope, and forbidden delegated/synthetic Start paths.

## 3. Physical GUI verification

### 3.1 Add and execute the owned-shell UTIT case

**目的：** Prove real left-Win gestures open and close the owned Start surface while recovery remains safe.
**輸入：** Release candidate, existing owned-shell UTIT/recovery helpers, Start window traces.
**產出：** Headful script, catalog/contract entry, two clean reports, screenshots, and recovery records.
**依賴：** 2.2 and source compilation.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** GUI-01; tasks 3.1.1–3.1.3.
**完成門檻：** Two consecutive runs observe closed→open→closed, process survival, and restoration of the prior shell/Explorer state.

- [x] 3.1.1 Implement bounded real-key injection, Start open/close observation, screenshots, and `finally` recovery in a focused UTIT script.
- [x] 3.1.2 Register the case and add parser/catalog tests for mandatory assertions and artifacts.
- [x] 3.1.3 Run the focused GUI case twice and index candidate hashes, traces, screenshots, survival, and recovery results.

## 4. Source and package gates

### 4.1 Run deterministic source quality gates

**目的：** Exclude regressions across the nested workspace.
**輸入：** Completed implementation and UTIT source.
**產出：** Formatting/parser, focused test, workspace test, Clippy, and release logs.
**依賴：** 3.1.2.
**Owner／Wave：** Primary agent / Wave 4.
**Gate／Evidence：** SRC-01; tasks 4.1.1–4.1.3.
**完成門檻：** Every command exits zero and denied-warning Clippy is clean.

- [x] 4.1.1 Run Rust formatting, PowerShell parser checks, and focused hotkey/runtime/UTIT tests.
- [x] 4.1.2 Run the full SuperDesktop workspace tests and workspace Clippy for all targets with warnings denied.
- [x] 4.1.3 Build the exact SuperDesktop release candidate and record its hash.

### 4.2 Build and verify the integrated installer

**目的：** Package the verified nested candidate through the parent installer without disturbing unrelated work.
**輸入：** Passing source/GUI gates, committed nested source, staged parent gitlink.
**產出：** Installer, hashes, provenance report, and restored unrelated dirty paths.
**依賴：** 3.1.3 and 4.1.
**Owner／Wave：** Primary agent / Wave 5.
**Gate／Evidence：** REL-01; tasks 4.2.1–4.2.2.
**完成門檻：** Installer build exits zero, package provenance names the nested commit/candidate, and unrelated paths remain unchanged.

- [x] 4.2.1 Commit intended nested changes and synchronize only the SuperDesktop gitlink plus intended integration artifacts in the parent.
- [x] 4.2.2 Build the all-component installer without launch and record installer/candidate hashes and repository status.

## 5. Final traceability review

### 5.1 Close specifications and evidence

**目的：** Deliver an auditable, strictly valid change with no unresolved blocking defect.
**輸入：** All implementation, GUI, source, release, package, and repository evidence.
**產出：** Requirement traceability, final review, completed tasks, strict validation, and final integration commit.
**依賴：** 4.2.
**Owner／Wave：** Primary agent / Wave 6.
**Gate／Evidence：** REVIEW-01; tasks 5.1.1–5.1.3.
**完成門檻：** Every leaf has unique passed evidence, every scenario maps to a gate, OpenSpec validates strictly, zero P0/P1 remains, and tracked repositories are clean.

- [x] 5.1.1 Map proposal, decisions, requirements, scenarios, tasks, gates, commands, and artifacts in the evidence index.
- [x] 5.1.2 Review hook bounds, state recovery, callback borrowing, shell isolation, GUI cleanup, regressions, and unrelated diffs.
- [x] 5.1.3 Run strict OpenSpec, detailed-task, evidence-schema, formatting, and final repository validation; commit the completed change.
