## Why

GPUI native HWND、AppBar、Shell Hook、ExplorerPatcher Start host、per-monitor DPI 與 guardian lease 是整個方案的高風險前提，必須在 production 實作前用最小 spike 取得 go/stop 證據。

## What Changes

- 在凍結 Windows 11 build 26200.9168 + ExplorerPatcher 26100.8457.70.3 profile 驗證必要 Windows/GPUI 能力。
- 驗證 FFI no-unwind、resource cleanup、Safe Mode/unsupported session fail-closed。
- 產生固定 source/binary/profile hashes 與 go/stop disposition。

## Capabilities

### New Capabilities

- `windows-gpui-shell-capability`：GPUI/Win32 bridge、AppBar、Shell Hook、Start host、DPI/topology、guardian lease 與平台安全能力 gate。

### Modified Capabilities

無。

## Impact

依賴 `bootstrap-superdesktop-workspace`；只建立 spike/harness 與證據，不實作完整桌面/工作列或切換 Explorer 表面。
