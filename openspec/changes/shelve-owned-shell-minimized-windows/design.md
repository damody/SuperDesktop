## Context

The owned taskbar enumerates visible, unowned top-level windows and uses `IsIconic` for minimized state. Its minimize actions currently call only `ShowWindow(SW_MINIMIZE)`. With Explorer absent, Windows still creates the documented minimized representation but no inbox taskbar owns its destination, so the legacy iconic title tile can remain visible at the lower-left work area.

Microsoft defines `WINDOWPLACEMENT.ptMinPosition` as the minimized position and keeps the restored rectangle independently in `rcNormalPosition`. Live Windows 11 build 26200 evidence from this change proved `SetWindowPlacement` clamps an off-screen `ptMinPosition=(-32000,-32000)` to the workspace edge `(-2,-2)`. A second physical attempt proved cross-process `SetWindowPos` corrupts a DPI-unaware application's normal rectangle by converting `200,200,680,440` into physical pixels `350,350,1190,770`. The safe route is therefore to hide only the already-iconic representation asynchronously and retain its shell task model independently.

## Goals / Non-Goals

**Goals:**

- Remove every eligible minimized application's desktop tile in owned-shell mode while retaining the taskbar button and true minimized state.
- Preserve normal and maximized placement across taskbar, Alt+Tab, system-menu, and application-owned restore paths.
- Cover both SuperDesktop-originated and application-originated minimization with bounded recovery.
- Reject stale/reused HWNDs and report failures once per continuous identity/state episode.

**Non-Goals:**

- Hiding applications from the taskbar or Alt+Tab.
- Changing application styles, ownership, visibility, normal bounds, or maximize semantics.
- Mutating minimized windows in preview mode with Explorer present.
- Replacing Windows minimize animations or adding a custom window manager.

## Decisions

### 1. Use a dedicated minimized-position adapter

`platform-win::taskbar` requires a currently visible and iconic eligible top-level window, then calls `ShowWindowAsync(SW_HIDE)`. This changes only `WS_VISIBLE`; it keeps `WS_MINIMIZE` and all normal/max placement owned by Windows and the application. A later observation must see the same live identity as hidden and iconic before the episode is considered stable.

Plain hiding without shell state was initially rejected because the truthful task filter would remove the taskbar entry. The final design pairs asynchronous hide with an exact-identity bounded cache: hidden iconic windows are reintroduced only into the owned taskbar model, while normal enumeration and Alt+Tab keep the live HWND. `SetWindowPlacement` and `SetWindowPos` were both rejected by physical clamping/DPI evidence. Changing `WS_EX_TOOLWINDOW` remains rejected because it mutates third-party taskbar eligibility.

### 2. Revalidate an exact live identity before mutation

The adapter accepts HWND, PID, and stable `win:<pid>:<HWND>` identity. It re-snapshots that HWND and requires equality plus `visible && minimized && !tool_window && !cloaked && !owned_transient`. A retired, reused, hidden, restored, transient, or excluded identity fails before `ShowWindowAsync`.

This preserves the existing stale-HWND safety boundary and prevents an enumeration-to-mutation race from targeting a new window.

### 3. Reconcile from the existing task snapshot

The runtime owns one `MinimizedWindowShelf` per shell session. The same 50 ms refresh that builds taskbar models passes its snapshot into the shelf before grouping. The reconciler:

- prunes identities no longer eligible and minimized;
- retains a copied task snapshot only while the exact live identity remains hidden and iconic;
- merges that cached identity into taskbar grouping without claiming the HWND is generally visible;
- retries asynchronous hiding if an iconic identity remains visible;
- applies the validated adapter to newly minimized identities;
- caches one failure result per continuous identity/state episode so console output does not flood;
- clears failure/success cache entries after restore, destruction, hiding, or identity replacement so a later minimize retries.

Preview mode bypasses the reconciler completely.

### 4. Minimize commands get an immediate fast path

`apply_window_action_to_owned_identity` performs the existing identity check. For `Minimize`, it calls `ShowWindow(SW_MINIMIZE)` and then the same placement adapter. If the target has not reached iconic state synchronously, the fast path returns a typed pending result and the next snapshot completes reconciliation; it never hides or moves a normal window.

Other actions remain unchanged. Restoring needs no inverse placement mutation because `rcNormalPosition` was never changed.

### 5. Verification is identity- and geometry-based

Pure tests cover eligibility, cache pruning, idempotence, retry after state change, flag/coordinate construction, and taskbar-model retention. A Windows fixture exposes an ordinary top-level app window with a known normal rectangle and controls for application-owned minimize/restore.

Headful UTIT verifies both the taskbar minimize path and the application's own minimize path. It observes `IsIconic`, `IsWindowVisible`, `GetWindowPlacement`, taskbar UI Automation presence, exact restore bounds, traces, process survival, and cleanup. Two final-candidate runs are blocking.

### 6. Evidence correction policy

- **A — task refinement:** command, ordering, fixture, or evidence-path refinements that preserve scope and gates.
- **B — design/spec correction:** a discovered Win32 timing or placement fact within the approved shelf scope requires updating design/spec/tasks and invalidating dependent evidence.
- **C — material change:** style/visibility mutation, new shell ownership, different platform, permission expansion, destructive behavior, or weakened physical/package gates requires user approval.

## Risks / Trade-offs

- **[Application immediately shows its iconic window again]** → the bounded snapshot reconciler drops the stable episode and retries asynchronous hiding while the identity remains eligible.
- **[HWND is reused between snapshot and placement]** → revalidate PID and stable identity immediately before mutation.
- **[Cross-thread call blocks GPUI]** → use `ShowWindowAsync` and perform no waiting in the UI loop.
- **[Shelf accidentally affects preview Explorer]** → pass the explicit owned-shell boolean; source and headful tests prove preview is observation-only.
- **[Normal restore bounds drift]** → perform no placement mutation and compare the complete pre/post `rcNormalPosition`; block release on any difference outside DPI rounding in the physical fixture.
- **[Console flooding]** → cache failure identity until a non-minimized/retired transition clears it.

## Migration Plan

No data or settings migration is required. Ship platform adapter, runtime reconciler, tests, and installer together. Rollback is the prior nested commit plus parent gitlink; no application placement data is persisted by SuperDesktop.

## Open Questions

None. The Win32 placement contract and requested Explorer parity determine the behavior.
