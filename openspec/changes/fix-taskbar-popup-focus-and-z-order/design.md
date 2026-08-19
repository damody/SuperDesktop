## Context

`TaskbarContextView` owns a focus handle and dismiss callback but does not retain a window-activation observer, unlike the notification overflow and system flyout views. `open_task_preview` creates both passive hover and active click previews as GPUI popup windows; `WindowKind::PopUp` does not place their HWNDs in the Windows topmost band. The hover route must remain non-activating, HWND operations must be restricted to current-process windows, and the existing Explorer-free boundary must remain intact.

## Goals / Non-Goals

**Goals:**

- Close the owned taskbar context menu when its popup window loses activation.
- Keep focus movement between menu descendants from causing dismissal.
- Put task previews above ordinary application windows without stealing focus on hover.
- Reject invalid, retired, zero, and foreign HWNDs and expose deterministic trace/test evidence.

**Non-Goals:**

- Redesign popup geometry, hover timing, menu commands, thumbnail rendering, or other flyouts.
- Change the global foreground window, install hooks, delegate to Explorer, or alter settings/extension ABI.

## Decisions

### Observe context-window activation in the view

`TaskbarContextView::new` will register `Context::observe_window_activation` and retain the returned `Subscription`. When `Window::is_window_active` becomes false, the observer clones and invokes the existing dismiss callback. This follows established owned-popup behavior and observes the correct window-level boundary. Element `focus_out` was rejected because descendant focus movement can produce false dismissal; polling in the composition root was rejected as redundant and timing-dependent.

### Promote previews with a non-activating owned-HWND adapter

The Windows taskbar adapter will add a focused helper that validates a nonzero live HWND, confirms `GetWindowThreadProcessId` matches `GetCurrentProcessId`, and calls `SetWindowPos` with `HWND_TOPMOST` plus `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`. `open_task_preview` will call the helper immediately after resolving its destination HWND and before creating the view. `SWP_NOACTIVATE` preserves the passive hover contract; an already activated click preview remains active. Activating the hover popup or using `SetForegroundWindow` was rejected because it steals user focus. `WindowKind::PopUp` alone was rejected because it does not establish the required z-order.

### Fail closed when stacking cannot be guaranteed

If HWND resolution or topmost promotion fails, preview creation returns without publishing the window handle, removes the just-created popup, resets the active-popup identity, and traces `task-preview:topmost-rejected`. Success traces `task-preview:topmost-established`. This avoids leaving a misleading preview behind other windows.

## Risks / Trade-offs

- [Risk] A topmost preview could cover unrelated applications while the pointer remains over the taskbar or preview. → Mitigation: retain the existing hover-leave pointer monitor, grace timer, and single-preview slot.
- [Risk] Deactivation can race with command-triggered dismissal. → Mitigation: all routes use the existing idempotent remove-window callback and clear the shared slot.
- [Risk] A stale HWND could be reused by another process. → Mitigation: validate liveness and current-process ownership immediately before `SetWindowPos`.
- [Risk] Platform flags regress hover focus behavior. → Mitigation: assert `SWP_NOACTIVATE`, compare foreground HWND in headful evidence, and retain the existing hover/click activation tests.

## Migration Plan

No data migration is required. Land platform validation first, then preview composition and context lifecycle changes, followed by focused and headful evidence. Rollback removes the new helper call and activation subscription; no persisted state or public contract requires reversal.

## Open Questions

None. The user authorized the agent to resolve implementation details and continue without confirmation.
