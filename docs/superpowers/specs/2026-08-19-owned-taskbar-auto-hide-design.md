# Owned Windows Taskbar Auto-Hide Design

## Scope

Add Windows-style automatic taskbar hiding to SuperDesktop without Explorer, `Shell_TrayWnd`, undocumented AppBar messages, or global input hooks. The feature applies to both Preview and Shell modes, preserves the configured one-to-three-row height, and is controlled by the owned taskbar settings surface.

## Decisions

1. Add `auto_hide` to `TaskbarSettings`, defaulting to false for backward compatibility.
2. Replace the disabled Automatically hide the taskbar row with an enabled localized toggle backed by the authoritative setting.
3. Use a platform adapter that observes the physical cursor and moves only a validated current-process taskbar HWND. No foreign window is mutated.
4. The visible position retains the current Preview work-area bottom or Shell monitor bottom. The hidden position leaves a two-physical-pixel reveal edge at that same bottom.
5. Pointer contact with the reveal edge shows the taskbar immediately. Pointer departure starts a 500 ms hide delay. Start, task context, taskbar settings, notification overflow, system flyouts, keyboard focus, active resizing, and attention keep it visible.
6. State changes use exact endpoint geometry first. A bounded animation may be added only after endpoint, input, and recovery gates pass; animation failure must fall back to the correct endpoint.
7. Shell auto-hide does not reserve desktop work area. Disabling auto-hide restores the normal AppBar reservation when the broker is available and the existing owned-bottom fallback otherwise.

## Components and data flow

- `settings-store` owns decoding, encoding, migration fallback, and atomic persistence of `auto_hide`.
- `taskbar-ui` renders the settings row and exposes typed model effects. It also reports visibility holds from owned popups and attention state without querying Explorer.
- `platform-win` owns cursor observation, HWND ownership validation, and exact visible/hidden positioning.
- `superdesktop-app` owns the auto-hide reducer and 50 ms reconciliation timer. The reducer consumes settings, pointer geometry, popup/focus/resize holds, and elapsed time; it emits only `Show`, `StartHideDelay`, `Hide`, or `NoChange`.

## State and timing

The state machine has `Visible`, `HidePending(deadline)`, and `Hidden`. Reveal-edge contact transitions directly to `Visible`. A visibility hold cancels a pending hide and reveals a hidden bar. When no hold exists and the pointer is outside the full visible taskbar rectangle, `Visible` enters `HidePending`; the transition to `Hidden` occurs only after 500 ms of continuously eligible time. Repeated timer ticks and duplicate settings refreshes are idempotent.

## Failure and lifecycle behavior

Invalid, retired, or foreign HWNDs fail closed with zero mutation. Cursor-query failure keeps the current position rather than hiding. Settings save failure leaves the previous authoritative behavior. Disabling auto-hide, normal shutdown, and recovery all restore the visible endpoint before releasing taskbar/AppBar ownership. Preview never mutates system work area. No path launches or invokes Explorer.

## Verification

- Unit tests cover legacy settings, round-trip, reducer timing, edge geometry, popup/focus/attention holds, duplicate ticks, invalid HWNDs, DPI, and negative monitor origins.
- Headful tests at the host's 175% scale verify visible height, two-pixel hidden edge, immediate reveal, 500 ms delayed hide, one-to-three-row persistence, locked/unlocked behavior, and right/settings UI.
- Explorer-present Preview and Explorer-free Shell captures prove the correct bottom anchor and process non-interference.
- Workspace fmt, locked/offline tests, clippy warnings-as-errors, release build, strict OpenSpec validation, and both NSIS installers remain blocking gates.

## Explicit limitations

This change owns SuperDesktop taskbar visibility. It does not implement the undocumented Explorer AppBar broker for third-party AppBars, and it does not claim system-wide multi-monitor work-area mutation while auto-hide is disabled; those remain separate Shell-broker work.
