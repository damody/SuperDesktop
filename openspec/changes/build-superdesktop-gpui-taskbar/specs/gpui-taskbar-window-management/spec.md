## ADDED Requirements

### Requirement: Taskbar 必須是 Per-monitor GPUI AppBar
系統 SHALL 建立 bottom GPUI taskbar；Shell mode 使用 AppBar，preview 不修改 work area。

#### Scenario: Preview
- **WHEN** taskbar 以 preview 顯示
- **THEN** Explorer taskbar/work area 保持不變

#### Scenario: Virtual dual monitor
- **WHEN** 可控 topology 提供兩個 monitor
- **THEN** 每個目標 monitor 有正確 bounds/work-area model

### Requirement: Layout 必須支援一至三列並預設雙列
系統 SHALL 依 reference profile 顯示 compact icon/title task，running underline 與 active background。

#### Scenario: 無設定首次啟動
- **WHEN** rows 尚未保存
- **THEN** 使用兩列且 title 省略/hit target 可操作

#### Scenario: 無效 rows
- **WHEN** rows 不在一至三
- **THEN** 只將 rows 回復二

### Requirement: Window Tracker 必須權威且可復原
系統 SHALL 以 Shell Hook 增量、`EnumWindows` 對帳並排除 invisible/tool/cloaked/owned-transient。

#### Scenario: Hook 遺失
- **WHEN** helper window 存在但 event 遺失
- **THEN** reconciliation 補入/移除且無 duplicate

#### Scenario: 排除窗口
- **WHEN** helper 為任一 excluded 類別
- **THEN** 不顯示一般 task button

### Requirement: Group 與順序必須穩定
系統 SHALL 分離 WindowId/ApplicationId，並只在 membership/pin/user reorder 時改變 group order。

#### Scenario: Title/icon churn
- **WHEN** title/icon/foreground/minimize 快速改變
- **THEN**內容更新但 group order 不變

### Requirement: Task Click 必須符合基本 Windows 語意
系統 SHALL 對 active task 最小化、對 minimized/inactive task 還原啟用、對無窗口 pinned app 啟動。

#### Scenario: Active click
- **WHEN** 使用者點擊 foreground task
- **THEN** 發出一次 minimize effect

#### Scenario: Minimized click
- **WHEN** 使用者點擊 minimized task
- **THEN** 還原並嘗試啟用，失敗時顯示真實狀態

### Requirement: 工作列必須固定顯示 SuperExplorer 入口
系統 SHALL 在主要 taskbar 顯示 pointer/keyboard/UIA 可操作的「SuperExplorer」入口，且無窗口時仍存在。

#### Scenario: 啟動固定入口
- **WHEN** 使用者操作入口
- **THEN** 發送一次無 initial-path 的 bridge command，不標示為「本機」

#### Scenario: Bridge failure
- **WHEN** bridge 回報 executable/spawn failure
- **THEN** 入口保持可操作並顯示規範 repair prompt

### Requirement: Start、Time 與核心狀態必須 Truthful
系統 SHALL probe/invoke reference Start host，並顯示 time/date、network、volume/mute、input language、可取得時的 power/battery、notification count。

#### Scenario: Provider unavailable
- **WHEN** 個別非必要 provider unavailable
- **THEN** 只有該狀態顯示 unavailable，其餘功能保持可操作

#### Scenario: Reference Start unavailable
- **WHEN** frozen profile 的 Start host probe 失敗
- **THEN** lifecycle gate 不得允許 Shell takeover

#### Scenario: Reference Preview 與 Shell mode Start
- **WHEN** 在凍結 ExplorerPatcher profile 分別以 preview 與 Shell mode 啟動 Start
- **THEN** 兩種模式都必須保存獨立 probe/invocation 結果；Shell mode probe 失敗 SHALL 阻止 takeover
