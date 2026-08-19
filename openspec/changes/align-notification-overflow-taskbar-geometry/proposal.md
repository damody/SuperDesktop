## Why

Hidden notification icons reserve the taskbar twice in replacement-shell mode, leaving an extra row-sized gap. The current shell-parity suite also does not force or geometrically admit the overflow panel.

## What Changes

- Make overflow placement explicitly preview/shell aware.
- Preserve the Windows 344 DIP, six-column, 48 DIP-cell panel across DPI/rows/origins.
- Force 20 documented NotifyIcons in an Explorer-free UTIT case and measure the owned panel.
- Run full quality, traceability, release and no-launch package gates without archiving.

## Capabilities

### New Capabilities

- `notification-overflow-taskbar-geometry`: Defines owned overflow dimensions, taskbar anchoring and Explorer-free runtime admission.

### Modified Capabilities

None.

## Impact

Changes `superdesktop-app`, NotifyIcon capture/catalog, evidence and packages. No Explorer/system-tray call, privilege, persistence, protocol or undocumented API is added.
