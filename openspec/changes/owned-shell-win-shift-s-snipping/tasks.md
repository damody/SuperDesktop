## 1. Baseline and evidence contract

### 1.1 Freeze shortcut and platform baseline

**目的：** Record the current hotkey, runtime, protocol, and dirty-worktree boundaries before implementation.
**輸入：** Approved design, current parent/nested revisions, existing shell-hotkey tests.
**產出：** Baseline artifact, evidence schema, and append-only index.
**依賴：** None.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** BASE-01; `evidence/index.jsonl` tasks 1.1.1–1.1.3.
**完成門檻：** Revisions, call paths, exclusions, and unique evidence record fields are written and parse successfully.

- [x] 1.1.1 Record parent/nested revisions and preserve all pre-existing dirty paths.
- [x] 1.1.2 Record the reducer, hook, runtime dispatch, and native activation baseline.
- [x] 1.1.3 Create and validate the unique append-only evidence index contract.

## 2. Owned-shell shortcut implementation

### 2.1 Extend the bounded shell-hotkey reducer

**目的：** Route `Win+Shift+S` once without changing existing chords or hook lifetime.
**輸入：** `owned-shell-screen-snipping-shortcut` spec and `shell_hotkey.rs`.
**產出：** New action/code round trip, reducer mapping, and focused tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** HOTKEY-01; tasks 2.1.1–2.1.4.
**完成門檻：** Initial, repeat, key-up, search, and unsupported modifier matrices pass with no hook-side launch work.

- [x] 2.1.1 Add `OpenScreenSnip` to the bounded action bit/code round trip.
- [x] 2.1.2 Map Windows+Shift+S separately from Windows+S in the pure reducer.
- [x] 2.1.3 Preserve repeat suppression, matching key-up consumption, and Control/Alt pass-through.
- [x] 2.1.4 Add focused reducer and queue tests for every shortcut scenario.

### 2.2 Activate the fixed built-in Windows protocol

**目的：** Open the native image-snipping overlay without Explorer or path discovery.
**輸入：** Fixed observed `ms-screenclip:///?source=HotKey` design and platform Win32 boundary.
**產出：** Narrow fallible `IApplicationActivationManager::ActivateApplication` helper and source-contract tests.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** NATIVE-01; tasks 2.2.1–2.2.3.
**完成門檻：** Fixed URI admission compiles, retains no process handle, rejects all fallback mechanisms, and returns typed errors.

- [x] 2.2.1 Implement fixed-AUMID `ActivateApplication` admission with `ms-screenclip:///?source=HotKey` and `AO_NONE`.
- [x] 2.2.2 Keep URI activation outside the low-level hook and expose a fallible platform result.
- [x] 2.2.3 Add source contracts forbidding Explorer, executable lookup, key reinjection, and third-party fallback.

### 2.3 Dispatch and diagnose on the GPUI foreground path

**目的：** Connect the queued action to native activation without panic or UI-thread reentrancy.
**輸入：** New action and platform helper, existing refresh dispatch.
**產出：** Runtime match arm, success traces, and scoped console failure.
**依賴：** 2.2.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** RUNTIME-01; tasks 2.3.1–2.3.3.
**完成門檻：** Dispatch is exhaustive, emits requested/accepted traces, reports rejection to console, and preview mode remains unhooked.

- [x] 2.3.1 Dispatch `OpenScreenSnip` from the existing foreground refresh loop.
- [x] 2.3.2 Emit requested/accepted trace events and scoped console errors without unwrap or panic.
- [x] 2.3.3 Add composition tests proving owned-shell-only hook installation and exhaustive runtime routing.

## 3. Physical shortcut verification

### 3.1 Add a privacy-preserving headful UTIT case

**目的：** Prove a real owned-shell chord uses a bounded verified Explorer broker, opens the built-in overlay, and returns to Explorer-absent state.
**輸入：** Release candidate, existing UTIT input helpers/catalog patterns, runtime traces.
**產出：** Headful PowerShell case, catalog entry, hashed JSON/log evidence without screen-content images.
**依賴：** 2.3.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** GUI-01; tasks 3.1.1–3.1.4.
**完成門檻：** Test starts and ends Explorer-absent, sends physical keys, observes the temporary verified broker plus signed/system overlay identity, dismisses with Escape, proves app survival, and stores no screenshot.

