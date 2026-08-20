## Why

When SuperDesktop replaces Explorer as the shell, the native `Win+Shift+S` path is no longer guaranteed to reach Windows' Snipping Tool overlay. The owned shell must preserve this standard Windows shortcut without implementing a substitute capture UI or depending on Explorer.

## What Changes

- Add a distinct, repeat-safe `Win+Shift+S` action to the owned-shell keyboard hook while preserving `Win+S` search.
- Activate the built-in Windows screen-clipping overlay through the exact observed AUMID/argument and a bounded verified inbox Explorer broker required by Windows 11 while the overlay is open.
- Keep Explorer-compatible mode delegated entirely to Windows and prevent duplicate activation.
- Print protocol activation failures to the console without panic, fallback executable lookup, or third-party launch.
- Add reducer, source-contract, headful Explorer-free, release, installer, and provenance gates.

## Capabilities

### New Capabilities

- `owned-shell-screen-snipping-shortcut`: Native-parity `Win+Shift+S` routing and built-in Snipping Tool overlay activation for SuperDesktop's owned shell.

### Modified Capabilities

None.

## Impact

The change affects `platform-win` shell hotkey routing, packaged-app activation, and verified Explorer lifecycle, `superdesktop-app` action dispatch and diagnostics, the SuperDesktop UTIT catalog/scripts, and the existing Windows release/installer evidence pipeline. It adds no setting, migration, privilege, user-controlled URI, custom screenshot implementation, or public API break.
