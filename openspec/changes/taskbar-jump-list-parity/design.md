## Context

SuperDesktop owns taskbar buttons, delayed hover previews, Jump List popup windows, provider-backed commands, and local window actions. These paths currently share neither an explicit popup-exclusion transition nor an application-ownership gate for Recent data. The provider enumerates the global Recent known folder, which produces unrelated URI/link labels for arbitrary task buttons. The approved source design is `docs/superpowers/specs/2026-08-20-taskbar-jump-list-parity-design.md`.

The implementation must remain Explorer-free, keep GPUI callbacks non-blocking except for the existing bounded provider query, preserve exact window-identity checks, and continue working in preview and owned-shell modes.

## Goals / Non-Goals

**Goals:**

- Match File Explorer's popup exclusivity: a task right-click leaves only the Jump List visible.
- Invalidate pending preview timers before opening the menu.
- Omit Recent/Frequent data whose ownership by the selected application cannot be proven.
- Put pin/unpin before one final close command in an unheaded bottom action area.
- Preserve exact minimize, maximize, pin/unpin, single-window close, and grouped close execution.
- Prove behavior with deterministic unit tests and physical-pointer headful UTIT evidence.

**Non-Goals:**

- Reverse-engineer automatic-destination files.
- Claim parity with every application's custom Jump List extension.
- Introduce a new public protocol version or a new external dependency.
- Remove the already requested minimize/maximize commands.

## Decisions

### Explicit preview cancellation owns the race boundary

`HoverPreviewController` will expose `cancel()`, which clears task/popup hover state and increments its generation. The task-context callback will call it before any early return, remove the preview window slot, and clear the active preview HWND. Existing async timers then fail `can_open` or `can_close` token checks.

Alternative: rely on pointer-leave and close grace. Rejected because a right-click can occur before pointer-leave and an already queued open can win after the Jump List appears.

### Fail closed on destination ownership

The provider will not enumerate `FOLDERID_Recent` for an arbitrary executable. Until the request carries a verified AppUserModelID and uses Windows application document-list APIs, Recent and Frequent SHALL be empty. The application may still show provider tasks that are derived from the selected canonical executable.

Alternative: filter global Recent entries by extension or file association. Rejected because association does not prove that an item belongs to the selected application's Jump List.

### Explorer-aligned bottom actions

Local commands will be composed in this order: optional minimize/maximize, pin/unpin, then exactly one close command. Grouped tasks use `Close all windows`; single-window tasks use `Close window`. The UI will render the local group without an `Actions` heading while preserving separator spacing.

Alternative: retain both close commands and the heading. Rejected because it duplicates the terminal action and differs from File Explorer's bottom command area.

### Evidence and adaptive corrections

The focused UTIT will hover the fixture, right-click its task button, and assert that `Window previews` is absent immediately and after the hover delay. It will assert pin/unpin and the applicable close label, then exercise exact window actions.

- **A — task refinement:** command order, script timing, fixture discovery, or evidence-path changes that do not change requirements or gates may update tasks directly.
- **B — design/spec correction:** an implementation discovery within approved scope pauses affected work and updates design, spec, tasks, and stale evidence together.
- **C — material change:** weakening popup exclusivity, showing unowned Recent data, removing required pin/close actions, changing platform/framework, or lowering required evidence needs user approval.

## Risks / Trade-offs

- [Some applications temporarily show no Recent section] → Omission is truthful and safer than global unrelated history; future AppUserModelID work can add scoped destinations.
- [Right-click occurs while a preview window is being created] → Generation cancellation happens before window removal and the open path rechecks the token.
- [Grouped task selection has multiple exact windows] → Close operates on the captured application window set; single-window actions retain captured HWND/PID/window identity.
- [Provider unavailable] → Local actions are composed independently and remain visible.
- [UTIT pointer timing is flaky] → Foreground the owned taskbar, use physical input with bounded settling, and assert both immediate and delayed absence.

## Migration Plan

1. Add controller cancellation and tests.
2. Coordinate preview dismissal in the task-context callback.
3. Suppress unscoped Recent/Frequent provider output.
4. Recompose and render Explorer-aligned bottom actions.
5. Extend and run the focused UTIT, then run package and release gates.

Rollback is a normal git revert of the implementation commit and submodule pointer. No persisted schema migration is required; existing pins remain valid.

## Open Questions

None. Application-specific Recent support is deliberately deferred until ownership can be proven through a real AppUserModelID-backed source.
