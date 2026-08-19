# SuperDesktop UTIT Runner Tasks

## 1. Catalog and command admission

### 1.1 Scaffold the isolated UTIT crate and typed model

**目的：** Establish a test-only binary and stable catalog/report types.
**輸入：** Approved design, workspace manifest, existing test script inventory.
**產出：** New crate, CLI shell, case/suite/prerequisite/recovery/result DTOs.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-UTIT-CATALOG`; crate/model test report.
**完成門檻：** Workspace compiles and every public enum/ID has deterministic serde and validation behavior.

- [x] 1.1.1 Add `superdesktop-utit` workspace crate, binary entrypoint, and module boundaries.
- [x] 1.1.2 Define stable case, suite, tier, prerequisite, recovery, terminal, run, and artifact types.
- [x] 1.1.3 Add serialization, stable-order, duplicate-ID, invalid-timeout, and suite-closure tests.

### 1.2 Implement canonical catalog and CLI discovery

**目的：** Inventory current gates without arbitrary command authority.
**輸入：** Typed model and maintained Cargo/OpenSpec/headful commands.
**產出：** Compiled catalog, `list`, dry-run resolution, path admission.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-UTIT-CATALOG`, `G-UTIT-ADMISSION`; catalog/admission report.
**完成門檻：** Every admitted argv resolves under approved roots; all escape and shell-string fixtures fail before spawn.

- [x] 1.2.1 Add smoke, shell-parity, full, hardware/external case catalog and tags.
- [x] 1.2.2 Implement CLI parsing for list/run/validate-report, suite, case/tag filters, output, and dry-run.
- [x] 1.2.3 Test canonical workspace/script/binary roots, traversal, duplicate filters, unknown cases, and partial-run detection.

## 2. Execution, preflight, and recovery

### 2.1 Build host preflight and truthful prerequisites

**目的：** Determine what the current Windows host can honestly execute.
**輸入：** Catalog prerequisites, Windows command observations, workspace layout.
**產出：** Host facts, prerequisite outcomes, blocked/not-applicable reasons.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-UTIT-ADMISSION`, `G-UTIT-RECOVERY`; preflight report.
**完成門檻：** Missing tools, displays, binaries, interactive session, reboot authority, and recovery markers have deterministic dispositions.

- [x] 2.1.1 Capture OS/build, architecture, interactive session, monitor count, Explorer state, tools, and workspace revision.
- [x] 2.1.2 Evaluate tool/file/interactive/multi-display/reboot/external prerequisites without mutation.
- [x] 2.1.3 Test current, missing, conditional-not-applicable, one-display-blocked, and unresolved-recovery cases.

### 2.2 Implement bounded child execution and recovery checks

**目的：** Run admitted cases with complete logs and bounded host impact.
**輸入：** Resolved cases, preflight facts, unique run directory.
**產出：** Serial executor, stdout/stderr, timeout/failure results, recovery evidence.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-UTIT-EXECUTION`, `G-UTIT-RECOVERY`; fixture integration report.
**完成門檻：** Pass/fail/timeout/missing-artifact and Explorer-watchdog paths reach exact auditable terminal states.

- [x] 2.2.1 Implement explicit-argv spawn, log capture, polling deadline, exact-child kill, and serial scheduling.
- [x] 2.2.2 Add expected-artifact validation, recovery-report admission, and continue/fail-fast policy.
- [x] 2.2.3 Add fixture integration tests for pass, nonzero, timeout, blocked, malformed artifact, and recovery rejection.

## 3. Reports and current GUI suite integration

### 3.1 Implement canonical reports and validation

**目的：** Make every result reproducible, hash-bound, and consumable by CI.
**輸入：** Host facts, case executions, artifacts, source/tool identities.
**產出：** JSON, JUnit XML, Markdown, hashes, validator and replay metadata.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-UTIT-REPORT`, `G-TRACE`; report-schema and validator evidence.
**完成門檻：** Deterministic projections validate; duplicate, drifted, missing, malformed, and inconsistent reports fail closed.

- [x] 3.1.1 Add run decision/count derivation, canonical JSON writer, and SHA-256 artifact binding.
- [x] 3.1.2 Add escaped deterministic JUnit XML and Markdown summary projections.
- [x] 3.1.3 Add report validation and tests for Unicode, escaping, hash drift, count/state mismatch, filtering, and incomplete full runs.

### 3.2 Integrate maintained shell-parity cases

**目的：** Replace manual orchestration for currently implemented Windows GUI surfaces.
**輸入：** Runner, release app, maintained headful scripts and OpenSpec gates.
**產出：** Executable smoke/shell-parity catalogs, wrapper documentation, run evidence.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-UTIT-SMOKE`, `G-UTIT-SHELL-PARITY`, `G-UTIT-RECOVERY`; `evidence/runs/*`.
**完成門檻：** Smoke passes; safe current-host GUI cases execute serially and every Explorer suppression records successful recovery.

- [x] 3.2.1 Wire workspace/focused tests, Clippy, release, strict OpenSpec, and dry-run catalog cases.
- [x] 3.2.2 Wire taskbar, Start, desktop marquee, Show desktop, notification, system flyout/IME, resize/lock, and auto-hide scripts.
- [x] 3.2.3 Execute and validate smoke plus safe shell-parity runs; preserve truthful blocked full-suite prerequisites.

## 4. Product integration and release evidence

### 4.1 Harden source boundaries and operator workflow

**目的：** Keep UTIT test authority isolated and easy to invoke repeatedly.
**輸入：** Passing runner and catalog, architecture governance, repository scripts.
**產出：** Boundary checks, README/operator commands, stable wrapper, regression tests.
**依賴：** 3.2.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-UTIT-ADMISSION`, `G-UTIT-RECOVERY`, `G-SHELL-NONINTERFERENCE`; governance report.
**完成門檻：** No production crate depends on UTIT, no arbitrary shell path exists, and documented commands reproduce admitted runs.

- [x] 4.1.1 Update architecture allowlist/governance and prove production dependency direction remains one-way.
- [x] 4.1.2 Add `run_utit.bat` plus concise operator documentation for list, smoke, shell-parity, full, and report validation.
- [x] 4.1.3 Run source-boundary, command-injection, secret-redaction, interruption, and recovery regression gates.

### 4.2 Run full quality, packaging, and traceability gates

**目的：** Admit the developer tool and refreshed product packages without archive mutation.
**輸入：** Committed runner, validated current-host reports, clean implementation revision.
**產出：** Full gates, release hashes, both NSIS packages, 24-leaf evidence index.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-UTIT-SMOKE`, `G-UTIT-SHELL-PARITY`, `G-TRACE`, `G-PACKAGE`; final evidence.
**完成門檻：** 24/24 leaves pass, full suite truthfully reports external blockers, strict/detailed validation passes, and the change remains unarchived.

- [x] 4.2.1 Run fmt, locked/offline workspace tests, Clippy warnings-as-errors, and release build.
- [x] 4.2.2 Commit implementation and build standalone plus combined NSIS installers without launch.
- [x] 4.2.3 Record revisions/binary/package hashes, create 24 unique evidence records, and pass detailed/strict validation.
