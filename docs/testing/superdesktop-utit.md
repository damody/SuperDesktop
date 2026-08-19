# SuperDesktop UTIT

UTIT is the test-only UI Test Integration Tool for the SuperDesktop replacement shell. It inventories deterministic, headful, Explorer-free, hardware, reboot, and external-review gates, then writes JSON, JUnit XML, Markdown, per-case logs, screenshots, and SHA-256 bindings under one run directory.

## Commands

From `D:\SuperExplorer\SuperDesktop`:

```bat
run_utit.bat list
run_utit.bat list --json
run_utit.bat run --suite smoke
run_utit.bat run --suite shell-parity
run_utit.bat run --suite full
run_utit.bat run --suite shell-parity --case gui-start
run_utit.bat run --suite shell-parity --dry-run
run_utit.bat validate-report utit-results\run-<id>\report.json
```

`smoke` does not suppress Explorer. `shell-parity` runs GUI cases serially and admits Explorer-free cases only when their scripts own a recovery watchdog. `full` executes every safe actionable case, then reports unavailable mixed-DPI hardware, reboot/installer mutation, or independent review as `blocked`; it never turns those gaps into a pass.

JSON is authoritative. `junit.xml` and `summary.md` are deterministic projections. A filtered run is always `partial`, even when all selected cases pass. `validate-report` recalculates counts/decision and verifies every recorded artifact hash.

The runner accepts only compiled cases. It does not execute arbitrary command text, `cmd /c`, PowerShell `-Command`, network-discovered tests, or paths outside the canonical workspace/script roots.
