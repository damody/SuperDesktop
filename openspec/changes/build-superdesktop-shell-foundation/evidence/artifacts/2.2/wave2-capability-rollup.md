# Wave 2 Windows/GPUI capability roll-up

- Child change: `validate-superdesktop-windows-platform` (kept active at the user's request)
- OpenSpec schema: `spec-driven`
- Strict validation: passed
- Detailed task state: 42/42 complete, `all_done`
- Evidence validator: 45 records / 42 coverage mappings, passed
- Required capability matrix: 37 rows, no failed/blocked/stale/N/A rows
- `G-ARCH`: passed
- `G-SHELL-TAKEOVER-CAPABILITY`: passed
- `G-DPI-MONITOR`: passed
- `G-TASKBAR`: passed
- `G-GUARDIAN-RECOVERY-CAPABILITY`: passed
- `G-SAFETY`: passed
- Capability matrix SHA-256: `BCE7990FE557C167F2F3B0DE104C6E2BBC4E5876F5FF085712BC8B39613306D6`
- Platform-common API/ABI manifest SHA-256: `A7CF3DDDF723DA157675A246B6B226E0B17BC1F7060C1A34A1AB338C02733A2D`
- Signed GO disposition SHA-256: `76393690CB75174E8F4B7EDC8354A4CE3220C4FFAEFD961B45EFDDFFFADD123B`

The capability predecessor is sufficient for `build-superdesktop-shell-core`,
whose explicit dependency is the signed capability GO disposition. Program
task 2.2.8 remains pending because the user requested that all active changes
be implemented before any change is archived.
