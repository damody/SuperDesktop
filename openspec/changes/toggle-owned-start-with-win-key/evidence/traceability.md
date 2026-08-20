# Traceability

| Requirement / scenario | Implementation | Task / gate | Evidence |
|---|---|---|---|
| Left/right standalone Win opens Start | `reduce_standalone_windows_key`, `ToggleStart` | 2.1.1–2.1.3 / HOTKEY-01 | focused reducer tests; gui-run-4/5 |
| Open Start closes on second Win | shared `callbacks.start` dispatch | 2.2.1, 3.1.3 / RUNTIME-01, GUI-01 | exact two toggle traces and `start:closed` in gui-run-4/5 |
| Repeat emits once | candidate state ignores matching repeat keydown | 2.1.2–2.1.3 / HOTKEY-01 | `standalone_windows_key_toggles_once_on_matching_release` |
| Supported chord has no trailing Start | non-Win keydown cancels before chord reducer | 2.1.2–2.1.3 / HOTKEY-01 | Win+E reducer matrix |
| Unsupported chord passes without trailing Start | cancellation returns unconsumed non-Win event | 2.1.2–2.1.3 / HOTKEY-01 | unsupported `0x46` reducer matrix |
| Dual Win is ambiguous | second distinct Win cancels tracked candidate | 2.1.2–2.1.3 / HOTKEY-01 | dual/mismatch reducer test |
| Mismatched release preserves state | unrelated key-up returns unchanged state | 2.1.3 / HOTKEY-01 | dual/mismatch reducer test |
| Shared owned Start lifecycle | runtime clones and invokes `callbacks.start` | 2.2.1–2.2.2 / RUNTIME-01 | composition contract and gui-run-4/5 |
| Missing callback is nonfatal | scoped `report_error` branch | 2.2.1–2.2.2 / RUNTIME-01 | source contract and workspace tests |
| Preview leaves Windows in control | hook construction remains guarded by `shell` | 2.2.2 / RUNTIME-01 | existing shell-scope contract |
| Headful open/close | physical `keybd_event` down/up twice | 3.1.1–3.1.3 / GUI-01 | gui-run-4/5 report, trace, screenshot |
| Failure cleanup restores host | script `finally`, shell snapshot, Explorer watchdog | 3.1.1–3.1.3 / GUI-01 | `shell_restored=true`, `explorer_restored=true` in gui-run-4/5 |
| Exact packaged candidate | NSIS extraction hash equals GUI candidate | 4.1.3, 4.2.2 / SRC-01, REL-01 | `package-gates.json` |

The first GUI run is retained as superseded observation lineage: product traces passed, but the original HWND-only test selected the owned desktop surface. A-002 changed only test observation to UI Automation structural identification. Runs 2/3 proved the behavior on the pre-commit candidate. The commit-derived rebuild changed the binary hash, so A-003 invalidated those runs for release provenance and runs 4/5 re-proved the final packaged candidate.
