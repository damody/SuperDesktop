## Why

The owned taskbar context menu currently survives loss of window activation, and delayed task previews can be covered by ordinary application windows. These defects break expected Windows taskbar popup behavior and make the preview unavailable precisely when another window occupies the same screen area.

## What Changes

- Dismiss the owned taskbar context menu immediately when its popup window loses activation, without treating focus movement inside the menu as dismissal.
- Promote task hover and click previews into the Windows topmost band without activating passive hover previews.
- Reject invalid, retired, or foreign popup HWNDs and close a preview when its required z-order cannot be established.
- Add automated and headful regression evidence for focus-loss dismissal, topmost stacking, and foreground-focus preservation.

## Capabilities

### New Capabilities

- `taskbar-popup-lifecycle`: Defines activation-loss dismissal, passive topmost preview stacking, HWND ownership validation, and observable regression evidence for owned taskbar popups.

### Modified Capabilities

None. The repository has no archived base capability for these popup lifecycle requirements; related completed changes remain historical inputs.

## Impact

Affected areas are `taskbar-ui` context-menu lifecycle state, `superdesktop-app` preview composition, the Windows taskbar platform adapter, focused unit/source-contract tests, and headful UITest evidence. No public extension ABI, settings schema, dependency, installer contract, or Explorer integration changes.
