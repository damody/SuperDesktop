## Why

SuperDesktop has substantial deterministic and Windows headful coverage, but its gates are spread across independent scripts with duplicated recovery, hashing, and result semantics. A single owned UTIT executable is needed to run the replacement-shell matrix repeatably, preserve host recovery, and distinguish real passes from unavailable hardware or reboot evidence.

## What Changes

- Add a standalone Rust `superdesktop-utit` binary with typed `list`, `run`, and `validate-report` commands.
- Add compiled `smoke`, `shell-parity`, and `full` catalogs covering automated, GUI, Explorer-free, installer, and external-prerequisite gates.
- Admit only canonical fixed executable/script argv; reject arbitrary shell strings and paths outside approved roots.
- Add bounded execution, timeout, log capture, artifact hashing, recovery contracts, host preflight, and truthful blocked/not-applicable states.
- Produce canonical JSON plus deterministic JUnit XML and Markdown summaries with replay metadata.
- Run current-host smoke and safe shell-parity suites, full workspace gates, release, traceability, and both NSIS packages without archiving the change.

## Capabilities

### New Capabilities

- `superdesktop-utit-runner`: Defines test catalog, execution safety, suite completeness, report/evidence contracts, CLI behavior, and Explorer-free recovery.

### Modified Capabilities

None.

## Impact

Adds one test-only workspace crate, a suite manifest/report schema, fixture integration tests, focused wrapper documentation, OpenSpec evidence, and packaged developer tooling. It does not modify production shell authority, registry state, product protocols, or Explorer-free implementation paths.
