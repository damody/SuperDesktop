## Context

SuperDesktop currently relies on workspace tests and more than twenty Windows capture/verification scripts. Individual scripts prove important behavior, but there is no single catalog, consistent timeout/recovery policy, or canonical cross-case report. The approved source design is `docs/superpowers/specs/2026-08-19-superdesktop-utit-design.md`.

## Goals / Non-Goals

**Goals:**

- Add a test-only Rust executable that inventories and runs typed shell-parity cases.
- Make suite selection, prerequisites, timeout, evidence, recovery, and final disposition deterministic.
- Consolidate current GUI/headful coverage without putting test authority in product binaries.
- Emit validated JSON, JUnit, Markdown, logs, hashes, and replay argv.
- Run safely with Explorer absent only through bounded watchdog-owning cases.

**Non-Goals:**

- Claiming unavailable physical mixed-DPI, reboot, installer-mutation, or independent-review evidence passed.
- Rewriting every PowerShell/C# UIA adapter in Rust in this change.
- Shipping UTIT in the end-user installer or adding a production control server.
- Allowing arbitrary user shell commands, unbounded case plugins, or network test discovery.

## Decisions

### Separate test-only crate

`crates/superdesktop-utit` owns catalog, preflight, executor, and reporting modules. It depends only on serde/JSON and standard library process/filesystem APIs. Production crates do not depend on UTIT. Embedding test control in `superdesktop-app` was rejected because it would enlarge the release authority boundary.

### Compiled typed catalog

Each `TestCase` has stable ID, title, tier, tags, fixed program kind, argv, timeout, prerequisites, expected artifacts, and recovery contract. Paths are resolved relative to a canonical workspace and admitted against explicit roots. The executor never assembles `cmd /c` or PowerShell `-Command`. Existing repository scripts are invoked with `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <canonical-script> <fixed argv>`.

### Truthful suite closure

`smoke` includes deterministic non-mutating cases; `shell-parity` includes smoke plus bounded headful/Explorer-free cases; `full` adds packaging and external/hardware cases. Filters mark a run partial. Missing optional prerequisites produce evidence-backed `not-applicable`; missing mandatory physical/reboot prerequisites produce `blocked` and keep the run incomplete.

### Bounded serial executor

The first version runs cases serially for reproducibility and because GUI focus/AppBar/Explorer suppression cannot safely overlap. It captures stdout/stderr to unique files, polls a child until deadline, kills only that exact child on timeout, then records failure. Explorer-free scripts must declare watchdog ownership and produce a recovery field; otherwise catalog validation rejects them.

### Canonical report and projections

JSON is the authoritative run format. JUnit XML and Markdown are deterministic projections. SHA-256 uses the existing Windows `certutil` only for files when available, with a pure deterministic fallback identifier forbidden; a missing hash tool fails evidence admission. Report validation checks schema version, stable unique IDs, terminal states, counts, required hashes/artifacts, and derived decision.

### Correction policy and gates

- **A — task refinement:** case order, helper extraction, log filenames, or fixture argv may change without altering contracts or gates.
- **B — design/spec correction:** unsafe command admission, invalid suite closure, or unreasonable report semantics require design/spec/tasks updates and stale evidence replacement.
- **C — material change:** production control authority, arbitrary commands, network discovery, weaker recovery, new external writes, or converting blocked evidence to pass requires user approval.

Blocking gates are `G-UTIT-CATALOG`, `G-UTIT-ADMISSION`, `G-UTIT-EXECUTION`, `G-UTIT-REPORT`, `G-UTIT-RECOVERY`, `G-UTIT-SMOKE`, `G-UTIT-SHELL-PARITY`, `G-TRACE`, and `G-PACKAGE`.

## Risks / Trade-offs

- **[PowerShell adapters remain heterogeneous]** → Freeze their argv/artifact contracts in the catalog and validate every terminal report.
- **[Timeout leaves grandchildren]** → Headful scripts retain their own watchdog/cleanup; UTIT kills its exact child and verifies recovery evidence before continuing.
- **[A filter creates a misleading green run]** → Mark all filtered runs partial and never derive complete/full passed.
- **[Current host lacks physical/reboot prerequisites]** → Record blocked with host facts and keep full suite incomplete.
- **[Logs expose local paths]** → Reports use workspace-relative paths where possible and never copy environment values or credentials.

## Migration Plan

1. Land crate, catalog, CLI, pure validation tests, and fixture integration tests.
2. Admit current automated/headful scripts and run smoke.
3. Run safe shell-parity with Explorer watchdog recovery.
4. Validate reports, run workspace/release gates, and rebuild packages.

Rollback removes the test-only crate and wrapper documentation. No product data, registry, protocol, or installer migration exists.

## Open Questions

None.
