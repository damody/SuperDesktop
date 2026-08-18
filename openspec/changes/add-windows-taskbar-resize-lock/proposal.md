## Why

SuperDesktop can store one, two, or three taskbar rows, but users cannot resize the surface directly or lock it, Preview/Shell placement is not explicitly gated, and multi-row rendering draws unwanted horizontal separators. The owned taskbar must reproduce the classic Windows interaction without relying on Explorer.

## What Changes

- Persist an owned taskbar locked state with backward-compatible defaults.
- Add a localized checked “Lock the taskbar” item to the owned right-click menu and owned settings.
- Enable top-edge native resizing only while unlocked, quantize to one through three rows, and atomically persist the result.
- Keep Preview above Explorer’s current work area and place Shell at the physical monitor bottom.
- Synchronize Shell AppBar reservation whenever row count changes.
- Remove horizontal separators between taskbar rows while preserving the outer top border and task indicators.
- Add automated, headful, Explorer-free, accessibility, traceability, release, and installer evidence.

## Capabilities

### New Capabilities

- `windows-taskbar-resize-lock`: Defines placement modes, taskbar locking, native row resizing, AppBar synchronization, continuous multi-row chrome, and Explorer-free safety.

### Modified Capabilities

None.

## Impact

Changes taskbar settings serialization/model/views, taskbar context callbacks, taskbar GPUI composition, the Windows taskbar style adapter, AppBar lease access, production refresh logic, tests, evidence, and packaged SuperDesktop. Existing settings files remain readable and existing row values remain compatible.
