## Why

只有在桌面、工作列與 SuperExplorer 都可用後，才能安全地接管目前工作階段；owner 競態、FFI panic 或主程序 crash 若無 guardian 會留下錯誤 work area 或無 Shell 狀態。

## What Changes

- 建立 preview/shell mode composition、session-scoped single-owner lease 與六階段交易式 takeover。
- 建立不可偽造 guardian lease、安全 Explorer target 與 10-run/10-second crash recovery gate。
- 建立 FFI no-unwind、failpoint、normal shutdown、non-owner rejection 與 registry-mutation guard。

## Capabilities

### New Capabilities

- `windows-shell-lifecycle`：預覽安全、單一 owner、交易式接管、guardian、安全復原與有序關閉。

### Modified Capabilities

無。

## Impact

依賴 desktop、taskbar、bridge 完成；會影響目前互動 session，但 M0 不修改登入 Shell 登錄值或安裝自動啟動。
