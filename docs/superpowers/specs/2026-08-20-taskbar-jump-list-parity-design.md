# Taskbar Jump List Parity Design

## Goal

Make a task button's right-click behavior match the Windows taskbar contract: the Jump List is the only popup shown, its entries belong to the selected application, and essential taskbar actions remain available.

## Scope

- Cancel a pending hover-preview open when the user right-clicks a task button.
- Close an already visible task preview before opening the Jump List.
- Prevent a stale hover timer from reopening the preview while the Jump List is active.
- Stop presenting the global Recent-folder contents as application-specific Jump List history.
- Always expose `Pin to taskbar` or `Unpin from taskbar`.
- Always expose the appropriate close action for a running application.
- Retain the existing minimize and maximize window actions.

Reading and reproducing every application's native automatic/custom destination list is outside this focused correction. Until SuperDesktop can associate Windows destination data with the selected application's real AppUserModelID, it must omit Recent rather than display unrelated global history.

## Interaction Design

Right-clicking a task button performs one atomic popup transition:

1. Invalidate the hover-preview controller generation and clear its tracked task/popup hover state.
2. Remove any owned preview window and clear its HWND identity.
3. Close a previously open Jump List when the same toggle path requires dismissal, or build a new Jump List for the selected application.
4. Open only the Jump List and give it keyboard focus.

The resulting Jump List is grouped like a Windows taskbar menu:

- Provider-supplied application tasks may appear when valid.
- The inaccurate global `Recent` and `Frequent` groups do not appear.
- Local taskbar actions appear at the bottom.
- A single-window application shows `Close window`; a grouped application shows `Close all windows`.
- Exactly one of `Pin to taskbar` and `Unpin from taskbar` is present according to persisted state.

## Architecture

`HoverPreviewController` gains an explicit cancellation operation. This operation increments its generation so already scheduled asynchronous opens fail their token check, and resets both task and popup-hover state.

The task-context callback owns popup exclusivity because it already has access to the preview controller, preview window slot, active preview HWND, Jump List window slot, selected task identity, and persisted pin state. It performs preview cancellation before any early return or provider request.

Jump List composition remains bounded and validated. The application callback supplies an empty Recent/Frequent set unless a future provider can prove that destinations belong to the selected application. Provider task commands and local actions continue through the existing typed command path.

## Error Handling

- Preview removal is best-effort, but the slot and HWND identity are cleared deterministically.
- A stale scheduled preview cannot reopen because its generation token is invalidated.
- Provider failure falls back to local actions, so pinning and closing remain available.
- Window mutations retain exact HWND, process ID, and window-identity validation.
- Settings persistence failure is reported through the existing action trace and does not claim success.

## Verification

Extend the focused headful UTIT case to use real pointer input and verify:

- hover can schedule or display a preview before the context action;
- right-click opens a Jump List and leaves no `Window previews` surface;
- no delayed preview appears while the Jump List remains open;
- the menu contains the correct pin/unpin label;
- the menu contains `Close window` or `Close all windows`;
- minimize, maximize, and close still mutate the exact fixture window;
- the report records each observed condition and is accepted by `validate-report`.

Unit tests cover controller cancellation and Jump List group composition. Formatting, package tests, Clippy with warnings denied, release build, and installer packaging are the completion gates.
