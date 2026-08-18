## Context

The approved source design is `docs/superpowers/specs/2026-08-18-windows-taskbar-visual-states-design.md`. `taskbar-ui` already contains independent progress and attention fields, but production composition only provides partial attention and no real progress. Existing rendering uses arbitrary full-width borders and static strips rather than Windows 11 indicator geometry and animation.

The change spans Windows observation/compatibility, process isolation, contracts, reconciliation, GPUI animation, accessibility and release evidence. Normal steady-state Shell mode must work without Explorer; preview must remain non-invasive.

## Goals / Non-Goals

**Goals:**

- Reproduce Windows 11 active/inactive/minimized/grouped running indicators.
- Consume documented progress semantics and Windows state priority for ordinary applications.
- Observe and render finite and foreground-terminated attention flashing.
- Keep every task state independent, generation-bound and fail-closed.
- Prove colors, geometry, cadence, accessibility and Explorer-free behavior on Windows 11.

**Non-Goals:**

- Infer fake progress from titles, CPU usage or application activity.
- Replace arbitrary undocumented taskband persistence or pinning protocols.
- Print percentage text on buttons when Windows represents progress as proportional fill.
- Register system compatibility ownership while Explorer owns preview mode.

## Decisions

### Isolated taskbar-state provider

Add bounded taskbar-state DTOs and an isolated provider/compatibility boundary. `platform-win` owns HWND/PID/session validation and documented Windows callbacks. `superdesktop-app` consumes owned snapshots and never receives raw COM pointers, HICONs or borrowed structures.

This is preferred over direct GPUI integration because a malformed callback or third-party COM interaction must not terminate the desktop process.

### Evidence-driven `ITaskbarList3` compatibility

First verify the documented Windows `CLSID_TaskbarList` behavior while Explorer is absent. If progress calls remain observable through a documented route, use that route. If they require an Explorer-owned compatibility endpoint, implement the minimum isolated Shell-mode compatibility proxy needed for `SetProgressValue` and `SetProgressState`, without replacing COM ownership in preview. The gate remains failed until an unchanged controlled application using the normal Windows API drives SuperDesktop progress.

### Exact grouped progress reducer

Maintain progress per validated top-level HWND. Group reduction uses Windows priority: error, paused, normal, indeterminate. Same-priority determinate collisions choose the least complete value. `SetProgressValue` implies normal unless paused/error blocks it and clears indeterminate. `NOPROGRESS` removes state.

All completed/total arithmetic uses `u64`, rejects total zero and converts to bounded permille only after checked division.

### Shell Hook attention reducer

The existing registered Shell Hook path publishes attention start/stop with HWND generation. Flash cadence uses the requested timeout or the system caret blink interval when zero. Finite flash exhausts the admitted count and leaves steady attention; timer-no-foreground continues until foreground activation. Activation, close, retirement, restart and HWND reuse clear attention.

### Immutable visual presentation

`taskbar-ui` computes a pure `TaskVisualState` from active, minimized, grouped, availability, progress, attention, animation phase and accessibility preferences.

- inactive: centered 6px, 3px-high indicator;
- active: centered 16px accent indicator;
- grouped: active-width indicator plus a second offset layer;
- minimized: same geometry at reduced opacity;
- unavailable: disabled neutral indicator.

Progress fills the button background proportionally: green normal, yellow paused, red error, moving green segment indeterminate. Attention alternates the background using Windows amber while preserving progress and indicators. High contrast uses explicit colors; reduced motion uses steady states.

### Adjustment and evidence policy

- A-level refinements may split tasks, alter commands or tune implementation mechanics without changing requirements, gates or evidence.
- B-level corrections within approved scope update design/spec/tasks, pause affected work and mark dependent evidence stale.
- C-level changes to scope, public compatibility, blocking gates, thresholds, platform, permissions or external/destructive operations require user approval.

Blocking gates are `G-TASKBAR-INDICATOR`, `G-TASKBAR-PROGRESS`, `G-TASKBAR-ATTENTION`, `G-TASKBAR-A11Y`, `G-TASKBAR-ISOLATION`, `G-SHELL-NONINTERFERENCE` and `G-TRACE`.

## Risks / Trade-offs

- **[TaskbarList behavior depends on Explorer]** → retain an evidence-gated isolated compatibility branch; never claim ordinary-app support from model tests alone.
- **[Shell Hook events omit requested flash count]** → capture documented state where available and use a bounded Windows-default fallback explicitly recorded in evidence.
- **[HWND reuse restores stale overlay]** → bind every state to PID/session/HWND observation generation and clear on mismatch.
- **[Animation consumes excess frames]** → schedule only while an indeterminate/flash state is live and stop timers immediately afterward.
- **[Progress hides task identity]** → render progress behind content and preserve icon, label, underline and accessible name.

## Migration Plan

1. Add contracts, pure reducers and negative tests.
2. Add Windows attention observation and progress capability probe.
3. Implement the admitted provider/compatibility path selected by evidence.
4. Integrate supervised snapshots and pure GPUI presentation.
5. Capture Windows 11 reference comparisons and package the broker if new.

Rollback disables the taskbar-state provider and removes progress/flash visuals while preserving existing task switching, notification, status and guardian behavior. No user data migration is required.

## Open Questions

No user decision remains. The progress transport branch is resolved by a blocking implementation probe; failure to establish normal-application compatibility keeps the change incomplete rather than weakening the requirement.
