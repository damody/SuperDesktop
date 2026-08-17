## 1. Preview 與 Session Ownership

### 1.1 實作 Preview Zero-mutation 與環境 Probe

**目的：** 預設 preview 且在任何 Shell mutation 前拒絕 Safe Mode/unsupported session。
**輸入：** Capability go、settings execution preference、platform probes。
**產出：** Execution admission state machine、zero-mutation tests。
**依賴：** Desktop、taskbar、bridge changes 已通過。
**Owner／Wave：** Lifecycle owner／Wave 5。
**Gate／Evidence：** `G-SHELL-TAKEOVER-PROVISIONAL`、`G-SAFETY`；`evidence/artifacts/1.1/`。
**完成門檻：** 預設啟動不改 AppBar/Explorer/work area；Safe Mode/unsupported session 在 mutation 前拒絕。

- [x] 1.1.1 實作預設 preview 與每次啟動的明確 Shell-mode opt-in。
- [x] 1.1.2 實作 Windows Safe Mode 與 unsupported/non-interactive session probe。
- [x] 1.1.3 建立 preview before/after AppBar、Explorer 與 work-area zero-mutation test。
- [x] 1.1.4 建立 Safe Mode/unsupported session zero-mutation rejection test。

### 1.2 實作 Session-scoped Single-owner Lease

**目的：** 在任何 AppBar/Explorer mutation 前取得具 identity fencing 的唯一 owner lease。
**輸入：** 1.1 admission、bootstrap product identity。
**產出：** Session lease、owner fencing、race tests。
**依賴：** 1.1。
**Owner／Wave：** Lifecycle owner／Wave 5。
**Gate／Evidence：** `G-SHELL-TAKEOVER-PROVISIONAL`、`G-SAFETY`；`evidence/artifacts/1.2/`。
**完成門檻：** 同 session 只有一個 owner；PID/creation/session/user token/app file identity/nonce 每次 mutation 與 cleanup 都 revalidate；non-owner 不能操作；crash 後可安全 transfer。

- [x] 1.2.1 實作 session-scoped atomic lease/mutex 與 protected metadata。
- [x] 1.2.2 綁定 PID、process creation time、session ID、user token、SuperDesktop executable file identity 與 nonce fencing。
- [x] 1.2.3 執行兩個主程序 simultaneous takeover race test。
- [x] 1.2.4 在每次 AppBar/Explorer mutation 與 cleanup 前重新驗證 owner lease identity。
- [x] 1.2.5 執行 wrong-file、replaced-binary、wrong-user/token 與 non-owner cleanup rejection tests。
- [x] 1.2.6 執行 owner crash 後 lease transfer test。

### 1.3 組裝 Preview Composition Root

**目的：** 在零 Shell mutation 的 preview 中把 core、settings、desktop、taskbar 與 SuperExplorer bridge 接成真實產品路徑。
**輸入：** 1.1 admission、desktop/taskbar/bridge 已通過的 public contracts、core contract hash。
**產出：** `superdesktop-app` composition root、effect routing、end-to-end preview tests。
**依賴：** 1.1、所有 Wave 4 changes 已通過。
**Owner／Wave：** Lifecycle owner／Wave 5。
**Gate／Evidence：** `G-ARCH`、`G-DESKTOP`、`G-TASKBAR`、`G-EXPLORER-BRIDGE`；`evidence/artifacts/1.3/`。
**完成門檻：** 真實 adapters 只經 core command/event contract 連接，preview 可完成桌面檔案 association 與工作列 SuperExplorer 啟動且維持 zero-mutation。

- [x] 1.3.1 組裝 shell-core、settings-store 與 platform-win common adapters。
- [x] 1.3.2 組裝 desktop-ui 與 desktop namespace/association effects。
- [x] 1.3.3 組裝 taskbar-ui 與 window/AppBar/Start effects，但 preview 禁止 AppBar mutation。
- [x] 1.3.4 組裝 fixed SuperExplorer entry 到 explorer-bridge dispatcher 的真實路由。
- [x] 1.3.5 執行 preview 桌面一般檔案 association end-to-end test。
- [x] 1.3.6 對 desktop 與 taskbar fixed entries 分別執行 pointer/keyboard/UIA 到真實 SuperExplorer launch end-to-end test。
- [x] 1.3.7 驗證 composition preview 前後 Explorer、AppBar 與 work area zero-mutation。

