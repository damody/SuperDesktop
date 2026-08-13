## Why

SuperDesktop 必須用 SuperExplorer 開啟資料夾，但目前唯一穩定合約是新程序加 `EXPLORER_INITIAL_PATH`；需要把解析、deadline、取消、錯誤提示與 source boundary 獨立驗證。

## What Changes

- 實作三段式 executable resolution 與 existing absolute directory validation。
- 實作 child-only environment、5 秒 admission deadline、cancellation 與 exactly-once terminal。
- 實作固定「SuperExplorer」入口的預設啟動與 GPUI 修復提示。

## Capabilities

### New Capabilities

- `superexplorer-process-bridge`：既有程序啟動合約、路徑安全、deadline/cancellation、終端事件與修復 UI。

### Modified Capabilities

無。

## Impact

依賴 shell core；不得修改 `D:\SuperExplorer` 或以 path dependency 連結它，也不得把無 initial-path 啟動標成「本機」。
