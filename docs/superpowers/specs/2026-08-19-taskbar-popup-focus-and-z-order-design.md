# Taskbar popup focus and z-order design

## Problem

The owned taskbar context menu remains visible after its popup window loses activation. The delayed task hover preview is created as a passive GPUI popup but is not promoted into the topmost window band, so ordinary application windows can cover it.

## Design

`TaskbarContextView` will retain a GPUI window-activation subscription, matching the established notification-overflow and system-flyout lifecycle pattern. When the context popup becomes inactive, the subscription invokes the existing dismiss callback. Escape, command activation, replacement, and teardown continue to use that same idempotent dismissal route. Focus movement between menu descendants must not dismiss the popup because dismissal is based on window activation rather than element focus.

The Windows taskbar platform adapter will expose a narrowly scoped helper that promotes a live popup owned by the current process to `HWND_TOPMOST` with `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`. `open_task_preview` will call it after obtaining the destination HWND and before the preview is handed to the user. Both hover and click previews become topmost; the hover path remains passive and does not steal keyboard focus, while the click path preserves its existing explicit activation behavior.

Invalid, retired, zero, or foreign HWNDs are rejected. Promotion failure is traced and the popup is removed instead of exposing a preview with an incorrect stacking contract. No Explorer window, shell tray HWND, global hook, or foreground-window mutation is introduced.

## Alternatives

Element-level `focus_out` dismissal was rejected because moving focus between menu items can look like focus loss and close the menu prematurely. Runtime-level polling was rejected because GPUI already exposes deterministic window activation notifications. Activating the hover preview or calling `SetForegroundWindow` was rejected because a passive hover surface must not steal focus. Relying only on `WindowKind::PopUp` was rejected because it does not establish the required Windows topmost z-order.

## Verification

Automated tests will assert that the context view retains an activation subscription and dismisses on deactivation, that preview promotion validates current-process ownership and uses non-activating topmost flags, and that hover previews remain non-activating while click previews retain keyboard focus. Focused crate tests, formatting, and compilation will run. A headful check will open the context menu and deactivate it, then hover a task while another ordinary window overlaps the preview area and record that the menu closes and the preview stays above that window without changing the foreground window.

## Scope

This change affects only the owned empty-taskbar context menu and task hover/click preview stacking. It does not redesign popup geometry, hover delays, thumbnails, other flyouts, taskbar topmost behavior, or global window activation policy.
