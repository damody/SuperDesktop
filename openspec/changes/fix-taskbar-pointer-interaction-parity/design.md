## Context

The taskbar view currently attaches child-specific callbacks but the root also owns a right-button handler for the taskbar background menu. Task and notification controls use a mix of click, mouse-down, and mouse-up handlers, so a handled right click can still reach the root. Input and volume expose only primary activation. Existing headful scripts invoke several controls through UIA and accept broad traces, leaving the exact physical left/right contract unverified.

The authoritative source design is `docs/superpowers/specs/2026-08-20-taskbar-pointer-interaction-parity-design.md`. The work crosses `taskbar-ui`, `superdesktop-app`, Win32 NotifyIcon compatibility, and `superdesktop-utit`, so it uses a detailed task plan.

## Goals / Non-Goals

**Goals:**

- Make every affected pointer gesture emit exactly one Explorer-aligned action.
- Prevent taskbar-background handling from replacing child context behavior.
- Give input and volume fixed-target owned context menus.
- Preserve task application activation/minimize/restore/group and Jump List semantics.
- Prove exact visible/overflow notification callback kinds.
- Make real-pointer evidence mandatory in UTIT with bounded Explorer recovery.

**Non-Goals:**

- Reimplement Windows Quick Settings, third-party application menus, or undocumented Explorer internals.
- Add arbitrary executable, URI, verb, or argument launch inputs.
- Redesign taskbar visuals unrelated to pointer state or popup ownership.
- Claim unsupported audio/input hardware paths passed.

## Decisions

### Centralize gesture classification at the view boundary

`taskbar-ui` will expose typed primary/context action callbacks and use button-specific handlers that stop propagation after handling. Small pure helpers will describe gesture-to-action mapping for unit tests. This keeps raw GPUI events out of application composition and avoids scattered source-string exceptions.

Alternative: patch only the root background handler. Rejected because it cannot express input/volume context behavior or prove exact notification/task semantics.

### Keep product mutation and fixed launches in composition/platform code

The view emits typed actions. `superdesktop-app` owns popup slots, mutual exclusion, tracing, and dispatch. Input context offers Language preferences through the existing fieldless status command. Volume context offers fixed Open volume mixer and Sound settings actions implemented as compile-time Windows targets in `platform-win`; no target crosses the callback boundary.

Alternative: launch Settings directly from UI elements. Rejected because it weakens the existing typed authority boundary and complicates testing.

### Treat popups as one ownership domain

Before opening a child popup, composition dismisses conflicting taskbar background, Jump List, overflow, system-flyout, and status-context windows. The same control toggles its popup closed. Popup deactivation and Escape clear their owning slot. A monotonically checked slot/handle prevents a stale dismiss callback from clearing a replacement window.

Alternative: permit overlapping windows and rely on z-order. Rejected because current failures are partly caused by a later background popup covering the intended child popup.

### Verify physical pointer input and exact terminal evidence

UTIT cases will use cursor movement and explicit left/right injection against UIA-resolved bounds. Reports must include control identity, pointer button, expected action, exact trace/callback, unintended-popup absence, popup lifecycle, and recovery state. NotifyIcon evidence must parse separate Activate and Context callback records instead of accepting the presence of any callback.

## Platform and data flow

1. UIA identifies the taskbar child and the fixture moves the pointer to its center.
2. GPUI classifies the button and invokes one typed callback while stopping propagation.
3. Composition dismisses conflicting popup ownership and performs or opens the requested action.
4. For notification icons, the event crosses the existing notification host and is translated by negotiated NotifyIcon version to the exact native callback payload.
5. Trace/UIA/fixture output is reconciled into a versioned UTIT JSON report and artifact hashes.

## Failure handling and observability

- Missing controls, unavailable provider state, wrong callback kind, duplicate callback, background-menu appearance, popup overlap, timeout, or lost recovery is a failed case.
- Hosts with fewer than two input profiles may mark only the profile-mutation subcheck not-applicable; pointer routing and popup checks remain mandatory.
- Every new action writes a stable trace token. Reports contain no raw input profile identity.
- Explorer-free cases retain a watchdog and restore Explorer in `finally`.

## Risks / Trade-offs

- [GPUI pointer phase differences cause duplicate or root events] -> Cover down/up/click routing with unit source contracts and real-pointer headful evidence; stop propagation in the child handler that owns the gesture.
- [Fixed Settings URI behavior changes across Windows builds] -> Assert launch admission only, never claim Settings visibility, and keep targets compile-time constants.
- [Popup dismissal races clear a newer popup] -> Compare the dismissing handle/generation with the current slot before clearing it.
- [Headful timing is flaky] -> Poll bounded UIA state and exact trace records rather than relying on fixed sleeps alone.

## Migration Plan

Land typed view callbacks and unit tests first, then composition/platform actions, then strengthen fixtures and add UTIT catalog cases. Rollback is source-level: remove the new context surfaces/callbacks and catalog entries together; no persisted schema or public IPC migration is required. Existing evidence remains immutable and is superseded by the new report schemas.

## Adjustment policy

- A-level changes may split tasks, adjust polling, or refine commands without altering requirements or gates.
- B-level corrections within this approved pointer scope require updating design, spec, tasks, and dependent evidence before continuing.
- C-level changes to scope, public contracts, fixed targets, permissions, destructive actions, or mandatory evidence require user approval. Gates cannot be weakened silently.

## Open Questions

None. Windows build-specific absence of an optional Settings surface is handled as truthful launch rejection, not as a design ambiguity.