## 2. Transactional Takeover 與正常回復

### 2.1 實作六階段 Takeover Transaction

**目的：** 只有 prerequisites 全數成立才 commit Shell takeover，任一失敗可 rollback。
**輸入：** 1.2 owner lease、1.3 composition root、guardian executable。
**產出：** Takeover coordinator、journal、failpoints。
**依賴：** 1.2、1.3。
**Owner／Wave：** Lifecycle owner／Wave 5。
**Gate／Evidence：** `G-SHELL-TAKEOVER-PROVISIONAL`；`evidence/artifacts/2.1/`。
**完成門檻：** Probe、guardian、surfaces、AppBar/hooks、五秒 input health、Explorer mutation、commit 順序固定；health failure/timeout 與所有 failpoint 皆回復 baseline且零 Explorer mutation。

- [x] 2.1.1 實作 prerequisite probe 與 immutable takeover journal start。
- [x] 2.1.2 啟動 guardian 並完成 lease handshake 後才允許後續 mutation。
- [x] 2.1.3 建立 desktop/taskbar surfaces 並驗證 ready identities。
- [x] 2.1.4 註冊 AppBar、hooks/hotkeys 並保存 rollback tokens。
- [x] 2.1.5 在五秒 deadline 內以 pointer、keyboard、focus 與必要 Start probe 驗證 desktop/taskbar input health。
- [x] 2.1.6 驗證 health failure/timeout 會 rollback 且不執行 Explorer surface mutation。
- [x] 2.1.7 通過 health 後才執行 Explorer surface mutation 並寫入 commit marker。
- [x] 2.1.8 對每個 takeover phase 注入失敗並驗證反向 rollback。
- [x] 2.1.9 在 reference profile Shell mode 執行 Start probe/invocation 並保存 UI、Explorer 與 work-area snapshots。

### 2.2 實作有序且冪等的正常關閉

**目的：** 正常退出以反向順序解除資源並恢復 Explorer/work area。
**輸入：** 2.1 committed transaction。
**產出：** Shutdown coordinator、idempotence tests、recovery evidence。
**依賴：** 2.1。
**Owner／Wave：** Lifecycle owner／Wave 5。
**Gate／Evidence：** `G-SHELL-TAKEOVER-PROVISIONAL`、`G-SAFETY`；`evidence/artifacts/2.2/`。
**完成門檻：** 重複/交錯 shutdown 不 double-unregister/close，Explorer 與 work area 回 baseline，lease 最後釋放。

- [x] 2.2.1 實作停止接受新 command。
- [x] 2.2.2 實作 bridge 與其他進行中 request cancellation rundown。
- [x] 2.2.3 實作 hook/hotkey unregister。
- [x] 2.2.4 實作 AppBar removal 與 work-area restore。
- [x] 2.2.5 實作 desktop/taskbar GPUI surface teardown。
- [x] 2.2.6 實作 Explorer surface restore。
- [x] 2.2.7 實作 COM apartment/resource release 並驗證 callback rundown。
- [x] 2.2.8 Flush settings 與 diagnostics，記錄 durable completion marker。
- [x] 2.2.9 完成 guardian terminal handshake 後最後釋放 owner lease。
- [x] 2.2.10 執行 shutdown ordering timeline assertion。
- [x] 2.2.11 執行 repeated、concurrent、partial-shutdown 與各邊界 crash tests。
- [x] 2.2.12 保存 raw timeline、thread/handle/COM/GPUI resource 與 work-area evidence。

## 3. Guardian 防偽與 Crash Recovery

### 3.1 實作受保護 Guardian Lease 與 Target Validation

