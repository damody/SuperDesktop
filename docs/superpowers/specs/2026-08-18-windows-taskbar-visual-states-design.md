# Windows Taskbar Visual States Design

## Objective

SuperDesktop shall reproduce the visible Windows 11 taskbar states used by ordinary desktop applications: every running task has a stable running indicator, active and grouped states remain distinguishable, `ITaskbarList3` progress appears with Windows-compatible priority and colors, and `FlashWindowEx` attention requests produce a bounded Windows-style flash/highlight sequence.

The implementation must continue to work when Explorer is absent. Existing applications must not require a SuperDesktop-specific SDK for the documented Windows behaviors covered by this design.

## Current state

`taskbar-ui` already defines independent `ProgressState` and `attention` fields and renders a bottom border plus a progress strip. Product composition currently supplies attention from an incomplete window observation and never supplies real application progress. The renderer uses arbitrary colors and a full-width border rather than Windows 11 running indicators, does not animate indeterminate progress or attention, and does not implement grouped progress selection.

## Chosen approach

Use a process-isolated taskbar-state compatibility boundary, with `platform-win` owning documented Windows observation/admission and `superdesktop-app` consuming owned generation-bound DTOs.

Two alternatives were rejected:

- Inferring progress from window title/process activity cannot recover a real percentage, paused state, error state or explicit removal.
- A SuperDesktop-only SDK would not support existing Windows applications that already use `ITaskbarList3` and `FlashWindowEx`.

## Architecture

### Taskbar-state protocol

Add bounded DTOs for:

- window identity: PID, session ID, HWND identity and observation generation;
- progress state: none, indeterminate, normal, paused and error;
- completed and total `u64` values with safe normalized permille/percentage;
- attention request, stop reason, cadence, remaining flashes and persistent highlight;
- overlay generation, authoritative snapshot generation, overflow and provider health.

Every mutation requires a live same-session HWND and a current host generation. Stale events cannot restore an old progress or attention state.

### Progress compatibility

The broker accepts the documented semantics of `ITaskbarList3::SetProgressValue` and `SetProgressState`. The implementation route is evidence-driven: use the documented Windows COM/taskbar contract when it remains reachable without Explorer; otherwise provide an isolated compatibility class/proxy admitted only in committed Shell mode. Preview mode does not replace or register system COM ownership.

For grouped task buttons, select progress exactly in this priority:

1. error;
2. paused;
3. normal;
4. indeterminate.

When multiple windows have determinate progress at the same priority, display the least complete progress. `SetProgressValue` implies normal unless paused/error blocks it, and clears indeterminate. A zero total, invalid fraction, dead HWND or wrong session fails closed.

### Attention compatibility

Observe Shell Hook attention/flash notifications and bind them to the current HWND generation. The model records whether the request is finite or `FLASHW_TIMERNOFG`, the requested count and the Windows timeout. A zero timeout uses the current system caret blink interval.

The UI alternates between the normal task state and the attention state. A finite request stops after its admitted count and leaves a steady attention highlight; a timer-no-foreground request continues until the window becomes foreground. Activating, closing or retiring the window immediately clears flash state.

### Rendering

Every available running task renders a 3px bottom running indicator:

- inactive running window: centered 6px neutral/blue indicator;
- active window: centered 16px accent indicator;
- grouped window: 16px indicator plus a second 1px offset layer;
- minimized window: same geometry at reduced opacity;
- unavailable/retired window: neutral indicator and disabled action.

Progress is drawn behind task content across the full button width:

- normal: Windows green proportional fill;
- paused: Windows yellow proportional fill;
- error: Windows red proportional fill;
- indeterminate: moving green highlight segment;
- none: no progress fill.

The visible task label is not replaced by numeric text. Accessibility state includes the exact percentage and progress kind. This matches Windows, which communicates determinate progress through the filled button rather than printing a percentage label.

Attention is a separate layer. During the active flash phase the button alternates between its normal background and a Windows amber/orange attention surface. The progress layer and running indicator remain present. When flashing ends without foreground activation, a steady amber indicator remains.

High contrast uses system/high-contrast colors and disables opacity-only distinctions. Reduced-motion mode replaces animation with a steady attention/progress state.

## State independence and reconciliation

`active`, `minimized`, `grouped`, `attention`, `progress`, `badge` and `availability` are independent fields. Updating one field must not clear another. Authoritative window snapshots retain overlay state only for the same validated window generation; HWND reuse or host restart clears it.

The renderer computes one immutable visual presentation per frame from these fields. It does not mutate the reducer while rendering.

## Failure handling

- Broker crash clears progress/flash state but leaves task switching operational.
- Queue overflow preserves stop/terminal events, records overflow and schedules one full reconciliation.
- Unsupported `ITaskbarList3` operations remain unavailable without fabricating progress.
- Callback panic is caught at the FFI/COM boundary and cannot unwind into the host or GPUI process.
- Explorer-present preview never takes over progress COM/class ownership.

## Verification

Automated gates cover:

- all running/active/minimized/grouped indicator geometries;
- progress priority, least-progress group selection and zero/overflow arithmetic;
- normal/paused/error/indeterminate color and accessibility states;
- finite flash, timer-no-foreground, foreground stop, close and HWND reuse;
- independent overlay fields and stale-generation rejection;
- high contrast and reduced motion;
- complete locked/offline workspace tests and clippy warnings-as-errors.

Headful Windows 11 gates use controlled ordinary applications that call `ITaskbarList3` and `FlashWindowEx`. Captures must show every state at the host DPI, include UIA names, raw timestamps, broker/window identities and binary hashes, and compare taskbar geometry/colors with the built-in Windows reference.

## Rollout and rollback

The compatibility boundary is admitted only in committed Shell mode until the Explorer-free lifecycle gate passes. Disabling it reverts to ordinary task switching without progress/flash overlays. No filesystem migration is required, and guardian recovery remains unchanged.
