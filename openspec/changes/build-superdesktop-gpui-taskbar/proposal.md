## Why

使用者要求依目前 ExplorerPatcher UI 重現高密度雙列工作列；視窗追蹤、AppBar 與 task interaction 是獨立高風險工作流，需要與桌面、橋接和 guardian 分離。

## What Changes

- 建立一至三列、預設雙列的 per-monitor GPUI taskbar 與 AppBar。
- 實作 window eligibility、Shell Hook/EnumWindows reconciliation、ApplicationId 群組與穩定排序。
- 實作 task 切換、固定 SuperExplorer 入口、Start host、時鐘與核心系統狀態。

## Capabilities

### New Capabilities

- `gpui-taskbar-window-management`：M0 工作列、AppBar、視窗追蹤/群組/切換、固定入口與系統狀態。

### Modified Capabilities

無。

## Impact

依賴 platform spike 與 shell core；擁有 `taskbar-ui` 與 taskbar platform adapters，不實作完整 tray/Start menu 或 Shell takeover。
