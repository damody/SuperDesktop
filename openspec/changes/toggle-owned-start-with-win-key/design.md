## Context

The shell-scoped `WH_KEYBOARD_LL` hook currently recognizes only chords such as Win+E and Win+D. A standalone Win key passes through, but Explorer is intentionally absent in owned-shell mode, so no Start surface appears. The owned taskbar already exposes one guarded Start callback that toggles the GPUI Start window and applies monitor/alignment settings.

Microsoft's current Windows shortcut contract defines the Windows logo key as opening or closing Start. The input decision must wait until the gesture is known to be standalone; triggering on keydown would race every Win chord and key-repeat stream.

## Goals / Non-Goals

**Goals:**

- Make either standalone Windows logo key toggle the owned Start menu exactly once on release.
- Keep supported and unsupported Win chords from opening Start after the chord ends.
- Reuse the taskbar Start callback and existing focus, monitor, alignment, dismissal, and panic containment.
- Preserve bounded hook lifetime, shell-only scope, console diagnostics, and Explorer recovery in headful tests.

**Non-Goals:**

- Reimplementing every Windows-key chord in this change.
- Delegating Start to Explorer, StartMenuExperienceHost, synthetic input, or protocol activation.
- Changing Start layout, search, taskbar alignment settings, or preview-mode keyboard ownership.

## Decisions

### 1. Recognize the gesture with an explicit release-time reducer state

Add a compact atomic state representing idle, candidate left/right Win, and cancelled left/right Win. The hook consumes Win down/up while tracking the gesture. The matching release emits `ToggleStart` only from candidate state. Any other keydown—including a second Win key—moves the gesture to cancelled before chord routing proceeds.

This is preferred over keydown activation because a chord cannot be distinguished at first keydown, and over timers because native Start behavior has no dwell requirement. Atomic state keeps the callback allocation-free and testable without a Windows desktop.

### 2. Preserve chord routing as an independent second reducer stage

Standalone-state reduction runs first to update eligibility, then existing chord reduction handles actions such as Win+E. A supported chord remains consumed and queued once; an unsupported chord passes onward while still cancelling the standalone candidate. Matching Win release clears either candidate or cancelled state.

### 3. Queue a dedicated `ToggleStart` action

Extend the bounded bitset action queue with one stable power-of-two code. The GPUI refresh loop resolves and invokes `callbacks.start`, the same callback used by the taskbar Start button. A missing callback is a console error and does not panic.

This is preferred over sharing the misleading `OpenSearch` action name and over duplicating the Start open/close code.

### 4. Verify pure semantics and the real shell boundary

Reducer tests cover both Win keys, repeats, chord cancellation, dual Win keys, mismatched releases, and action queue round trips. Source-contract tests require the dedicated action and shared callback route. A Windows headful UTIT script uses real injected Win down/up events only as test input, verifies open then close from window/trace evidence, and restores the prior Winlogon Shell and Explorer process in a `finally` block.

The release, Clippy, workspace, installer, and strict OpenSpec gates remain blocking.

### 5. Evidence correction policy

- **A — task refinement:** commands, order, or evidence filenames may be refined without changing requirements or gates.
- **B — design/spec correction:** an implementation discovery within the approved standalone-toggle scope pauses affected work; design, spec, tasks, and stale evidence are updated and revalidated.
- **C — material change:** changes to shortcut scope, shell ownership, required platform, permissions, destructive behavior, or blocking gates require user approval.

## Risks / Trade-offs

- **[Stale key state after hook restart]** → Reset standalone and chord atomics during start and worker shutdown.
- **[A Win chord opens Start on release]** → Cancel on every non-Win keydown before running chord matching; cover both supported and unsupported chords.
- **[Dual Win keys produce duplicate toggles]** → Treat the second distinct Win keydown as cancellation and clear on matching tracked release.
- **[Hook suppresses native behavior outside owned shell]** → Instantiate the hook only in the existing shell-mode path; preview mode remains pass-through.
- **[UI borrow panic while dispatching]** → Resolve the callback inside `try_update`, end handle borrows before invoking it, and retain `guard_ui_action` inside the callback.
- **[Headful failure strands the machine without Explorer]** → Snapshot registry/process state, use bounded waits, and restore Explorer in `finally`; restoration evidence is blocking.

## Migration Plan

There is no data migration. Ship the new hook reducer and runtime route in the normal SuperDesktop binary and installer. Rollback is the prior nested SuperDesktop commit plus parent gitlink; configuration files remain compatible.

## Open Questions

None. The approved scope and Windows shortcut contract determine the behavior.
