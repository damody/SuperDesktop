## ADDED Requirements

### Requirement: 設定必須使用版本化 schema 與原子寫入
系統 SHALL 保存 schema version、execution-mode preference、工作列列數、釘選、桌布、桌面位置、螢幕 mapping、SuperExplorer path、theme 與 accessibility preference，並以暫存檔寫入、flush、驗證及原子取代方式更新正式設定。保存的 execution-mode preference MUST NOT 讓沒有明確 Shell mode 參數的啟動自動接管 Shell。

#### Scenario: 設定寫入成功
- **WHEN** 使用者變更工作列列數、釘選、桌布或桌面位置
- **THEN** 新設定通過 schema 驗證後原子取代舊檔，重新啟動可讀回相同有效值

#### Scenario: 保存 Shell 模式偏好後一般啟動
- **WHEN** execution-mode preference 記錄先前使用 Shell 模式，但使用者下一次未明確指定 Shell mode
- **THEN** 系統仍以預覽模式啟動，可提示偏好但不得自動接管 Shell

#### Scenario: 寫入期間程序終止
- **WHEN** 程序在正式檔案原子取代前異常終止
- **THEN** 下一次啟動讀到完整舊設定或完整新設定，不會讀到部分寫入內容

### Requirement: 設定錯誤必須局部修正或隔離
系統 SHALL 對可安全獨立回復的無效欄位只套用該欄預設值；對無法解析或結構無效的設定檔建立 timestamped quarantine 並使用安全預設設定。

#### Scenario: 工作列列數超出範圍
- **WHEN** 設定檔其他內容有效但 row count 不在一至三之間
- **THEN** 系統只將 row count 回復為二，保留其餘有效設定並記錄修正

#### Scenario: 設定 JSON 損毀
- **WHEN** 正式設定檔無法解析
- **THEN** 系統不覆寫原始損毀內容，而是將其移至唯一 quarantine 路徑、使用安全預設值並保持 Shell 可啟動

### Requirement: 非同步結果必須受 request 與 generation 約束
系統 SHALL 為非同步平台工作標記 request ID 與 generation，且 MUST NOT 讓 stale generation 修改目前 shell state。

#### Scenario: 過期視窗列舉晚到
- **WHEN** generation N 的 `EnumWindows` 結果在 generation N+1 已套用後到達
- **THEN** 系統拒絕 N 的結果並保留 N+1 的 task model

#### Scenario: 取消後仍收到結果
- **WHEN** request 已取消但 worker 仍回傳結果
- **THEN** 系統記錄 late result 並保持使用者可見狀態不變

### Requirement: 有界事件 queue 必須明確處理 overflow
系統 SHALL 對視窗與桌面事件使用有界 queue；發生 overflow 時必須送出可觀測 overflow event 並安排權威對帳。

#### Scenario: 視窗事件風暴造成 overflow
- **WHEN** window event queue 超過容量
- **THEN** 系統合併可合併事件、標記 task state 需要 reconciliation，並以 `EnumWindows` 恢復權威狀態

#### Scenario: 桌面事件風暴造成 overflow
- **WHEN** desktop watcher queue 超過容量
- **THEN** 系統安排完整 namespace refresh，且不把未執行的增量事件視為成功套用

### Requirement: 診斷必須預設保護敏感資料
系統 MUST NOT 在一般診斷中記錄檔案內容、credential、clipboard 或完整使用者路徑；只有明確啟用的本機 debug 模式可記錄經範圍限制的完整路徑。

#### Scenario: SuperExplorer 使用者路徑啟動失敗
- **WHEN** 一個位於使用者 profile 下的資料夾啟動失敗
- **THEN** 一般診斷只記錄錯誤類型、correlation ID 與已 redact 路徑，不包含完整 profile 路徑

### Requirement: 測試與復原寫入必須限制於已驗證目標
系統 SHALL 在測試或清理前解析明確 fixture/recovery target，並 MUST NOT 對 workspace root、使用者 profile、磁碟根目錄或未解析路徑執行遞迴刪除或移動。

#### Scenario: Fixture 路徑逃逸
- **WHEN** 測試 target 經 canonicalization 或 reparse point 解析後離開受控 fixture root
- **THEN** 系統拒絕 destructive step、留下診斷與 evidence，且不修改逸出目標
