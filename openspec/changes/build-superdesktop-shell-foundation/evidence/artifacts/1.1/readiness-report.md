# Wave 0 readiness report

- Change: `build-superdesktop-shell-foundation`
- L2: `1.1`
- Recorded at: `2026-08-14T02:05:26+08:00`
- Primary integrator: Codex `/root`
- Decision: ready to establish the planning baseline and enter Wave 1.

## Child apply readiness

All eight child changes contain `proposal.md`, `design.md`, `tasks.md`, and at least one delta spec. `openspec validate --strict --all` passed 9/9 changes on the recorded machine.

| Child change | Specs | Mandatory leaves | Wave |
| --- | ---: | ---: | --- |
| `bootstrap-superdesktop-workspace` | 2 | 26 | 1 |
| `validate-superdesktop-windows-platform` | 1 | 37 | 2 |
| `build-superdesktop-shell-core` | 2 | 35 | 3 |
| `build-superdesktop-gpui-desktop` | 1 | 51 | 4A |
| `build-superdesktop-gpui-taskbar` | 1 | 47 | 4B |
| `integrate-superexplorer-process-bridge` | 1 | 30 | 4C |
| `add-superdesktop-shell-takeover-recovery` | 1 | 73 | 5 |
| `verify-superdesktop-m0` | 1 | 93 | 6 |

## Dependency and ownership disposition

The executable order is Wave 1 → 2 → 3 → parallel 4A/4B/4C → 5 → 6. No cycle or hidden production predecessor remains. Wave 4 may only consume the frozen Wave 2 platform-common ABI and Wave 3 public contract hashes. Wave 5 consumes all three accepted Wave 4 handoffs. Wave 6 consumes the Wave 5 provisional dispositions and produces the Windows 10 final dispositions.

`EXECUTION.md` assigns a unique production writer to root manifests/build scripts, platform common, shell core/settings, desktop, taskbar, bridge, lifecycle/guardian, and verification paths. Child agents write only their change-local evidence and handoff; the Primary integrator is the sole writer for program roll-up artifacts and archive synchronization.

## Program-to-child coverage

| Program capability | Producing child change(s) | Release verifier |
| --- | --- | --- |
| `windows-shell-lifecycle` | platform capability + lifecycle/guardian | `verify-superdesktop-m0` |
| `gpui-desktop-surface` | desktop | `verify-superdesktop-m0` |
| `gpui-taskbar-window-management` | taskbar | `verify-superdesktop-m0` |
| `superexplorer-process-bridge` | bridge, consumed by desktop/taskbar/lifecycle | `verify-superdesktop-m0` |
| `shell-settings-and-reconciliation` | shell core/settings | `verify-superdesktop-m0` |
| `shell-foundation-verification` | bootstrap evidence governance + platform capability + verification | `verify-superdesktop-m0` |

The child specs and tasks preserve the parent requirement/scenario/gate coverage. Blocking final takeover and guardian gates are produced only by the Windows 10 Wave 6 confirmation; Wave 5 results remain provisional.

## External prerequisites

The current host is Windows 11 Pro build 26200 with one physical 3840×2160 display. It is the approved ExplorerPatcher reference environment, not evidence for Windows 10 22H2 or a physical mixed-DPI dual-monitor confirmation. Those two Wave 6 confirmation branches are predeclared `blocked` until an external environment becomes available. They are mandatory and cannot use N/A.

## B-level corrective lineage

1. The former monolithic implementation plan was replaced by this program plus eight child changes; each child owns atomic L2/L3 work and change-local evidence.
2. The parent proposal incorrectly claimed desktop rename was M0. It now matches the approved design and desktop spec: rename is deferred and represented only by a typed unavailable boundary.
3. `EXECUTION.md` and program tasks retained the obsolete pre-apply stop condition. They now record the user's explicit apply authorization while preserving the preview-only lifecycle safety gate.

No previously passed implementation evidence exists, so these planning corrections do not stale any product evidence.

### Former monolith L2 replacement map

| Former L2 | Mandatory replacement work package(s) |
| --- | --- |
| 1.1 evidence governance | bootstrap 2.3 |
| 1.2 workspace | bootstrap 1.1, 1.2, 2.1 |
| 1.3 architecture boundaries | bootstrap 1.1, 2.2 |
| 1.4 capability spike | platform 1.1–3.4 |
| 2.1 shell state/reducer | core 1.1, 1.2 |
| 2.2 generation/queue/reconciliation | core 1.2, 2.1, 2.2 |
| 2.3 Win32/COM platform skeleton | platform 1.2–3.3 |
| 2.4 settings store | core 3.1, 3.2 |
| 3.1–3.5 desktop | desktop 1.1–4.1 |
| 4.1–4.4 taskbar | taskbar 1.1–4.1 |
| 5.1–5.2 SuperExplorer bridge/repair | bridge 1.1–3.1 plus desktop/taskbar fixed-entry consumers |
| 5.3–5.4 guardian/takeover | lifecycle 1.1–4.2 |
| 6.1 composition | lifecycle 1.3, 2.1, 2.2 plus Wave 4 handoffs |
| 6.2–6.4 accessibility/visual/performance | verifier 2.1–5.1 |
| 7.1–7.3 final gates/review | verifier 1.1, 1.2, 5.2, 5.3 plus program roll-up 4.2 |

Every replacement listed above is mandatory. The old 236-leaf monolith is a planning predecessor, not a completed evidence source; no old leaf is marked `superseded` until the corresponding replacement has valid passed evidence.
