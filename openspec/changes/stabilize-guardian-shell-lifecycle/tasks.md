# Guardian and shell lifecycle stabilization tasks

## 1. Guardian identity and acceptance

### 1.1 Normalize equivalent executable path identities

**目的：** Eliminate false guardian rejection without weakening immutable executable identity.
**輸入：** Existing `canonical_file`, `process_identity`, sealed claim, and file identity checks.
**產出：** Normalized comparison helper and guardian lease tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-GUARDIAN-IDENTITY`; `evidence/1.1/*`.
**完成門檻：** Extended-prefix and casing variants pass only with the same volume/file identity; different files fail.

- [x] 1.1.1 Implement normalized case-insensitive Windows executable path comparison with extended-prefix handling.
- [x] 1.1.2 Retain immutable file identity as a mandatory independent guardian validation gate.
- [x] 1.1.3 Add equivalent-path, different-path, and different-file guardian identity tests.

### 1.2 Make child acceptance bounded and diagnosable

**目的：** Distinguish immediate guardian rejection from a genuine startup timeout.
**輸入：** Parent child-process handle, nonce acknowledgement, guardian `LeaseReject`, and admission ordering.
**產出：** Five-second dual-observation loop, precise diagnostics, and timing tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-GUARDIAN-ACCEPTANCE`; `evidence/1.2/*`.
**完成門檻：** Valid acceptance succeeds; early exit returns immediately; only a live non-accepting child times out; Explorer is not closed on failure.

- [x] 1.2.1 Observe the guardian process handle while polling the nonce-bound acknowledgement for at most five seconds.
- [x] 1.2.2 Preserve the typed lease rejection in guardian console diagnostics and distinguish early child exit in the parent.
- [x] 1.2.3 Add acceptance, early-exit, invalid-acknowledgement, and live-timeout tests.

## 2. Shell rollback and AppBar behavior

### 2.1 Reconstruct a safe default-Explorer rollback

**目的：** Ensure an exact owned registration can always return to the default Explorer shell.
**輸入：** Current registry value, admitted app path, rollback store, and installer transaction contracts.
**產出：** Exact owned-value recognizer, atomic rollback reconstruction, and fail-closed tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-ROLLBACK-RECONSTRUCTION`; `evidence/2.1/*`.
**完成門檻：** Missing records reconstruct only for the exact owned command; unknown shells are unchanged.

- [x] 2.1.1 Add an exact current-owned-shell value helper shared by registration and restoration paths.
- [x] 2.1.2 Atomically create a default-Explorer rollback record before guardian arming when the exact owned value lacks one.
- [x] 2.1.3 Add exact-owned, Explorer-default, malformed-owned, and third-party-shell reconstruction tests.

### 2.2 Make Explorer restoration idempotent

**目的：** Stop repeated `rollback record is unavailable` errors while preserving registry safety.
**輸入：** Restore plan, rollback record, observed registry state, and Explorer recovery command.
**產出：** Idempotent restore behavior and repeated-command tests.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-EXPLORER-RETURN`; `evidence/2.2/*`.
**完成門檻：** Owned shell restores once; already-default Explorer succeeds repeatedly; unknown shell remains fail-closed.

- [x] 2.2.1 Treat an already-default Explorer registration without a rollback record as a successful no-op.
- [x] 2.2.2 Restore an exact owned registration through a valid or reconstructed record and remove the record after verification.
- [x] 2.2.3 Add repeated return, missing-record, state-drift, and unknown-shell tests.

### 2.3 Keep expected AppBar fallback trace-only

**目的：** Preserve correct owned geometry without reporting a normal degraded capability as a console warning.
**輸入：** AppBar registration branch, action trace, and taskbar configuration errors.
**產出：** Trace-only fallback and source/runtime tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-APPBAR-FALLBACK`; `evidence/2.3/*`.
**完成門檻：** Fallback traces remain, stderr warning is absent, and genuine failures remain visible.

- [x] 2.3.1 Remove only the direct expected AppBar fallback warning while retaining both action trace markers.
- [x] 2.3.2 Verify owned monitor geometry remains active and usable when AppBar registration is unavailable.
- [x] 2.3.3 Add source and runtime assertions separating expected fallback from genuine taskbar errors.

## 3. Lifecycle integration

### 3.1 Prove guardian and Explorer lifecycle ordering

**目的：** Verify the release topology arms recovery before Explorer shutdown and restores Explorer exactly once.
**輸入：** Release app/guardian, lifecycle fixture, registry/rollback isolation, process watchdog, and console logs.
**產出：** Two clean lifecycle reports bound to one candidate hash.
**依賴：** 1.2, 2.2, 2.3.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-LIFECYCLE-PHYSICAL`, `G-NO-CRASH`; `evidence/3.1/*`.
**完成門檻：** Guardian accepts, Explorer shutdown occurs afterward, abnormal parent exit recovers Explorer, and reported signatures are absent twice.

- [x] 3.1.1 Extend the guardian-parent/lifecycle fixture to record acceptance, ordering, recovery, and console signatures.
- [x] 3.1.2 Run the release lifecycle fixture twice against one candidate hash with isolated rollback state.
- [x] 3.1.3 Scan both runs for guardian rejection, child timeout, AppBar warning, missing rollback, panic, and Explorer recovery duplication.

## 4. Admission, packaging, and traceability

### 4.1 Pass automated and specification gates

**目的：** Admit the integrated lifecycle change with no warnings, placeholders, or contract drift.
**輸入：** Completed implementation, tests, OpenSpec artifacts, and release candidate.
**產出：** Format, tests, Clippy, release, strict OpenSpec, and detailed-task results.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-AUTOMATED`, `G-OPENSPEC`; `evidence/4.1/*`.
**完成門檻：** Every command exits zero and all 24 leaves have a unique passed evidence record.

- [x] 4.1.1 Run formatting plus focused and locked/offline workspace tests.
- [x] 4.1.2 Run Clippy warnings-as-errors and the locked/offline release build.
- [ ] 4.1.3 Run strict OpenSpec, detailed-task, placeholder, contradiction, and 24-record evidence validation.

### 4.2 Package and integrate the candidate

**目的：** Deliver the verified guardian/app pair through the normal installer and parent submodule.
**輸入：** Passing release candidate, clean tracked worktrees, and installer build.
**產出：** Nested commit, parent pointer commit, installer hash comparison, and final evidence summary.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / wave 5.
**Gate／Evidence：** `G-PACKAGE`, `G-TRACE`; `evidence/4.2/*`.
**完成門檻：** Installer builds without launch, packaged binaries match admitted hashes, tracked worktrees are clean, and the change remains unarchived.

- [ ] 4.2.1 Commit the nested implementation and update the parent submodule pointer without staging unrelated files.
- [ ] 4.2.2 Build the installer without launch and compare packaged app/guardian hashes with the admitted release binaries.
- [ ] 4.2.3 Write the final evidence summary/index and rerun strict validation plus tracked-status checks.
