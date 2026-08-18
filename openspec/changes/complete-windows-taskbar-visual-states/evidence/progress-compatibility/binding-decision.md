# `ITaskbarList3` compatibility binding decision

Windows 11 build 26200 registers `CLSID_TaskbarList` (`{56FDF344-FD6D-11d0-958A-006097C9A090}`) as the in-process `explorerframe.dll` “Task Bar Communication” class. Ordinary applications create that documented COM class and call `ITaskbarList3::SetProgressState` and `SetProgressValue`; GPUI rendering cannot observe those calls directly.

SuperDesktop therefore supplies `taskbar-state-host.exe`, an isolated local COM server implementing `ITaskbarList4` and its inherited interfaces. Controlled Shell enable registers it only under the current user's CLSID `LocalServer32`; preview mode does not register it. Shell restore removes only a key carrying the SuperDesktop ownership marker. The server validates live same-session HWND/PID identity and publishes bounded generation-tagged snapshots to SuperDesktop.

The committed `taskbar-progress-fixture` uses the ordinary documented CLSID/interface calls. It succeeds against Explorer with `CLSCTX_ALL`, and the process integration test forces `CLSCTX_LOCAL_SERVER`, observes `normal 42/100` through the isolated host, then performs clean shutdown.
