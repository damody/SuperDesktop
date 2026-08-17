# Verify SuperDesktop Shell Completion

## 1. Evidence contract and aggregation

### 1.1 Add fail-closed roll-up model

**目的：** Represent every child source, gate, limitation, and derived release decision without implicit defaults.
**輸入：** Eight completion child changes and the M0 blocking-gate vocabulary.
**產出：** Versioned Rust DTOs, deterministic aggregation, and failure diagnostics.
**依賴：** All eight completion implementation children.
**Owner／Wave：** Primary integrator / Wave 9A.
**Gate／Evidence：** `G-TRACE`, `G-SAFETY`; unit-test output.
**完成門檻：** Missing, duplicate, unexpected, pending, and failed gates all block release in tests.

- [x] 1.1.1 Add source, gate, limitation, command, and derived-disposition DTOs.
- [x] 1.1.2 Add exact required-child admission and fail-closed gate aggregation.
- [x] 1.1.3 Add deterministic serialization and negative matrix tests.

### 1.2 Add evidence schema and collector

**目的：** Validate child artifacts and emit a reproducible completion roll-up.
**輸入：** Child `evidence/verification.json` files and local command results.
**產出：** JSON schema, PowerShell collector, and current local roll-up.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 9A.
**Gate／Evidence：** `G-TRACE`, `G-ARCH`; schema and collector output.
**完成門檻：** The collector accepts exactly the expected children and rejects corrupted or incomplete input.

- [x] 1.2.1 Add a versioned roll-up JSON schema and exact child manifest.
- [x] 1.2.2 Add a non-mutating collector with source hashes and stable gate output.
- [x] 1.2.3 Emit and validate the current local roll-up artifact.

## 2. Cross-domain local verification

### 2.1 Execute functional, accessibility, DPI, performance, and safety suites

**目的：** Prove completion features integrate without regressing M0 contracts or published bounds.
**輸入：** Workspace tests, virtual visual fixtures, provider/process tests, installer dry-run.
**產出：** Local command log and passed local gate dispositions.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / Wave 9B.
**Gate／Evidence：** `G-DESKTOP`, `G-TASKBAR`, `G-A11Y-I18N`, `G-DPI-MONITOR-VIRTUAL`, `G-PERF`, `G-SAFETY`.
**完成門檻：** Fmt, offline check, clippy, all workspace tests, dry-run non-mutation, strict OpenSpec, and task validation pass.

- [x] 2.1.1 Run all workspace format, offline build, clippy, test, and process fixtures.
- [x] 2.1.2 Verify provider bounds, stale/cancel semantics, accessibility/i18n, virtual DPI, and performance/resource budgets.
- [x] 2.1.3 Run live installer read-only preflight and prove no registry/rollback mutation.
- [x] 2.1.4 Validate every completion child and this verification change with strict OpenSpec.

## 3. External gates and release disposition

### 3.1 Preserve physical and independent gates

**目的：** Complete release evidence on the required host/hardware and through independent review.
**輸入：** Exact Windows 11 ExplorerPatcher reference profile, physical mixed-DPI displays, reboot collector, independent reviewer.
**產出：** Signed/attributable external artifacts and final derived disposition.
**依賴：** 2.1 and external resources.
**Owner／Wave：** Physical operator plus independent reviewer / Wave 9C.
**Gate／Evidence：** `G-SHELL-TAKEOVER`, `G-GUARDIAN-RECOVERY`, `G-INSTALL-ROLLBACK`, `G-DPI-MONITOR-PHYSICAL`, `G-REVIEW`.
**完成門檻：** All physical runs and independent review pass; only then is `release_allowed=true`.

- [ ] 3.1.1 Capture exact Windows 11 ExplorerPatcher profile takeover, forced-crash recovery, installer enable/reboot/rollback, and normal-exit evidence.
- [ ] 3.1.2 Capture keyboard, pointer, visual, hot-plug, and work-area behavior on a physical mixed-DPI topology.
- [ ] 3.1.3 Obtain independent architecture/security/accessibility review with attributable disposition.
- [ ] 3.1.4 Recompute the roll-up and publish the final release disposition.