- [x] 3.1.1 Implement physical Windows+Shift+S and Escape input with bounded watchdog/recovery.
- [x] 3.1.2 Observe the temporary verified Explorer broker and built-in clipping surface/process, then prove both disappear after Escape.
- [x] 3.1.3 Register the mandatory owned-shell broker-bounded case and its privacy-preserving artifacts in UTIT.
- [x] 3.1.4 Add UTIT catalog/report contract tests for the new case.

### 3.2 Execute the focused physical gate twice

**目的：** Exclude transient success and stale candidate evidence.
**輸入：** Built release candidate and registered focused case.
**產出：** Two clean reports with candidate hash, trace, identity, dismissal, recovery, and survival results.
**依賴：** 3.1 and source validation.
**Owner／Wave：** Primary agent / Wave 4.
**Gate／Evidence：** GUI-02; tasks 3.2.1–3.2.2.
**完成門檻：** Two consecutive clean-launch runs pass every mandatory assertion with the same candidate hash.

- [ ] 3.2.1 Run and index the first owned-shell broker-bounded physical shortcut result.
- [ ] 3.2.2 Run and index the second clean-launch physical shortcut result.

## 4. Source and release gates

### 4.1 Run deterministic source quality gates

**目的：** Prove no regression across the SuperDesktop workspace.
**輸入：** Completed implementation and UTIT source.
**產出：** Formatting, focused, workspace, and Clippy results.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** SRC-01; tasks 4.1.1–4.1.3.
**完成門檻：** Every command returns exit 0 and Clippy reports no denied warning.

- [x] 4.1.1 Run Rust/PowerShell formatting or parser checks and focused shortcut tests.
- [x] 4.1.2 Run the complete SuperDesktop workspace test suite.
- [x] 4.1.3 Run workspace Clippy for all targets with warnings denied.

### 4.2 Build and package the exact verified candidate

**目的：** Produce a traceable installer containing the headful-tested binary.
**輸入：** Passing source and physical gates, committed nested source, staged parent gitlink.
**產出：** Release binary, installer, embedded-binary extraction, hashes, and provenance report.
**依賴：** 3.2 and 4.1.
**Owner／Wave：** Primary agent / Wave 5.
**Gate／Evidence：** REL-01; tasks 4.2.1–4.2.3.
**完成門檻：** Release and installer builds pass, embedded binary hash equals candidate hash, and unrelated files remain unstaged.

- [ ] 4.2.1 Build the SuperDesktop release candidate and record its hash.
- [ ] 4.2.2 Build the all-component SuperExplorer installer without launching it and record its hash.
- [ ] 4.2.3 Extract the packaged SuperDesktop binary and prove hash/provenance equality.

## 5. Final review and integration

### 5.1 Close traceability and repository state

**目的：** Deliver a complete auditable change and synchronize the parent repository safely.
**輸入：** All implementation, test, GUI, release, and package evidence.
**產出：** Requirement traceability, final review, strict validation, nested commits, and parent gitlink commits.
**依賴：** 4.2.
**Owner／Wave：** Primary agent / Wave 6.
**Gate／Evidence：** REVIEW-01; tasks 5.1.1–5.1.4.
**完成門檻：** Every leaf has one valid evidence record, zero P0/P1 remains, OpenSpec validates strictly, and only intended tracked paths are committed.

- [ ] 5.1.1 Map every requirement/scenario to tasks, gates, and immutable evidence.
- [ ] 5.1.2 Review hook bounds, protocol security, error paths, privacy, regressions, and unrelated changes.
- [ ] 5.1.3 Run strict OpenSpec, detailed-task, evidence, formatting, and diff validation.
- [ ] 5.1.4 Commit nested results and synchronize only the SuperDesktop gitlink in the parent repository.
