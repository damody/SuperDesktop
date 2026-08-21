## Why

Six shell-critical interactions regress when Explorer is absent: Windows-key chords can be missed, hidden minimized windows cannot complete a Win+D restore cycle, volume dragging blocks on provider round-trips, tray and input-profile commands fail across native/provider state transitions, and Windows notification callbacks do not reliably refresh the owned center. These failures prevent SuperDesktop from serving as a stable Explorer replacement and now need one integration change because they share keyboard, generation, lifecycle, and native-event boundaries.

## What Changes

- Track physical Windows/Shift key state in the owned-shell hook, make Win+D a reversible exact-window session, and preserve the existing fixed built-in Win+Shift+S protocol route.
- Make the volume slider render optimistically while coalescing native writes and committing the final pointer value.
- Resynchronize status-provider generations and tolerate bounded native propagation when activating exact TSF/HKL input profiles.
- Deliver version-correct notification-area left, keyboard, and context callbacks after validating the exact live icon owner.
- Keep Windows `UserNotificationListener` event callbacks alive, coalesce dirty refreshes, and confirm notification mutations before publishing state.
- Add focused unit, host, app, and physical GUI UTIT evidence plus installer/release verification.
- Non-goals: reintroducing Explorer as the persistent shell, third-party screenshot programs, Toast custom-action execution, synthetic mouse/keyboard replay, or changing public protocol compatibility beyond additive internal behavior.

## Capabilities

### New Capabilities

- `owned-shell-keyboard-parity`: Physical Windows-key state, reversible Win+D, and built-in Win+Shift+S routing.
- `smooth-system-volume`: Optimistic volume presentation, bounded coalescing, final commit, and authoritative recovery.
- `notification-area-interaction-parity`: Exact, version-aware left/keyboard/context callback delivery and stale-icon recovery.
- `input-profile-activation-recovery`: Generation resynchronization and bounded exact-profile activation observation.
- `windows-notification-event-recovery`: Long-lived WinRT event subscription, dirty reconciliation, and confirmed mutations.

### Modified Capabilities

None. The affected prior changes are not archived baseline capabilities; this change freezes their combined release behavior as new integration capabilities.

## Impact

- `platform-win`: low-level keyboard state, task-window snapshots, NotifyIcon callback delivery, system-status activation, and WinRT notifications.
- `taskbar-ui`: Show Desktop planning, volume interaction state, tray/input surfaces, accessibility, and dismissal.
- `superdesktop-app`: action dispatch, generation reconciliation, coalescing, shelf integration, and non-panicking GPUI scheduling.
- Provider hosts and protocol consumers: exact generation/identity validation without a breaking wire-format change.
- UTIT, installer, and OpenSpec evidence: physical Windows interaction, release binary, package, and installed hash verification.
