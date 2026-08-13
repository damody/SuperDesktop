## ADDED Requirements

### Requirement: 預覽模式不得接管 Windows Shell
系統 SHALL 將預覽模式設為安全開發入口，且不得在此模式隱藏 Explorer、修改系統 work area、取得工作階段 Shell 所有權或修改 Shell 登錄值。

#### Scenario: 啟動預覽模式
- **WHEN** 使用者未指定 Shell 模式而啟動 SuperDesktop
- **THEN** 系統顯示可互動的 GPUI 桌面與工作列預覽，並保持目前 Explorer 桌面、工作列、work area 及 Shell 登錄設定不變

### Requirement: Shell 接管必須採交易式啟動
系統 SHALL 在切換 Explorer 表面前完成 guardian lease、診斷、COM/GPUI、全部螢幕表面、AppBar、Hook/Hotkey、必要能力探測及互動健康檢查。

#### Scenario: 所有接管前置條件成功
- **WHEN** 使用者明確啟動 Shell 模式，且所有前置階段與健康檢查成功
- **THEN** 系統才切換目前工作階段的 Explorer Shell 表面，並記錄唯一成功接管事件

#### Scenario: 任一前置階段失敗
- **WHEN** 任一接管前置階段回報失敗或逾時
- **THEN** 系統不切換 Explorer 表面、撤銷已建立的 SuperDesktop 資源、保留可操作 Explorer，並記錄失敗階段

#### Scenario: 必要開始主機能力缺失
- **WHEN** 凍結 ExplorerPatcher reference profile 上的開始主機能力探測失敗
- **THEN** 系統拒絕 Shell 接管並維持可操作 Explorer，而不把開始按鈕宣稱為可用

### Requirement: 正常關閉必須有序釋放外部資源
系統 SHALL 停止新命令、取消進行中工作、解除 callback/Hook/Hotkey、移除 AppBar、恢復 work area、銷毀 GPUI 視窗、釋放 COM、寫入設定與診斷，最後才釋放 guardian lease。

#### Scenario: Shell 模式正常退出
- **WHEN** 使用者要求正常關閉 SuperDesktop
- **THEN** Explorer 與 work area 恢復可用，所有 SuperDesktop 外部註冊均解除，且 guardian 收到正常終端結果

#### Scenario: 重複清理
- **WHEN** cleanup 因重試或程序競態被呼叫超過一次
- **THEN** 每個清理步驟保持冪等，不產生重複 AppBar、錯誤 work area 或額外 Explorer 程序

### Requirement: Guardian 必須復原異常終止
系統 SHALL 由獨立 `superdesktop-guardian` 監看主程序 process handle 與不可偽造租約，並在接管後主程序異常終止時恢復可操作的 Explorer 工作階段。在凍結 ExplorerPatcher reference profile 的 10 次獨立 forced-crash run 中，每次都必須在主程序 handle 變為 signaled 的 monotonic timestamp T0 後 10 秒內恢復可接受 pointer/keyboard 輸入的 Explorer Shell 與正確 work area。

#### Scenario: 主程序被強制終止
- **WHEN** SuperDesktop 已完成 Shell 接管後被強制終止
- **THEN** guardian 在 T0 後 10 秒內移除或修正遺留 AppBar、恢復 work area，並顯示既有 Explorer 或在缺失時啟動一個 Explorer Shell，且保存 T0、ready timestamp、work-area snapshot 與程序 identity

#### Scenario: 任一復原樣本超時
- **WHEN** 10 次 reference forced-crash run 中任一次未在 T0 後 10 秒內達到可操作終態
- **THEN** `G-GUARDIAN-RECOVERY` 失敗，Shell 模式不得標為可發行

#### Scenario: Guardian 復原被重複觸發
- **WHEN** 相同 crash 的復原流程因重試而再次執行
- **THEN** 系統達到相同可操作終態，只保留一個復原 terminal record，且不重複啟動 Explorer

