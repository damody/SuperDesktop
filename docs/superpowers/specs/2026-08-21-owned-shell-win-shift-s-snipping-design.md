# Owned-shell Win+Shift+S Snipping Tool design

## Goal

When SuperDesktop owns the Windows shell, pressing `Win+Shift+S` shall open the built-in Windows Snipping Tool image-capture overlay, matching native Windows behavior. When Explorer remains the shell, SuperDesktop shall not intercept the chord and Windows retains its normal handler.

Microsoft documents `Win+Shift+S` as the shortcut that opens the Snipping Tool overlay for an image snapshot. This change targets that overlay behavior, not merely the Snipping Tool editor window.

## Hotkey routing

The existing shell-scoped low-level keyboard hook gains an `OpenScreenSnip` action. The reducer maps `S` with Windows and Shift down, while preserving the existing `Win+S` search action. Control or Alt modifiers keep the chord unsupported and pass it to the next hook.

The active-key fence consumes repeats and the matching key-up exactly as for the other Windows shell shortcuts. The action uses the next unused request bit and participates in the existing bounded atomic queue. No work, allocation, URI launch, or UI mutation occurs inside the hook callback.

## Native activation

The GPUI foreground refresh path receives `OpenScreenSnip` and invokes a platform-owned helper. A live Windows 11 observation of the real Explorer-handled chord showed Snipping Tool launched with AUMID `Microsoft.ScreenSketch_8wekyb3d8bbwe!App` and argument `ms-screenclip:///?source=HotKey`. Explorer-free testing proved that ShellExecute did not create the overlay and `ActivateForProtocol` was rejected because the package does not expose that URI as a Windows.Protocol extension. The helper therefore uses Windows' documented `IApplicationActivationManager::ActivateApplication` with the observed fixed AUMID, fixed argument, and `AO_NONE`. `CLSCTX_LOCAL_SERVER` owns activation-argument lifetime without Explorer or a retained process handle.

Microsoft's newer `ms-screenclip://capture/...` app-integration protocol is not used: it requires a packaged caller plus a registered redirect URI and is intended to return captured media to an app. SuperDesktop is matching the OS hotkey and does not request captured content.

This is preferred over ShellExecute, a hard-coded `SnippingTool.exe` path, or undocumented executable switches. Re-injecting the same key chord is rejected because it can recurse through the low-level hook and can double-trigger when another shell component is present.

## Mode boundary

`ShellHotkeys::start` remains guarded by the existing owned-shell flag. Therefore:

- owned shell / Explorer absent: SuperDesktop consumes the chord and launches `ms-screenclip:///?source=HotKey`;
- Explorer-compatible preview mode: SuperDesktop installs no shell hotkey hook and Windows handles the chord unchanged;
- startup or hook failure: existing console diagnostics remain authoritative and no partial second hook is installed.

## Error handling and observability

The fixed URI helper returns a typed `Result`. A rejected or unregistered protocol produces `SuperDesktop error [shell-hotkey:screen-snip]: ...` on the console. It does not panic, unwrap, launch Explorer, search `PATH`, or fall back to a third-party screenshot program.

Successful admission records `shell-hotkey:screen-snip-requested` and `shell-hotkey:screen-snip-accepted` trace events. The first proves reducer-to-runtime delivery; the second proves Windows accepted the native protocol activation.

## Verification

- Reducer tests distinguish `Win+S`, `Win+Shift+S`, repeats, key-up, and unsupported Control/Alt variants.
- Source-contract tests prove fixed AUMID/argument activation, forbid `ShellExecuteExW` and `ActivateForProtocol`, forbid `explorer.exe` and executable-path lookup, and cover the complete action-code round trip.
- A headful Explorer-free UTIT sends the physical chord to the release candidate, observes both trace events and the built-in screen clipping surface/process, sends Escape to dismiss it, and verifies SuperDesktop remains alive without panic/error signatures.
- The focused headful case passes twice from clean launches.
- Formatting, workspace tests, Clippy with warnings denied, release build, installer build, embedded-binary hash comparison, and parent gitlink integration remain blocking gates.

## Non-goals

This change does not implement a custom screenshot UI, change `Win+Shift+R`, remap Print Screen, modify Snipping Tool settings, capture user screen contents as test evidence, or replace Windows clipboard/notification behavior after a snip.

## Rollback

Reverting the action, fixed URI helper, runtime match arm, and focused tests restores the previous routing. There is no settings or persistent-data migration.
