## Why

SuperDesktop currently exposes automatic taskbar hiding as unavailable, leaving a visible Windows behavior gap in both Preview and Explorer-free Shell operation. The owned taskbar can provide this behavior without Explorer or undocumented AppBar protocols because SuperDesktop already owns its settings, HWND geometry, popups, focus, and attention state.

## What Changes

- Add a backward-compatible, persisted automatic-hide setting and an enabled Windows-style behavior row.
- Add an owned visibility state machine with immediate reveal, a two-pixel screen-edge target, and a 500 ms delayed hide.
- Keep the taskbar visible while owned menus, Start, settings, flyouts, focus, resize, or attention require it.
- Position only validated current-process taskbar HWNDs at exact Preview and Shell visible/hidden endpoints.
- Avoid Shell work-area reservation while auto-hide is active and restore ordinary placement when it is disabled.
- Add Explorer-present and Explorer-free headful evidence, lifecycle recovery, DPI, accessibility, packaging, and traceability gates.

## Capabilities

### New Capabilities

- `owned-taskbar-auto-hide`: Defines persisted controls, visibility state, reveal/hide timing, owned-HWND placement, lifecycle recovery, accessibility, and Explorer-free behavior.

### Modified Capabilities

None.

## Impact

- Affects `settings-store`, `taskbar-ui`, `platform-win`, and `superdesktop-app`.
- Adds no external dependency and invokes no Explorer, system Start, tray UI, or undocumented Shell protocol.
- Changes Shell work-area behavior only when explicit taskbar auto-hide is enabled; ordinary launch remains Preview and non-mutating.
