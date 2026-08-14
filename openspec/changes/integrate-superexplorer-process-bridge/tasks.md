## 1. Executable Resolution 與 Launch Contract

### 1.1 實作安全且確定的 Executable Resolver

**目的：** 依固定優先序解析 SuperExplorer 並拒絕可疑 target。
**輸入：** Settings path、bundled path、developer fallback policy。
**產出：** Resolver、file identity validator、negative fixtures。
**依賴：** `build-superdesktop-shell-core` contract hash。
**Owner／Wave：** Explorer bridge owner／Wave 4C。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`、`G-SAFETY`；`evidence/artifacts/1.1/`。
**完成門檻：** 優先序 deterministic；missing/non-file/reparse/untrusted target 被 typed validation 拒絕。

- [x] 1.1.1 實作 settings、bundled、developer fallback 的 resolver 優先序。
- [x] 1.1.2 實作 canonical path、regular-file 與 executable identity validation。
- [x] 1.1.3 加入 missing、directory、reparse、PATH/CWD substitution negative tests。
- [x] 1.1.4 保存 resolver decision trace 且 redact 使用者敏感路徑。

### 1.2 定義 Default 與 Folder Launch 語意

**目的：** 在無起始路徑時 truthful 顯示 SuperExplorer，有路徑時只用 child environment 傳入。
**輸入：** Resolver result、core launch request。
**產出：** Command builder、child environment policy、tests。
**依賴：** 1.1。
**Owner／Wave：** Explorer bridge owner／Wave 4C。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`、`G-SAFETY`；`evidence/artifacts/1.2/`。
**完成門檻：** Default 不宣稱 This PC；folder path round-trip；parent environment 完全不變。

- [x] 1.2.1 實作無 `EXPLORER_INITIAL_PATH` 的 default SuperExplorer command。
- [x] 1.2.2 實作 folder launch 的 child-only `EXPLORER_INITIAL_PATH` environment block。
- [x] 1.2.3 加入 Unicode、空白、長路徑與特殊字元 round-trip tests。
- [x] 1.2.4 驗證 parent process environment before/after 完全相同。

## 2. Admission、取消與錯誤回復

### 2.1 實作 5 秒 Exactly-once Admission

**目的：** 每個 launch request 在五秒內只產生一個 terminal result。
**輸入：** Core request/generation contract、1.2 command builder、process adapter。
**產出：** Dispatcher、deadline timer、terminal mapper。
**依賴：** 1.2。
**Owner／Wave：** Explorer bridge owner／Wave 4C。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`；`evidence/artifacts/2.1/`。
**完成門檻：** launched/validation-failed/spawn-failed/cancelled/timed-out exactly once；late callback 只成 diagnostic。

- [x] 2.1.1 實作 correlation ID、generation 與 monotonic admission deadline。
- [x] 2.1.2 實作 launched、validation-failed 與 spawn-failed terminal mapping。
- [x] 2.1.3 實作 bridge-owned cancellation 與 timed-out terminal mapping。
- [x] 2.1.4 加入 cancel-vs-success race test。
- [x] 2.1.5 加入 timeout-vs-late-spawn callback test。
- [x] 2.1.6 加入 duplicate callback 與 shutdown-rundown test。
- [x] 2.1.7 對 Windows process completion callbacks 套用 Wave 2 凍結的 no-unwind wrapper。

### 2.2 實作 Process/Handle Cleanup

**目的：** 確保成功、失敗、取消、逾時與 shutdown 都關閉 thread/process 資源。
**輸入：** 2.1 dispatcher、Windows process adapter。
**產出：** RAII handle ownership、rundown tests、resource evidence。
**依賴：** 2.1。
**Owner／Wave：** Explorer bridge owner／Wave 4C。
**Gate／Evidence：** `G-SAFETY`、`G-PERF`；`evidence/artifacts/2.2/`。
**完成門檻：** 所有 terminal path 無 double-close/leak，取消不誤殺已成功交付的 child。

- [x] 2.2.1 實作 explicit application name、restricted handle inheritance 與 RAII ownership。
- [x] 2.2.2 驗證 success 與 spawn-failure 的 process/thread handle cleanup。
- [x] 2.2.3 驗證 cancellation、timeout 與 shutdown rundown cleanup。
- [x] 2.2.4 保存各 path 的 handle-count before/after evidence。

### 2.3 實作 GPUI Repair Result Contract

**目的：** 將 bridge failure 轉成可修復且保護隱私的 UI model。
**輸入：** 2.1 typed terminal results、settings command contract。
**產出：** Repair model、retry/open-settings actions、redacted diagnostics。
**依賴：** 2.1。
**Owner／Wave：** Explorer bridge owner／Wave 4C。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`、`G-A11Y-I18N`；`evidence/artifacts/2.3/`。
**完成門檻：** 每種 failure 有 truthful message/action；log 不含 credential 或完整敏感 path；不 fallback Explorer。

- [x] 2.3.1 定義 validation/spawn/cancel/timeout 的 localized repair model。
- [x] 2.3.2 實作 retry 與 open-settings commands。
- [x] 2.3.3 實作 diagnostic path/environment redaction。
- [x] 2.3.4 加入 no-Windows-Explorer-fallback 與 privacy tests。

## 3. 真實整合 Gate

### 3.1 驗證 SuperExplorer Process Integration

**目的：** 使用既有 D:\SuperExplorer binary 驗證 default 與 folder launch，不修改其 repository。
**輸入：** 1 至 2 產出、可執行 SuperExplorer binary。
**產出：** Integration traces、process/resource evidence、gate disposition。
**依賴：** 1.1、1.2、2.1、2.2、2.3。
**Owner／Wave：** Explorer bridge owner／Wave 4C exit。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`、`G-SAFETY`；`evidence/artifacts/3.1/`。
**完成門檻：** Default/folder/failure/cancel/timeout scenarios 通過且 SuperExplorer worktree hash 未變。

- [x] 3.1.1 記錄 SuperExplorer repository status、HEAD 與 target binary hash baseline。
- [x] 3.1.2 執行真實 default SuperExplorer launch integration test。
- [x] 3.1.3 執行真實 Unicode folder launch integration test。
- [x] 3.1.4 執行 invalid path 與 missing binary repair-flow test。
- [x] 3.1.5 執行 cancellation/timeout/late-callback headful test。
- [x] 3.1.6 驗證 SuperExplorer repository status/HEAD 未變並發布 gate disposition。
- [x] 3.1.7 產生 bridge public/effect schema、binary、evidence-index 與 handoff SHA-256 manifest。
