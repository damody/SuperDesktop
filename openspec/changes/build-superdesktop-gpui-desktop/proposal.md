## Why

M0 需要可獨立驗證的全 GPUI 桌面表面，且桌面 Shell identity、選取、位置持久化與一般檔案關聯啟動不應與工作列或 Shell takeover 混在同一 change。

## What Changes

- 建立 per-monitor GPUI desktop host 與桌布模式。
- 合併 User/Public Desktop，解析 stable Shell identity、圖示與 watcher reconciliation。
- 實作選取、鍵盤、rubber-band、圖示重新定位、持久化與 Windows association 啟動。

## Capabilities

### New Capabilities

- `gpui-desktop-surface`：M0 桌面 surface、桌布、Shell items、互動、持久化、watcher 與一般檔案啟動。

### Modified Capabilities

無。

## Impact

依賴 platform spike 與 shell core；擁有 `desktop-ui` 與 desktop 相關 `platform-win` adapter，不實作 taskbar、SuperExplorer bridge 或 Shell takeover。