### Requirement: Guardian 租約與 Explorer target 必須防止偽造
系統 SHALL 以明確 inherited handle list 傳遞主程序 process handle 與 one-time lease channel，並以 PID、process creation time、session ID、主程式 file identity 與 nonce fencing 綁定 owner。復原 journal MUST NOT 作為授權來源。啟動 Explorer 時系統 SHALL 使用 Windows directory API 解析、canonicalize 並驗證的 Microsoft Windows Explorer 絕對路徑，提供 explicit application name，限制 inherited handles，並使用目前互動使用者 token 與受控 working directory/environment。

#### Scenario: 偽造或陳舊 journal
- **WHEN** 攻擊者或舊程序提供只有 journal、錯誤 nonce、PID creation time 或 file identity 的復原資料
- **THEN** guardian 拒絕復原動作，不修改 AppBar/work area，也不啟動任何程序

#### Scenario: PATH 或 CWD 含有替代 explorer.exe
- **WHEN** PATH 或 current working directory 中存在非系統 `explorer.exe`
- **THEN** guardian 忽略搜尋路徑，只以已驗證 Windows Explorer 絕對路徑與 explicit application name 啟動

#### Scenario: Session 或 token 不符
- **WHEN** lease 所屬 session/token 與目前互動使用者工作階段不符
- **THEN** guardian 拒絕對該工作階段進行復原，並記錄不含 credential 的安全事件

### Requirement: 每個 Windows 工作階段只能有一個 Shell owner
系統 SHALL 在任何 AppBar 或 Explorer surface mutation 前取得 session-scoped 原子 owner lease，並以 PID、creation time、session ID 與 nonce fencing 驗證 owner。非 owner MUST NOT 清理 owner 資源或切換 Explorer。

#### Scenario: 兩個主程序同時要求接管
- **WHEN** 同一 session 的兩個 SuperDesktop 主程序同時嘗試 Shell 接管
- **THEN** 只有一個程序取得 owner lease 並可繼續，另一個在修改 AppBar/Explorer 前收到 already-owned 結果

#### Scenario: Owner crash 後轉移
- **WHEN** owner crash 且其 guardian 完成 identity 驗證與復原
- **THEN** 舊 fencing token 失效，只有後續新 owner 能取得新 lease 並進行接管

#### Scenario: 非 owner 嘗試清理
- **WHEN** 非 owner 程序嘗試解除 AppBar 或恢復 Explorer
- **THEN** 系統拒絕操作且目前 owner 的桌面、工作列與 work area 保持不變

### Requirement: Win32 FFI callback 不得讓 Rust panic 穿越 ABI
系統 SHALL 將每個 `extern "system"`/Win32 callback 設為 no-unwind boundary，以 `catch_unwind` 將 panic 轉為 typed fatal event，並進入有序停止或 guardian recovery。Callback MUST 驗證輸入與 handle ownership，且不得因重複 callback 或 shutdown race 重複釋放資源。

#### Scenario: Callback 內注入 panic
- **WHEN** 測試在 Shell Hook、AppBar 或 message callback 內注入 Rust panic
- **THEN** panic 不穿越 Win32 ABI，系統產生 typed fatal event，並由 orderly shutdown 或 guardian 達到規範復原終態

#### Scenario: Shutdown race 與重複 callback
- **WHEN** callback 在解除註冊期間重複或延遲到達
- **THEN** 系統拒絕或安全處理該 callback，且每個 handle/resource 至多釋放一次

### Requirement: M0 不得持久取代登入 Shell
系統 MUST NOT 在本 change 中修改 Windows 登入 Shell 登錄值、安裝自動啟動項目或執行解除安裝遷移。

#### Scenario: 完成 M0 Shell 測試
- **WHEN** Shell 模式測試與驗證結束
- **THEN** 使用者登出後的下一次登入仍使用測試前已設定的 Windows Shell
