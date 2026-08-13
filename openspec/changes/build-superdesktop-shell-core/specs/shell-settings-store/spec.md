## ADDED Requirements

### Requirement: Settings v1 必須完整且安全
系統 SHALL 保存 schema version、execution preference、taskbar rows/pins、wallpaper、desktop positions、monitor mapping、SuperExplorer path、theme 與 accessibility preferences。

#### Scenario: Round trip
- **WHEN** 有效 settings v1 被保存並重新讀取
- **THEN** 所有欄位保持相同且 revision 正確增加

#### Scenario: Shell preference 無明確 opt-in
- **WHEN** preference 記錄 Shell mode 但下次啟動未帶明確 `--shell`
- **THEN** app 仍選擇 preview mode

### Requirement: 寫入必須原子且可從中斷復原
系統 SHALL 以 temp-write、flush、validate、atomic replace 更新正式檔案。

#### Scenario: Replace 前 crash
- **WHEN** 程序在 atomic replace 前終止
- **THEN** 下次只讀到完整舊檔或完整新檔

### Requirement: 無效設定必須局部 fallback 或 quarantine
系統 SHALL 對獨立欄位錯誤只回復該欄；結構/解析損毀則保留 timestamped quarantine 並使用安全 defaults。

#### Scenario: Row count 無效
- **WHEN** rows 不在一至三而其他欄位有效
- **THEN** rows 回復二且其他欄位保留

#### Scenario: JSON 損毀
- **WHEN** 正式檔無法解析
- **THEN** 原內容移至唯一 quarantine，Shell 使用 defaults 啟動

### Requirement: 測試寫入必須限制在 Fixture Root
系統 MUST NOT 對 workspace/profile/drive root 或 canonicalization/reparse 後逃逸的 target 執行 destructive operation。

#### Scenario: Reparse escape
- **WHEN** target 解析後離開 fixture root
- **THEN** 操作被拒絕且逸出位置不變
