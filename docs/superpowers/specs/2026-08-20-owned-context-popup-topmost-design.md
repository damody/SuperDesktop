# Owned Context Popup Topmost Design

## Goal

Every independent SuperDesktop right-click popup must appear above ordinary application windows while preserving Windows context-menu focus behavior.

## Scope

Apply the existing owned-popup Win32 z-order adapter to these separately owned popup windows:

- task application Jump Lists;
- taskbar background context menus;
- input-method context menus;
- volume context menus;
- future system-control context menus using the same creation route.

Desktop item/background context menus rendered inside the desktop surface are not separate native popup windows and therefore do not require a z-order promotion.

## Behavior

Immediately after GPUI creates a right-click popup window, SuperDesktop obtains its owned HWND and calls the existing topmost adapter. The adapter uses `HWND_TOPMOST` with `SWP_NOACTIVATE`, so the popup rises above normal windows without activating an unrelated owner or creating a focus loop.

Promotion is performed exactly once per popup creation. SuperDesktop must not poll, repeatedly force z-order, or leave a background worker running after dismissal.

If HWND extraction or topmost promotion fails, SuperDesktop removes the popup, clears its slot, writes the error to console and action trace, and does not claim that the menu opened. Successful promotion is traced separately for each popup kind.

The popup retains its existing focus and dismissal contract. Losing focus dismisses the menu through the view's existing activation subscription; topmost status must not make a dismissed window remain visible.

## Architecture

Add a small application-layer helper that accepts an owned GPUI `Window`, a trace context, and the current callback context. It extracts the HWND, invokes `promote_owned_popup_topmost`, emits success/failure trace, and removes the window on failure. Individual popup creation closures call this helper before constructing their views.

Using one helper prevents the Jump List, taskbar background menu, and system-control menu from drifting into different error policies. The Win32 implementation remains confined to `platform-win`; UI crates remain free of native handles.

## Alternatives

1. **Shared one-time Win32 promotion — selected.** Reuses an already tested boundary and preserves non-activation.
2. **Rely on GPUI `WindowOptions`.** Rejected because the current popup options do not establish a verifiable native topmost contract across all owned routes.
3. **Periodic z-order repair.** Rejected because it can steal ordering after dismissal, wastes work, and complicates shutdown.

## Verification

- Unit/source tests require every independent right-click popup route to call the shared promotion helper before view construction.
- Platform tests retain invalid/foreign/retired HWND fail-closed coverage.
- Headful UTIT opens each supported right-click menu with physical pointer input and verifies the native topmost extended style while the menu is visible.
- UTIT dismisses each menu by moving focus to another owned or fixture window and verifies the popup closes.
- Formatting, affected package tests, Clippy with warnings denied, complete release build, and combined installer packaging remain blocking gates.
