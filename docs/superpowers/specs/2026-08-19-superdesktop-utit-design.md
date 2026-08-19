# SuperDesktop UTIT Design

## Intent

Create a single executable UI Test Integration Tool (`superdesktop-utit`) that inventories, runs, verifies, and reports SuperDesktop's automated Windows shell-parity tests. The tool replaces ad-hoc manual orchestration without moving test-only privileges into SuperDesktop production binaries or pretending hardware/reboot evidence passed when the host cannot provide it.

## Chosen architecture

`superdesktop-utit` is a separate Rust workspace binary with four isolated layers:

1. **Catalog** — compiled, typed test cases grouped into `smoke`, `shell-parity`, and `full` suites. Every case declares risk, timeout, prerequisites, artifact expectations, and an argv vector. No case accepts an arbitrary command string.
2. **Preflight** — records Windows build, interactive session, monitor/DPI topology, Explorer state, required binaries/tools, and workspace identity. Missing prerequisites yield `blocked` or `not-applicable`, never `passed`.
3. **Executor** — launches fixed executables and in-repository scripts with explicit arguments, bounded time, isolated output directories, captured stdout/stderr, and process cleanup. Explorer-free cases may run only when their script owns a watchdog and the user selected a shell-parity-capable suite.
4. **Reporter** — writes one canonical JSON run, JUnit XML, Markdown summary, per-case logs, artifact hashes, and replay argv. A validator rejects duplicate case IDs, missing artifacts, stale hashes, invalid terminal states, or a run that claims completion while mandatory cases failed or remain blocked.

The first catalog absorbs the current trustworthy gates rather than rewriting UI Automation immediately: Cargo tests/Clippy/release, strict OpenSpec validation, and the maintained headful scripts for taskbar, Show desktop, Start, desktop marquee, notification center, system status/flyouts, resize/lock, and auto-hide. Script adapters remain narrow Windows mechanisms; scheduling, classification, evidence, timeout, and final disposition move into UTIT.

## Alternatives considered

- **Continue with independent PowerShell scripts:** lowest initial cost, but duplicated watchdog, logging, hashing, and pass/fail logic has already drifted. Rejected as the primary architecture.
- **Embed a test server in `superdesktop-app`:** direct control would be convenient, but it increases production attack surface and risks shipping test authority. Rejected.
- **Rewrite every Windows UIA adapter in Rust immediately:** strongest long-term type safety, but the workspace denies unsafe code and the existing tested UIA/Win32 adapters are PowerShell/C#; a full rewrite would delay usable coverage. Deferred behind the typed runner boundary.

## Suite semantics

- `smoke`: non-mutating deterministic tests, source guards, schema validation, and optional preview captures.
- `shell-parity`: smoke plus headful and Explorer-absent cases whose scripts have bounded watchdog recovery.
- `full`: shell-parity plus packaging, physical mixed-DPI, reboot/exact-rollback, and external review cases. Unsupported physical or reboot prerequisites terminate as `blocked` with evidence and make the overall run incomplete.

Filtering is explicit (`--case`, `--tag`) and cannot silently turn a filtered run into a complete full-suite result. `--dry-run` resolves and validates every argv and prerequisite without launching cases.

## Safety and Explorer independence

The runner canonicalizes the workspace, executable, script, and output roots. Scripts must remain under `scripts/`; binaries must remain under the workspace or known Windows system directories. Arguments are passed directly through `std::process::Command`; no `cmd /c` or PowerShell `-Command` string is assembled by UTIT. Each Explorer-free catalog item declares its watchdog contract and recovery artifact. A run refuses shell-parity cases when a prior recovery marker is unresolved.

The product under test never calls Explorer for Start, taskbar, desktop, tray, IME, Show desktop, or system flyouts. Explorer is used only by the test recovery watchdog after a bounded test finishes or fails, preserving host recoverability.

## Report contract

Each case records stable ID, suite/tags, start/end timestamps, terminal state, exit code, timeout, prerequisites, argv, stdout/stderr paths and hashes, artifact list and hashes, and recovery disposition. The run records tool/source/binary hashes, host facts, selected and mandatory counts, and a derived overall decision. JSON is authoritative; JUnit and Markdown are deterministic projections.

Terminal states are `passed`, `failed`, `blocked`, `skipped`, and `not-applicable`. Only `passed` and evidence-backed `not-applicable` satisfy a mandatory case. A timeout is always `failed`; a missing physical prerequisite is `blocked`.

## Error handling and recovery

- Spawn failure, non-zero exit, timeout, malformed report, missing artifact, or hash drift fails that case.
- Headful cases execute serially to avoid UI focus and Explorer suppression races; deterministic cases may remain sequential in the first version for reproducibility.
- On interruption, the runner writes an incomplete report before exiting and invokes only the declared recovery program for the active case.
- Previous result directories are never overwritten unless `--replace-run` names the exact run directory.

## Verification

- Unit tests cover catalog uniqueness, suite closure, filter completeness, prerequisite disposition, command admission, timeout, output capture, artifact hashing, report derivation, JUnit escaping, Markdown stability, and report validation.
- Integration tests execute passing, failing, timing-out, blocked, and malformed fixture programs without Explorer mutation.
- A smoke run validates the current workspace.
- A shell-parity run executes the safe current-host GUI matrix and proves Explorer recovery.
- Full workspace tests, Clippy warnings-as-errors, release build, strict/detailed OpenSpec validation, and standalone/combined NSIS packaging gate completion.

## Scope boundary

This change delivers the UTIT framework and a maintained catalog covering currently implemented shell surfaces. It does not convert unavailable physical mixed-DPI, reboot, installer mutation, or independent review evidence into a pass, and it does not claim that every Windows Explorer feature already exists. Future GUI parity changes add cases to this catalog as part of their own completion gates.
