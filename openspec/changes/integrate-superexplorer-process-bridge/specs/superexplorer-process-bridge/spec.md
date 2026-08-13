## ADDED Requirements

### Requirement: Executable Resolution 必須確定且安全
系統 SHALL 依 user absolute setting、`D:\SuperExplorer\target\release\SuperExplorer.exe`、adjacent executable 排序，且只接受 existing regular file。

#### Scenario: 第一候選有效
- **WHEN** user setting 有效
- **THEN** 選用它且不使用後續候選

#### Scenario: 全部無效
- **WHEN** 所有候選失敗
- **THEN** 回報 executable-unavailable 且不 fallback Explorer

### Requirement: Folder Launch 必須使用 Child-only Environment
系統 SHALL 只接受 existing absolute directory，並只在 child environment 設 `EXPLORER_INITIAL_PATH`，不得加入 unsupported CLI。

#### Scenario: 有效資料夾
- **WHEN** launch request 指向 existing absolute dir
- **THEN** child 收到精確 env，parent env 不變

#### Scenario: 無效路徑
- **WHEN** path 相對、不存在或為一般檔案
- **THEN** spawn 前回報 invalid-initial-directory

### Requirement: Default Entry 必須 Truthful
系統 SHALL 以「SuperExplorer」名稱啟動無 `EXPLORER_INITIAL_PATH` 的程序，且 MUST NOT 保證或標示「本機」。

#### Scenario: 固定入口
- **WHEN** taskbar/desktop 發出 default launch
- **THEN** child 未收到 initial-path 並自行選擇預設位置

### Requirement: Launch 必須在 5 秒內 Exactly-once 結束 Admission
系統 SHALL 從 dispatcher 接受 request 的 monotonic T0 起 5 秒內產生 launched/validation-failed/spawn-failed/cancelled/timed-out 第一 terminal。

#### Scenario: Timeout 與 late success
- **WHEN** 5 秒到期後才到達 success callback
- **THEN** terminal 保持 timed-out，late callback 只記診斷

#### Scenario: Cancel 與 success 競態
- **WHEN** cancellation 與 child creation 競態
- **THEN** 只接受第一 terminal；已建立 child 不被強制終止，本端 handles 全部關閉

### Requirement: Failure 必須可修復且保護隱私
系統 SHALL 保持 desktop/taskbar responsive，顯示繁中/英文 keyboard/UIA repair prompt，普通診斷 redact 完整 profile path。

#### Scenario: Executable 被移除
- **WHEN** resolved executable 在 spawn 前消失
- **THEN** 顯示設定路徑/重試 action，不 fallback Explorer

### Requirement: Integration 不得修改 SuperExplorer Repository
系統 MUST NOT 寫入、commit 或 path-link `D:\SuperExplorer`。

#### Scenario: 前後來源稽核
- **WHEN** bridge change 完成
- **THEN** SuperExplorer tracked/untracked source state 與基準相比沒有由本 change 造成的變更
