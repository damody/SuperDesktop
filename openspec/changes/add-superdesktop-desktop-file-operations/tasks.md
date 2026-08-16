# Add SuperDesktop Desktop File Operations

## 1. Operation contracts and layout behavior

### 1.1 Add typed operation planning

**目的：** Replace unavailable placeholders with validated, correlated desktop operation contracts.
**輸入：** Desktop item identities, activation tokens, provider protocol limits, approved design.
**產出：** Operation requests, policies, progress, per-item outcomes, cancellation and terminal state.
**依賴：** `extend-superdesktop-shell-contracts` 14/14.
**Owner／Wave：** Primary integrator / Wave 2.
**Gate／Evidence：** `G-DESKTOP-FILE-OPS`, `G-SAFETY`; unit tests.
**完成門檻：** Invalid inputs and implicit overwrites fail before mutation; first terminal wins.

- [x] 1.1.1 Add operation, policy, progress, cancellation, and terminal DTOs to `desktop-ui`.
- [x] 1.1.2 Add validation for names, roots, collisions, permanent deletion, and transfer intent.
- [x] 1.1.3 Add deterministic lifecycle tests for success, cancellation, partial failure, and duplicate terminal events.

### 1.2 Extend arrangement and persistence

**目的：** Provide Windows-like sorting, grid alignment, and stable position restore.
**輸入：** Existing `DesktopLayout`, namespace metadata, monitor logical geometry.
**產出：** Sort descriptors, ordered identities, alignment mode, serializable position snapshots.
**依賴：** 1.1 contracts.
**Owner／Wave：** Primary integrator / Wave 2.
**Gate／Evidence：** `G-DESKTOP-FILE-OPS`, `G-DPI-MONITOR`; deterministic layout tests.
**完成門檻：** All sort modes are stable and persisted positions remain visible after topology changes.

- [x] 1.2.1 Add name/kind/size/modified ordering with identity tie-breaks.
- [x] 1.2.2 Add align-to-grid and explicit reposition semantics.
- [x] 1.2.3 Add snapshot restore tests across DPI/monitor remap and missing items.

## 2. Windows effects and reconciliation

### 2.1 Implement filesystem and Recycle Bin adapters

**目的：** Execute admitted mutations through a narrow Windows/platform boundary.
**輸入：** Validated plans from 1.1 and canonical desktop paths.
**產出：** Rename, recycle, permanent-delete, chunked copy, and move effect functions.
**依賴：** 1.1; existing `platform-win` desktop adapter.
**Owner／Wave：** Primary integrator / Wave 2.
**Gate／Evidence：** `G-SAFETY`; temp-fixture tests and explicit destructive-policy tests.
**完成門檻：** Effects reject unsafe targets, never silently overwrite, and preserve source on cancelled copy.

- [x] 2.1.1 Add canonical root admission and filename validation helpers.
- [x] 2.1.2 Add rename, recycle, and explicit permanent-delete effects.
- [x] 2.1.3 Add cancellable chunked file copy and admitted move with collision policies.
- [x] 2.1.4 Add fixture tests for success, collision, cancellation, and rollback cleanup.

### 2.2 Wire reconciliation and verify

**目的：** Make terminal effects converge back to authoritative desktop namespace state.
**輸入：** Operation terminals, watcher queue, namespace enumeration, selection/layout models.
**產出：** Reconciliation controller and complete verification evidence.
**依賴：** 1.2 and 2.1.
**Owner／Wave：** Primary integrator / Wave 2 exit.
**Gate／Evidence：** `G-DESKTOP-FILE-OPS`, `G-TRACE`; Rust and OpenSpec validation logs.
**完成門檻：** Refresh is single-flight, stale deltas are suppressed, selection restores by identity, and all checks pass.

- [x] 2.2.1 Add operation-to-refresh reconciliation and stable-selection restore.
- [x] 2.2.2 Replace desktop deferred operation placeholders with executable operation effects.
- [x] 2.2.3 Run fmt, offline check, clippy, unit/integration tests, strict OpenSpec, and detailed-task validation.
