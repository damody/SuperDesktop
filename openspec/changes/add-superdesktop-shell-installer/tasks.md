# Add SuperDesktop Shell Installer

## 1. Transaction planning and safety

### 1.1 Add installer plans and preflight

**目的：** Make every proposed shell mutation explicit, fingerprinted, and fail closed.
**輸入：** App/guardian paths, current per-user Shell state, session/policy/recovery admission.
**產出：** Install/enable/disable/repair/uninstall plans, preconditions, affected targets, audit DTOs.
**依賴：** M0 takeover/guardian recovery and completion binaries.
**Owner／Wave：** Primary integrator / Wave 8.
**Gate／Evidence：** `G-INSTALL-ROLLBACK`, `G-SAFETY`; plan/preflight tests.
**完成門檻：** Dry-run is default; unsafe paths, missing guardian, unsupported session, and drift reject mutation.

- [x] 1.1.1 Add command, authority, observation, plan, fingerprint, and terminal audit types.
- [x] 1.1.2 Add canonical file, session, policy, guardian, and current-state preflight.
- [x] 1.1.3 Add deterministic plan fingerprint and dry-run rendering tests.

### 1.2 Add registry and rollback transaction

**目的：** Apply or restore the per-user Shell value with exact pre/post verification.
**輸入：** Authorized plan, compare-before-write observation, fsynced rollback record.
**產出：** Transaction engine, registry abstraction/adapter, rollback and verification records.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 8.
**Gate／Evidence：** `G-INSTALL-ROLLBACK`, `G-SAFETY`; fault-injection tests.
**完成門檻：** Every failure point preserves or restores a complete state and external drift is never overwritten.

- [x] 1.2.1 Add registry abstraction and per-user Winlogon Shell adapter.
- [x] 1.2.2 Add rollback-record write-before-mutate, compare, write/delete, read-back, and rollback engine.
- [x] 1.2.3 Add failure injection for record/write/readback/rollback and absent-vs-present state.

## 2. CLI workflow and verification

### 2.1 Add install lifecycle commands

**目的：** Expose safe dry-run/install/enable/disable/repair/uninstall workflows.
**輸入：** Transaction engine and explicit CLI authority flags.
**產出：** `shell-installer` binary, machine-readable output, stable exit codes.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / Wave 8.
**Gate／Evidence：** `G-INSTALL-ROLLBACK`, `G-TRACE`; CLI integration tests.
**完成門檻：** Commands are non-interactive, default dry-run, and uninstall restores before metadata removal.

- [x] 2.1.1 Add CLI parsing, dry-run default, explicit authority, and plan-fingerprint confirmation.
- [x] 2.1.2 Add install/enable/disable/repair/uninstall orchestration and exit codes.
- [x] 2.1.3 Add process tests proving no mutation without full authority.

### 2.2 Verify rollback contract

**目的：** Prove transaction safety locally and prepare physical reboot evidence collection.
**輸入：** Memory registry/filesystem faults and read-only live preflight.
**產出：** Verification evidence, physical-run collector, completed change.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / Wave 8 exit.
**Gate／Evidence：** `G-INSTALL-ROLLBACK`, `G-TRACE`; cargo/OpenSpec and dry-run logs.
**完成門檻：** Fault matrix, read-only dry-run, strict validation, and task validation pass; physical reboot stays an external verification gate.

- [x] 2.2.1 Add fault-matrix tests and fail-closed physical enable/rollback evidence collector.
- [x] 2.2.2 Run fmt, offline check, clippy, tests, strict OpenSpec, and task validation.
