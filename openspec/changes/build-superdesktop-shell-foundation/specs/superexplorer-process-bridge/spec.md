## ADDED Requirements

### Requirement: SuperExplorer 執行檔解析必須具備確定順序
系統 SHALL 依序檢查使用者設定的絕對執行檔、`D:\SuperExplorer\target\release\SuperExplorer.exe`、與 SuperDesktop 相鄰的 `SuperExplorer.exe`，並只接受存在的一般檔案。

#### Scenario: 使用者設定有效
- **WHEN** 使用者設定指向存在的絕對執行檔
- **THEN** 系統選用該路徑且不繼續使用較低優先序候選

#### Scenario: 所有候選無效
- **WHEN** 沒有候選通過絕對路徑、存在及一般檔案驗證
- **THEN** 系統回報可復原的 executable-unavailable 結果，不啟動 Windows Explorer 作為替代

### Requirement: 檔案系統資料夾必須使用既有環境變數合約啟動
系統 SHALL 只接受存在的絕對資料夾，建立新的 SuperExplorer 程序，將 child environment 的 `EXPLORER_INITIAL_PATH` 設為該資料夾，且不得傳入目前不支援的命令列參數。

#### Scenario: 啟動有效資料夾
- **WHEN** 使用者啟動存在的絕對檔案系統資料夾
- **THEN** child process 收到精確的 `EXPLORER_INITIAL_PATH`，父程序環境不被永久修改，且命令列不含額外參數

#### Scenario: 資料夾為相對路徑或不存在
- **WHEN** launch request 包含相對路徑、一般檔案或不存在的資料夾
- **THEN** 系統在 spawn 前拒絕請求並回報 invalid-initial-directory

### Requirement: SuperExplorer 入口必須使用應用程式預設位置
系統 SHALL 在啟動「SuperExplorer」入口時建立 SuperExplorer 程序，且 child environment 不得設定 `EXPLORER_INITIAL_PATH`。系統 MUST NOT 把此結果標示為保證導覽至「本機」。

#### Scenario: 啟動 SuperExplorer 入口
- **WHEN** 使用者從桌面或工作列啟動「SuperExplorer」
- **THEN** 系統啟動 SuperExplorer 並讓它選擇自己的預設位置

### Requirement: 每個啟動請求必須恰好產生一個終端事件
系統 SHALL 為每個 launch request 建立 correlation ID，並從 dispatcher 接受 request 的 monotonic timestamp 起算，在 5 秒內於成功、驗證失敗、spawn failure、取消或逾時中只接受第一個 terminal event。`explorer-bridge` SHALL 擁有 cancellation；若 child 已建立，取消不強制終止外部 SuperExplorer，但 MUST 關閉本端 process/thread handles。

#### Scenario: Spawn 成功
- **WHEN** 作業系統成功建立 SuperExplorer child process
- **THEN** 對應 correlation ID 產生一個 launched terminal event，後續重複 callback 被忽略並記錄

#### Scenario: Spawn 失敗與延遲 callback 競態
- **WHEN** spawn 回報失敗且較晚又到達同一 correlation ID 的 callback
- **THEN** 第一個 terminal event 保持權威，UI 不重複顯示或轉為相反結果

#### Scenario: 取消與成功競態
- **WHEN** shutdown cancellation 與 spawn success 對同一 correlation ID 競態
- **THEN** 系統只接受第一個 terminal；若 child 已建立則不強制終止它，但關閉所有本端 handles 並忽略較晚結果

#### Scenario: Launch admission 逾時
- **WHEN** request 在 5 秒內沒有其他 terminal event
- **THEN** 系統產生唯一 `timed-out` terminal、釋放本端資源，並將較晚 callback 只記為 late diagnostic

### Requirement: 啟動失敗必須可由 GPUI 修復
系統 SHALL 在 executable 缺失、驗證失敗或 spawn failure 時保持桌面與工作列可操作，顯示 GPUI 錯誤與設定執行檔的修復動作，且預設診斷不得洩漏完整使用者路徑。

#### Scenario: 執行檔在使用期間被移除
- **WHEN** 已解析的 SuperExplorer 執行檔在 spawn 前被移除
- **THEN** 系統顯示可修復失敗、不 fallback 至 Explorer，並允許使用者更新設定後重試
