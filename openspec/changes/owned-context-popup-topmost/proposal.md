## Why

SuperDesktop right-click popup windows can be covered by ordinary application windows because their native z-order is not promoted consistently. The same owned-shell session can also terminate when an asynchronous UI callback re-enters GPUI while its application `RefCell` is already borrowed; an unavailable AppBar must remain a recoverable condition rather than contributing to an application exit.

## What Changes

- Promote every independent owned right-click popup to native topmost exactly once without activating unrelated windows.
- Apply the policy to task Jump Lists, taskbar background context menus, and input/volume system-control context menus.
- Reject and remove a popup when HWND acquisition or topmost promotion fails; report the error through console and trace.
- Preserve existing focus-loss dismissal and avoid polling or persistent z-order workers.
- Add a fallible GPUI asynchronous application update path that returns borrow contention as an error instead of panicking.
- Migrate SuperDesktop timer/refresh callbacks from infallible re-entrant application updates to the fallible path and log rejected ticks.
- Keep AppBar registration failure explicitly recoverable: the owned taskbar remains alive and uses its existing geometry fallback.
- Extend headful UTIT coverage for native topmost state, focus-loss dismissal, AppBar-unavailable survival, and absence of `RefCell already borrowed` panics.

## Capabilities

### New Capabilities

- `owned-context-popup-topmost`: Defines native z-order, non-activation, failure, and dismissal requirements for independent owned right-click popups.
- `owned-shell-runtime-resilience`: Defines non-panicking asynchronous UI updates and recoverable AppBar-unavailable behavior.

### Modified Capabilities

None.

## Impact

- Vendored GPUI `AsyncApp`: new fallible update API preserving the existing API for other consumers.
- `superdesktop-app`: shared popup promotion policy and migration of asynchronous runtime loops/timers.
- `platform-win`: existing owned popup promotion adapter remains the sole Win32 z-order boundary.
- `superdesktop-utit` and PowerShell captures: native style, dismissal, and crash-survival evidence.
- No settings schema, provider protocol, or external dependency changes.