**目的：** 阻止 forged/stale journal、錯 session 與 executable substitution 觸發錯誤 recovery。
**輸入：** Capability guardian spike、owner lease、takeover journal。
**產出：** Guardian validation pipeline、negative fixtures。
**依賴：** 1.2、2.1。
**Owner／Wave：** Guardian owner／Wave 5。
**Gate／Evidence：** `G-GUARDIAN-RECOVERY-PROVISIONAL`、`G-SAFETY`；`evidence/artifacts/3.1/`。
**完成門檻：** 只有 inherited process handle 與完整 PID/creation/session/owner-token/file/nonce 驗證通過才執行 recovery。

- [x] 3.1.1 實作 restricted inherited handle list 與 one-time lease channel ACL。
- [x] 3.1.2 驗證主程序 PID、creation time、session、owner user-token identity、nonce 與 executable file identity。
- [x] 3.1.3 實作 journal ACL、canonical path 與 reparse protection。
- [x] 3.1.4 執行 forged、stale、reparse、wrong-session、same-session wrong-token、token-replacement race 與 wrong-owner zero-mutation/no-spawn tests。
- [x] 3.1.5 驗證 concurrent guardian 只有一個 recovery owner。

### 3.2 實作可信 Windows Explorer Recovery Launch

**目的：** 只啟動已驗證 Windows 系統 Explorer，限制 token/session/environment/handles。
**輸入：** 3.1 validated recovery authority、Windows directory API。
**產出：** Explorer launch adapter、negative tests。
**依賴：** 3.1。
**Owner／Wave：** Guardian owner／Wave 5。
**Gate／Evidence：** `G-GUARDIAN-RECOVERY-PROVISIONAL`、`G-SAFETY`；`evidence/artifacts/3.2/`。
**完成門檻：** 絕對系統路徑與 explicit application name 通過；PATH/CWD/wrong-session substitution 全被拒絕。

- [x] 3.2.1 由 Windows directory API 解析並 canonicalize 系統 explorer.exe。
- [x] 3.2.2 驗證 Microsoft Windows Explorer file identity 與 signature metadata。
- [x] 3.2.3 實作 explicit application name、interactive session/token 與 restricted inheritance。
- [x] 3.2.4 限制 working directory/environment 並清除非必要 inherited handles。
- [x] 3.2.5 執行 PATH、CWD、fake binary、wrong-session 與 unexpected-handle tests。

### 3.3 實作冪等 Shell Recovery Coordinator

**目的：** Crash 後先回復 AppBar/work area，再優先顯示既有 Explorer，缺失時只 spawn 一次。
**輸入：** 3.1 validated authority、3.2 trusted launcher、takeover rollback tokens。
**產出：** Recovery coordinator、Explorer probe/show adapter、exactly-once terminal tests。
**依賴：** 2.1、3.1、3.2。
**Owner／Wave：** Guardian owner／Wave 5。
**Gate／Evidence：** `G-GUARDIAN-RECOVERY-PROVISIONAL`、`G-SAFETY`；`evidence/artifacts/3.3/`。
**完成門檻：** AppBar/work area 先回 baseline；既有 Explorer 不重啟；缺失 Explorer只啟動一次；重複 trigger 共用單一 terminal。

- [x] 3.3.1 實作 guardian-side AppBar removal 與 per-monitor work-area restore。
- [x] 3.3.2 實作同 session Explorer Shell probe、file/token identity validation 與 show existing path。
- [x] 3.3.3 實作 no-usable-Explorer 時的 single-flight verified spawn path。
- [x] 3.3.4 實作 recovery request identity、single owner 與 exactly-once terminal registry。
- [x] 3.3.5 測試既有隱藏 Explorer 被顯示且 process count 不增加。
- [x] 3.3.6 測試 Explorer 缺失時只 spawn 一次並達 input-ready。
- [x] 3.3.7 測試 concurrent/repeated triggers 無 duplicate spawn/terminal。
- [x] 3.3.8 保存 AppBar/work-area、Explorer identity/process-count 與 terminal evidence。

### 3.4 驗證十秒 Crash Recovery Deadline

