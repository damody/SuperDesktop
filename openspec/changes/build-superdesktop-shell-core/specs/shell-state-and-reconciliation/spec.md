## ADDED Requirements

### Requirement: Shell Core 必須是唯一狀態權威
系統 SHALL 以不含 GPUI/Win32/COM type 的 immutable snapshot 管理 monitor、desktop、taskbar、selection、focus、application、settings revision 與 recovery phase。

#### Scenario: UI 或平台事件
- **WHEN** UI command 或 platform event 到達
- **THEN** 只有 reducer 可產生新 snapshot 與 typed effect

#### Scenario: 非法轉移
- **WHEN** event 與目前 lifecycle/state 不相容
- **THEN** reducer 回傳 typed rejection 且原 snapshot 不變

### Requirement: Identity 與 Terminal 必須穩定且 exactly-once
系統 SHALL 提供 MonitorId、ShellItemId、WindowId、ApplicationId、RequestId、Generation、CorrelationId，並只接受 correlation 的第一個 terminal。

#### Scenario: 重複 terminal
- **WHEN** 同一 correlation 的第二個 terminal 到達
- **THEN** 系統忽略狀態變更並記錄 duplicate/late diagnostic

### Requirement: Stale 與取消結果不得修改目前狀態
系統 SHALL 拒絕 generation 不符或已取消 request 的結果。

#### Scenario: 舊 generation 晚到
- **WHEN** generation N 結果在 N+1 已套用後到達
- **THEN** N 結果不修改 snapshot

#### Scenario: 取消後結果
- **WHEN** request 取消後 worker 回傳結果
- **THEN** 結果成為 late diagnostic，使用者狀態不變

### Requirement: Queue Overflow 必須啟動權威對帳
系統 SHALL 使用 bounded desktop/window queues、stable-identity coalescing 與顯式 overflow event。

#### Scenario: Window queue overflow
- **WHEN** window queue 超過容量
- **THEN** core 要求 `EnumWindows` authoritative snapshot 並在新 generation 收斂

#### Scenario: Desktop queue overflow
- **WHEN** desktop queue 超過容量
- **THEN** core 要求完整 namespace refresh，不把遺失增量視為已套用

### Requirement: Event 序列必須具確定性
系統 SHALL 在 duplicate、missing、reordered、stale 與 overflow 序列下產生可重現 snapshot。

#### Scenario: 重播相同序列
- **WHEN** 相同初始 snapshot 與 event sequence 被重播
- **THEN** 最終 snapshot、effects 與 terminal disposition 完全相同
