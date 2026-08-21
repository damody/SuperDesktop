## Context

SuperDesktop already owns separate keyboard, task-window, system-status, notification-area, and Windows-notification providers. The six reported failures occur where native state changes faster than GPUI/provider reconciliation: the hook infers modifier state, Show Desktop hides minimized windows from its own next snapshot, volume pointer moves synchronously await commands, identity-bound providers reject stale generations, and event sources rely on lifecycle-sensitive Windows callbacks. Explorer cannot be used as the persistent coordinator because SuperDesktop is the shell.

The authoritative source design is `D:/SuperExplorer/docs/superpowers/specs/2026-08-21-owned-shell-system-interactions-design.md`. Existing protocol payloads remain compatible and all native identities stay bounded and session-local.

## Goals / Non-Goals

**Goals:**

- Make physical Win+D reversible and Win+Shift+S reliably open the built-in overlay without reviving Explorer permanently.
- Decouple volume rendering from native command latency while preserving authoritative final state.
- Deliver exact tray callbacks and activate exact enabled input profiles across one bounded provider resynchronization.
- Make Windows notification add/remove events and confirmed mutations update the owned center without crashes.
- Prove each route with deterministic tests and physical GUI evidence.

**Non-Goals:**

- Persistent Explorer delegation, arbitrary URI/executable launch, third-party capture software, Toast custom actions, synthetic input replay, or a breaking provider-protocol migration.
- Broad visual redesign of the taskbar, flyouts, or notification center.

## Decisions

### Track modifier ownership in the keyboard hook

The hook records left/right Windows and Shift down/up state and uses it as the primary chord input. `GetAsyncKeyState` is only a recovery observation. Each physical non-modifier initial key-down can enqueue one action, repeats remain fenced, and the Windows-key release opens Start only if no chord consumed the gesture. This is more deterministic than querying asynchronous global state after the hook consumed an event.

### Merge the minimized shelf into Show Desktop restore planning

The first Win+D records only exact windows successfully minimized. `MinimizedWindowShelf` keeps the window iconic and visible to Windows but saves its `WINDOWPLACEMENT` and moves only the minimized-icon position beyond the virtual screen; it does not use `SW_HIDE`, which physical evidence proved non-reversible for some application frameworks. The second Win+D restores the default minimized-icon placement before restoring the exact window. Matching HWND, process ID, stable window identity, eligibility, and minimized state remain mandatory.

This is B-level correction `B-2026-08-21-MINIMIZED-PLACEMENT`: physical UTIT showed that externally hidden WinForms windows stayed invisible after a successful `SW_RESTORE`. The correction retains the approved hidden-shelf behavior and exact-identity gates while replacing the unreasonable native mechanism; all prior Show Desktop evidence is stale and rerun.

### Preserve the fixed screen-clipping protocol route

Win+Shift+S continues to invoke `ms-screenclip:///?source=HotKey` through the verified inbox broker path. The hook fix makes admission reliable; the protocol, overlay observation, cleanup, and no-fallback constraints remain unchanged. Direct executable discovery and generic caller-controlled launching remain forbidden.

### Use optimistic presentation with a one-in-flight volume coalescer

Pointer motion mutates a local displayed value immediately and submits the latest desired value to a coalescer. Only one provider command may be in flight; intermediate values are overwritten, not queued. Pointer release marks the latest value final, and completion schedules the next/final command or authoritative refresh. Keyboard increments use the same coordinator. This avoids both render stalls and unbounded command storms.

### Resynchronize identity-bound commands once

Tray and ordinary status commands validate the current host generation and exact native identity. A stale-generation response triggers one snapshot refresh and one replay only when the same target still exists. Input activation is the exception required by Windows' per-thread TSF contract: the SuperDesktop foreground process validates and activates the exact TSF/HKL identity, performs a bounded local observation, and then lets the isolated status host publish subsequent authoritative snapshots. Timeouts never fabricate success.

This is B-level correction `B-2026-08-21-FOREGROUND-TSF`: physical evidence proved that an out-of-process host can activate and observe its own TSF profile without changing the foreground GPUI thread, producing false deadline failures. Moving only the exact activation boundary into the foreground process preserves identity validation and provider isolation for observation while removing host-generation races from input switching.

### Keep native callbacks data-only

NotifyIcon and `UserNotificationListener.NotificationChanged` callbacks write bounded atomic/mutex state and signal a later provider/UI refresh; they never borrow GPUI state or await work. Tray dispatch chooses version-correct callback payloads and establishes foreground context before right-click delivery. Notification mutations publish only after authoritative absence is confirmed.

### Evidence-driven corrections

- **A — task refinement:** commands, task split/order, and evidence mechanics may change without altering requirements or gates.
- **B — design/spec correction:** a discovered Windows contract mismatch inside the approved scope pauses the affected package; design, specs, tasks, and stale evidence are updated together.
- **C — material change:** scope, public behavior, blocking gates, platform/permission model, external writes, or destructive operations require user approval. Gates may not be weakened silently.

## Risks / Trade-offs

- [Applications ignore or customize tray callbacks] → Deliver the documented version-specific message exactly, prove payloads with fixture windows, and report truthful per-icon failure rather than simulating menus.
- [TSF propagation exceeds ordinary latency] → Use a bounded deadline and fresh enumeration; preserve the prior active UI selection on timeout.
- [High-rate slider input overwhelms the provider] → Enforce one in-flight command and latest-value replacement, with a final-release commit.
- [WinRT access is denied or package identity is unavailable] → Keep NotifyIcon functionality independent and expose the truthful Windows-notification access state.
- [Explorer broker outlives snipping] → Track request ownership, validate canonical/session identity, and close only the request-owned broker after overlay disappearance.
- [Hidden window identity becomes stale] → Require the full exact identity before restore and drop unmatched targets.

## Migration Plan

No persisted-data or wire migration is required. Land platform reducers and tests first, then UI/app coordinators, provider-host recovery, physical UTIT, and packaging evidence. Rollback is a normal revert of the nested SuperDesktop commit and parent submodule pointer; no user data is transformed.

## Open Questions

None. Any observed Windows-version difference is handled through the B-level correction process without broadening scope or weakening gates.