**目的：** 從 guardian 觀察 inherited handle signaled 的 T0 起十秒內恢復可操作 Explorer 與 work area。
**輸入：** 3.3 recovery coordinator 與 instrumentation。
**產出：** Forced-crash harness、10-run raw timing evidence。
**依賴：** 3.3。
**Owner／Wave：** Lifecycle verification owner／Wave 5。
**Gate／Evidence：** `G-GUARDIAN-RECOVERY-PROVISIONAL`；`evidence/artifacts/3.4/`。
**完成門檻：** 10/10 runs 在 T0+10s 內達 Explorer pointer/keyboard usable 且 work area 正確，無失敗 run。

- [x] 3.4.1 實作 guardian T0、Explorer-ready 與 work-area timestamp instrumentation。
- [x] 3.4.2 實作 Shell-mode forced-crash harness 與每 run identity snapshot。
- [x] 3.4.3 執行十次 forced-crash recovery 並保存逐 run raw timestamps。
- [x] 3.4.4 驗證每 run Explorer pointer/keyboard 可操作與 work-area baseline。
- [x] 3.4.5 驗證逾時、錯 identity 或 terminal ambiguity 會使 gate 失敗。

## 4. FFI 與產品安全邊界

### 4.1 稽核 No-unwind Callback Boundary

**目的：** 稽核各 domain owner 已套用凍結 wrapper，並只在 lifecycle-owned callbacks 補齊實作。
**輸入：** Wave 2 FFI wrapper hash、desktop/taskbar/bridge manifests、lifecycle callbacks。
**產出：** Callback inventory、lifecycle adapter、release panic/race audit。
**依賴：** 2.1、3.1。
**Owner／Wave：** Platform safety owner 稽核；Lifecycle owner只改 lifecycle paths／Wave 5。
**Gate／Evidence：** `G-SAFETY`、`G-GUARDIAN-RECOVERY-PROVISIONAL`；`evidence/artifacts/4.1/`。
**完成門檻：** 每個 extern callback 受 wrapper 保護；panic/double-callback/shutdown-race 進入安全 terminal。

- [x] 4.1.1 產生按 common/desktop/taskbar/bridge/lifecycle 分域的 production callback inventory。
- [x] 4.1.2 驗證 Wave 4 manifests 中每個 domain callback 已套用相同 wrapper hash。
- [x] 4.1.3 對 lifecycle-owned entrypoints 套用 catch-unwind 與 input/ownership validation。
- [x] 4.1.4 將 lifecycle callback panic 轉成 typed fatal event 並觸發 guardian-safe rundown。
- [x] 4.1.5 對 release binary 的每種 callback 執行 panic injection test並驗證不是 abort。
- [x] 4.1.6 執行 double callback、shutdown race 與 handle close-pair tests。

### 4.2 驗證無登入 Shell 永久 Mutation

**目的：** 證明 M0 不修改登入 Shell registry、policy 或 installer state。
**輸入：** 完整 lifecycle binary、registry/system baseline capture。
**產出：** Before/after diff、safety disposition。
**依賴：** 2.2、3.4、4.1。
**Owner／Wave：** Safety owner／Wave 5 exit。
**Gate／Evidence：** `G-SAFETY`、`G-SHELL-TAKEOVER-PROVISIONAL`、`G-GUARDIAN-RECOVERY-PROVISIONAL`；`evidence/artifacts/4.2/`。
**完成門檻：** Preview、正常 Shell exit、forced crash 後登入 Shell/registry/policy baseline 無永久差異；只發布 reference-profile provisional dispositions，final gates 由 completion verifier 結合 candidate-bound installer/lifecycle evidence 判定。

- [x] 4.2.1 擷取登入 Shell、Explorer policy 與相關 registry baseline。
- [x] 4.2.2 執行 preview cycle 並比較 zero permanent mutation。
- [x] 4.2.3 執行正常 Shell-mode cycle 並比較 zero permanent mutation。
- [x] 4.2.4 執行 forced-crash cycle 並比較 zero permanent mutation。
- [x] 4.2.5 索引 lifecycle evidence並發布 reference-profile provisional takeover/recovery 與 safety dispositions。
- [x] 4.2.6 產生 lifecycle public/effect schema、binary、evidence-index 與 handoff SHA-256 manifest。
