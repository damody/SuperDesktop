## Context

SuperDesktop owns the taskbar window, settings, menus, Start, flyouts, focus model, and attention state, but the settings UI currently reports automatic hiding as unavailable. Explorer's AppBar broker cannot be a final dependency because Explorer is absent in the target Shell composition. The implementation must work on Windows 11 build 26200 at 175% DPI, preserve Preview non-mutation, and never move a foreign HWND.

## Goals / Non-Goals

**Goals:** Persist automatic hiding; match Windows reveal and delayed-hide behavior; retain configured rows and bottom anchor; keep owned interactions visible; fail closed; prove Preview and Explorer-free Shell behavior.

**Non-Goals:** Implement Explorer's undocumented AppBar broker, third-party AppBar compatibility, taskbar animation before endpoint correctness, automatic shell takeover, Windows 10 compatibility, or system-wide multi-monitor work-area mutation.

## Decisions

1. `TaskbarSettings.auto_hide` defaults false and remains in the current schema version. This preserves legacy files and isolates corruption to the field.
2. `TaskbarSettingsModel` exposes one enabled localized behavior toggle. Save remains an atomic typed effect; failure retains authoritative state.
3. A pure reducer owns `Visible`, `HidePending(deadline_ms)`, and `Hidden`. It consumes monotonic time, pointer availability/position, exact visible/reveal rectangles, enabled state, and visibility holds. Pure state makes timing, stale tick, and boundary behavior deterministic.
4. A `platform-win` adapter queries `GetCursorPos` and repositions only a live current-process taskbar HWND. Alternatives were Explorer AppBar auto-hide, which fails without Explorer, and a low-level mouse hook, which adds global callback and privilege risk.
5. Hidden geometry leaves two physical pixels at the relevant Preview work-area or Shell monitor bottom. Reveal is immediate; hiding requires 500 ms of continuous eligibility. Endpoint movement is idempotent and exact.
6. Owned Start, context menu, taskbar settings, Jump List, notification overflow, system flyout, focus, native resize, and attention are visibility holds. A hold reveals immediately and cancels a pending hide.
7. Shell auto-hide skips AppBar reservation. Disabling it reuses the existing registered AppBar path when available and the explicit owned-bottom fallback when unavailable. Preview never registers or modifies AppBar/work area.
8. Normal shutdown and setting disable restore the visible endpoint before lease teardown. Cursor-query or positioning failure preserves the current endpoint and emits a typed trace.

Implementation evidence can refine task order or split leaves as an A-level adjustment. A correction within this approved capability is B-level and must update design/spec/tasks and invalidate dependent evidence. Changes to timing, reveal thickness, platform claims, mutation authority, required gates, or Explorer dependency are C-level and require user approval; no gate may be weakened silently.

## Risks / Trade-offs

- [Polling costs CPU] → Reuse the bounded 50 ms runtime reconciliation cadence and avoid global hooks.
- [Popup closes between ticks] → Holds are authoritative each tick and the 500 ms delay prevents flicker.
- [DPI rounding loses reveal target] → Specify reveal thickness in physical pixels and test 100–200% plus negative origins.
- [Retired or reused HWND] → Revalidate liveness and current-process ownership before every move.
- [Auto-hide accidentally reserves work area] → Assert no AppBar reserve call while enabled and capture maximized-area-compatible Shell geometry.
- [Shutdown leaves bar hidden] → Restore visible endpoint before teardown and verify idempotent cleanup.

## Migration Plan

Decode missing `auto_hide` as false. Ship the setting disabled, so existing behavior is unchanged until the user enables it. Rollback ignores the unknown field and returns to always-visible behavior. No registry or installer migration is required.

## Open Questions

None.
