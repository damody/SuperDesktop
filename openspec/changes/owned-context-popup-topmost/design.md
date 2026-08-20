## Context

SuperDesktop creates task Jump Lists, taskbar background menus, and input/volume context menus as separate GPUI popup windows. Unlike task previews and Alt+Tab, these routes do not call the existing `platform-win` topmost adapter, so normal application windows can cover them.

The owned-shell runtime also runs several foreground futures that wake after timers and call `AsyncApp::update`. Native Windows operations can synchronously dispatch messages while GPUI already holds the application `RefCell`; a foreground future that resumes during this interval calls `borrow_mut` and panics with `RefCell already borrowed`. AppBar registration can legitimately fail when another shell reservation exists and must not turn this recoverable geometry condition into a process exit.

The approved source design is `docs/superpowers/specs/2026-08-20-owned-context-popup-topmost-design.md`.

## Goals / Non-Goals

**Goals:**

- Promote every separate owned right-click popup above normal windows exactly once with no unrelated activation.
- Fail closed and report promotion errors consistently.
- Preserve focus-loss dismissal.
- Replace panic-prone asynchronous application updates in SuperDesktop with a fallible borrow path.
- Keep the owned taskbar alive when AppBar registration is unavailable.
- Prove z-order, dismissal, contention recovery, and process survival headfully.

**Non-Goals:**

- Make configuration/settings windows permanently topmost.
- Promote context menus rendered inside the desktop's native surface.
- Add a persistent z-order polling worker.
- Change Explorer registration, AppBar ownership rules, or settings schema.
- Remove the existing infallible `AsyncApp::update` API used by unrelated GPUI consumers.

## Decisions

### Shared one-time popup promotion

An application-layer helper will extract the owned popup HWND and call `promote_owned_popup_topmost`. It returns a boolean, emits per-kind success/failure trace, writes failure through `report_error`, and removes the window on failure. Jump List, taskbar background, and system-control context creation closures call it before view construction and only store their window handle when promotion succeeded.

Alternative: use repeated z-order reconciliation. Rejected because it can reorder dismissed or unrelated windows and adds lifecycle state.

### Non-activating native boundary remains in platform-win

The existing adapter uses `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE)`, validates the HWND and calling process, and fails closed. UI crates receive no native types.

Alternative: a GPUI option. Rejected because current options do not expose a testable topmost guarantee for all popup routes.

### Add fallible AsyncApp update without breaking existing consumers

Vendored GPUI will add `AsyncApp::try_update`, using `try_borrow_mut`, checking quit state, and returning `anyhow::Result<R>`. Existing `update` remains source-compatible. SuperDesktop's timer, preview, transfer, auto-hide, refresh, and timed-shutdown callbacks migrate to `try_update`; contention is logged and the individual tick is skipped or retried by the next natural loop iteration.

One-shot operations that cannot naturally retry report rejection but do not panic. Repeating refresh loops continue on the next tick. No `catch_unwind` is used as normal control flow.

Alternative: catch the panic. Rejected because the panic hook still reports a crash and the borrow conflict remains uncontrolled.

### AppBar failure is an explicit fallback state

Initial AppBar registration failure records `taskbar:appbar-unavailable-owned-shell`, continues with owned monitor/taskbar geometry, and retains shell-hook ownership. Later auto-hide transitions keep their current recoverable behavior. Tests must prove the process remains alive and responsive after the trace.

## Risks / Trade-offs

- [A popup cannot be promoted] → Remove it, clear its slot, and report the exact popup kind; never claim open success.
- [Async contention drops a refresh tick] → Refresh/auto-hide loops run again within 50 ms; one-shot paths report rejection without terminating.
- [Topmost interferes with dismissal] → Retain activation subscriptions and verify focus-loss removal headfully.
- [Vendored GPUI change affects other crates] → Add only a new method, preserve the old API, and run workspace tests/Clippy.
- [AppBar remains unavailable] → Owned taskbar uses explicit geometry fallback and reports degraded reservation without quitting.

### Evidence corrections

- **A — task refinement:** test timing, evidence paths, or helper naming may change without altering requirements or gates.
- **B — design/spec correction:** an in-scope runtime discovery pauses affected tasks and updates design/spec/tasks plus stale evidence.
- **C — material change:** weakening topmost, dismissal, no-panic, or AppBar-survival gates; changing framework/platform; or adding destructive/external actions requires user approval.

## Migration Plan

1. Add and test the fallible GPUI update primitive.
2. Migrate SuperDesktop asynchronous update sites and add contention/error traces.
3. Add shared popup promotion and apply it to all independent right-click popup routes.
4. Extend UTIT for z-order, dismissal, AppBar fallback, and crash-log absence.
5. Run full quality/release/package gates and integrate scoped commits.

Rollback is a normal revert of the implementation and submodule pointer. No persisted data migration is required.

## Open Questions

None. Desktop in-surface menus are intentionally excluded because their z-order is the desktop window's z-order, not an independent popup HWND.
