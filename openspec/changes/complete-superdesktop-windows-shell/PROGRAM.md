# SuperDesktop Windows Shell Completion Program

## Ordered changes and ownership

| Order | Change | Owner | Depends on | Local state |
|---:|---|---|---|---|
| 1 | `extend-superdesktop-shell-contracts` | Protocol/platform | M0 foundation | 14/14 complete |
| 2 | `add-superdesktop-desktop-file-operations` | Desktop/platform | 1 | 13/13 complete |
| 3 | `add-superdesktop-shell-context-menu-host` | Provider/desktop | 1 | 11/11 complete |
| 4 | `add-superdesktop-start-search` | Provider/taskbar | 1 | 12/12 complete |
| 5 | `add-superdesktop-taskbar-advanced-interactions` | Taskbar/platform | 1, 4 | 11/11 complete |
| 6 | `add-superdesktop-notification-area-host` | Host/taskbar | 1, 5 | 10/10 complete |
| 7 | `add-superdesktop-virtual-desktops` | Platform/taskbar | 1, 5 | 10/10 complete |
| 8 | `add-superdesktop-shell-installer` | Installer/recovery | M0 takeover/recovery, 1–7 binaries | 11/11 complete |
| 9 | `verify-superdesktop-shell-completion` | Verification | 1–8 | 10/14 local; 4 external pending |

## Capability ledger

Implemented with documented/product-owned contracts:

- Versioned provider protocol/host with bounded frames, deadlines, cancellation, generation tokens, and health.
- Desktop rename, recycle-first delete, explicit permanent delete, cancellable copy/move, collision policy, layout, sort, and reconciliation.
- Isolated context-menu model/invocation with sanitized bounded DTOs.
- Owned Start UI with app/settings/file search, IME, ranking, cancellation, and accessibility semantics.
- Taskbar flyouts, Jump Lists, overlays/progress, settings persistence, bounded preview behavior, and truthful fallbacks.
- Owned versioned notification-area host, registry, overflow model, event routing, and unavailable-state behavior.
- Documented `IVirtualDesktopManager` window query/move behavior.
- Explicit dry-run shell installer with fingerprint authority, immutable exact rollback record, compare-before-write, Unicode readback, verification, and rollback.

Limitations and non-claims:

- Legacy Explorer notification-area protocol compatibility is not claimed; the implementation uses the owned provider protocol.
- Virtual-desktop enumerate/switch/create/remove/rename is unavailable because the documented adapter exposes query/move only.
- Release approval is not claimed until Windows 10 build 19045, physical mixed-DPI, reboot rollback, and independent review gates pass.

## Invariants no child may weaken

- Ordinary launch and local verification never mutate the login shell.
- Shell takeover and installer mutation require separate explicit authority; installer authority includes an exact current-plan fingerprint.
- Guardian and owner identity/session/binary bindings fail closed, and Explorer recovery remains exactly once and bounded.
- Provider and native callback boundaries own data, catch/fence terminal states, reject stale generations, and never unwind across FFI.
- Queues, caches, result sets, menu depth, frames, searches, and retries remain bounded with explicit overflow/unavailable states.
- Filesystem mutations admit only canonical allowed roots; default delete uses the Recycle Bin and permanent delete is explicit.
- Unsupported Windows behavior stays unavailable/not-claimed instead of being simulated as native parity.
- External evidence cannot be inferred from unit tests, and changes stay unarchived until explicitly requested.

## Current disposition

Production implementation is locally complete. Local workspace and completion verification gates pass. Release remains blocked only by the external gates recorded in `evidence/program-rollup.json`.
