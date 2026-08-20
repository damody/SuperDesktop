# Final Review

## Architecture and scope

- Production changes are limited to the Windows taskbar adapter, owned-shell task reconciliation, and existing minimize action callbacks.
- Only an exactly revalidated visible iconic task window is hidden with `ShowWindowAsync(SW_HIDE)`.
- A bounded per-identity cache reintroduces only hidden iconic windows into the owned taskbar model; it does not claim general visibility or persist state.
- Preview mode passes no shelf and never hides or caches host-shell windows.

## Safety and lifecycle

- Retired/reused HWND, PID mismatch, stable-identity mismatch, restored, hidden, tool, cloaked, and transient windows fail before mutation.
- No `SetWindowPlacement`, shelf `SetWindowPos`, style/owner mutation, custom geometry restore, Explorer fallback, production simulated input, or production `unwrap`/`expect` was introduced.
- Restore, maximize, Alt+Tab, application restore, and close retain their existing Windows actions against the same live HWND.
- Hidden cached identities are removed after restore, retirement, or state change; continuous failures are logged once and later episodes retry.

## Verification

- Focused formatting/parser/platform/runtime/catalog/manifest gates passed.
- Full workspace tests and all-target warnings-denied Clippy passed.
- Final GUI runs 8 and 9 used candidate `78B67A0D...`; both proved taskbar and application minimize are iconic, hidden, taskbar-retained, exactly restored, and emit exactly two shelf traces.
- Both final runs proved SuperDesktop/fixture survival, runtime error absence, Explorer recovery, and exact Winlogon Shell restoration.
- Installer `174AE90F...` built without launch. Extracted `superdesktop-app.exe` exactly equals the final GUI candidate.
- Existing unrelated evidence directories were restored unchanged after the source-clean package gate.

## Findings

- P0: 0
- P1: 0
- P2: 0 open
- Superseded evidence remains committed and is explicitly traced through B-001 and A-002.
