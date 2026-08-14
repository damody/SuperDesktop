# SuperDesktop M0 功能優先執行設計

日期：2026-08-14

狀態：設計已核准，待 OpenSpec 同步

## 1. 目的

本文件取代原本「先完成全部平台 capability gate，才開始產品功能」的執行順序。SuperDesktop M0 改採功能優先策略：先在不接管 Windows Shell 的 GPUI preview 中完成所有主要使用者功能，再進行第一次整合 smoke test，最後集中處理安全、復原、相容性、效能與正式 Shell mode 驗證。

產品方向不變：SuperDesktop 使用 Rust 與 GPUI 實作 Windows 10 風格桌面環境，以目前 Windows 11 + ExplorerPatcher UI 作為視覺參考，並透過程序邊界整合 `D:\SuperExplorer`。`D:\SuperExplorer` 不得被修改；`D:\SuperDesktop\PExplorer` 只作行為參考，不得複製或機械翻譯其程式碼。

## 2. 核心決策

### 2.1 功能完成早於 smoke test

第一次完整 smoke test 必須等下列功能全部實作並由 Primary integrator 組合完成後才執行：

1. 桌面。
2. 工作列。
3. 開始功能表。
4. 視窗切換。
5. 通知區。
6. SuperExplorer 整合。
7. 設定介面。

單一功能 owner 不得在交接時要求啟動整套桌面環境，也不得以逐功能 smoke gate 阻擋其他功能實作。功能開發期間允許的驗證只包括：

- `cargo check` 或等價的 compile-only 檢查。
- `cargo fmt --check`。
- 不啟動完整桌面環境的純邏輯單元測試。
- schema、資源檔與靜態 dependency boundary 檢查。

上述驗證不是 smoke test。不得在功能批次完成前執行完整 GPUI shell、AppBar、Shell Hook、Explorer mutation、Shell takeover、forced-crash 或跨功能 UI automation。

### 2.2 三層完成狀態

每個功能 leaf 必須明確屬於下列狀態之一：

- `implemented`：production code、資源與純邏輯行為已完成，compile-only gate 通過。
- `integrated`：Primary integrator 已接入共同 composition root，所有七個功能域可共同建置。
- `production-verified`：整體 smoke、安全、相容性、效能與復原 gate 已通過。

功能 owner 在 feature waves 只負責達到 `implemented`。Primary integrator 在整合波次負責 `integrated`。驗證波次才把功能提升為 `production-verified`。OpenSpec checkbox 與 evidence schema 必須區分這三者，不得用 `implemented` 證據宣稱整體可用或正式 Shell 安全。

### 2.3 選擇性安全閘門

功能優先不代表允許危險操作。下列操作仍然必須在對應安全 gate 通過前禁止：

- 隱藏、終止或取代 Explorer Shell。
- 註冊真實工作列 AppBar 並永久改變工作區。
- 修改登入 Shell、自動啟動、登錄值或安全設定。
- 啟動正式 guardian recovery 或 forced-crash recovery。
- 對非受控視窗執行 Shell Hook、HWND mutation 或程序操作。

在功能波次中，這些副作用一律由 fake、preview adapter 或 typed unavailable result 代替。產品 UI 必須能在 provider unavailable 時顯示合理狀態，但 feature owner 不必先證明完整 platform recovery 才能完成畫面與狀態流。

## 3. 目標架構

功能波次依賴穩定的 Rust 值型別與 effect traits，不依賴已完成的 Win32 capability：

```text
fake/preview providers ─┐
                       ├→ shell-core snapshot → GPUI desktop/taskbar/start/settings
Windows providers ─────┘

GPUI command → shell-core command → typed effect trait
                                   ├→ fake/preview effect（功能波次）
                                   └→ platform-win/explorer-bridge（整合與驗證波次）
```

`shell-core` 先提供最小可擴充 contract，讓 Desktop、Taskbar、Start/Settings 與 Explorer Bridge owner 可以平行工作。若平台能力尚未完成，UI 只消費 fake snapshot，不得等待真實 HWND、AppBar、guardian 或 ExplorerPatcher Start ABI。

開始功能表改由 SuperDesktop 擁有的 GPUI view 實作基本 M0 體驗，不再讓整個產品進度依賴 ExplorerPatcher Start host invocation。ExplorerPatcher/Windows Start invocation 可保留為後續相容 provider，但不是 preview 功能完成的前置條件。

## 4. 功能批次

### 4.1 Preview shell 與共用模型

先完成 application composition、preview window、theme/token、screen layout、command routing、fake provider、deterministic sample data 與設定 schema。此階段不要求 headful smoke，只要求 workspace 可編譯且各純 Rust contract 有單元測試。

### 4.2 桌面

完成桌布、圖示網格、Shell item model、選取、Ctrl/Shift 複選、框選、鍵盤焦點、Enter/雙擊命令、位置持久化、重新整理狀態與 SuperExplorer 固定入口。真實 Windows association、watcher、圖示解析與外部啟動可先接 typed effect，於整合波次換成 production adapter。

### 4.3 工作列與視窗切換

完成一至三列配置、預設雙列 Windows 10 視覺、工作按鈕、分組、釘選、作用中／執行中／注意狀態、點擊命令、固定 SuperExplorer 入口與多螢幕 layout model。視窗資料先由 fake tracker 驅動；真實 Shell Hook 與 EnumWindows 在後續整合。

### 4.4 開始功能表

完成 GPUI 開始面板、應用程式清單、基本搜尋、釘選區、常用系統入口、關機／重新啟動／登出命令的 typed effects，以及鍵盤開關與焦點管理。危險工作階段命令在 preview 中只能顯示確認流程與 fake result，不得真的執行。

### 4.5 通知區

完成溢出入口、時間、日期、通知數量、輸入法／網路／音量／電源的 provider slots、不可用與錯誤狀態、flyout composition boundary。功能波次使用 fake providers，不宣稱完整第三方 `Shell_NotifyIcon` 相容性。

### 4.6 SuperExplorer 整合

完成 executable resolution、固定入口、資料夾啟動 request、correlation ID、success/failure/cancel/timeout state、修復提示與設定連結。開發期可用 fake launcher 驗證狀態；整合波次才執行真實 `D:\SuperExplorer\target\release\SuperExplorer.exe` 或使用者設定的絕對路徑。不得修改 SuperExplorer repository。

### 4.7 設定介面

完成 GPUI 設定頁面、主題、工作列列數、桌布模式、SuperExplorer 路徑、執行模式偏好、語言與協助工具偏好。設定 storage contract 必須版本化並可 migration，但完整損毀／原子寫入壓力測試延至集中驗證。

## 5. 新執行波次

### Wave A：規劃與既有成果重分類

同步 program 與八個 child changes。已通過的 bootstrap、GPUI HWND、AppBar、DPI evidence 保留，不回復；它們改列為可用平台研究成果，而不是所有功能實作的 predecessor。未完成的 guardian 3.1 草稿保存為 WIP，不勾 task，移至 hardening wave。

### Wave B：共用功能 contract

Core owner 完成 shell-core、settings DTO、effect traits、fake providers 與 sample fixtures。只允許 compile、format、純邏輯 unit tests。發布 feature contract hash後開放平行功能波次。

### Wave C：平行功能實作

在最多三個 worker slot 中平行執行：

- Desktop owner：桌面與桌面 SuperExplorer 入口。
- Taskbar owner：工作列、視窗切換、通知區。
- Start/Settings owner：開始功能表與設定介面。

Explorer Bridge owner 在其中一個 slot 可用時接續處理 bridge；Primary integrator 永遠保留自己的 slot，不直接把 shared contracts 交給多個 agent 修改。

各 worker 一次只領一個 L2，交接條件是 owned paths 完成、compile-only gate 通過、純邏輯測試通過、handoff manifest 完整。不得執行完整 UI smoke。

### Wave D：SuperExplorer 與 composition 整合

Explorer Bridge owner 完成 production process boundary；Primary integrator 將七個功能域接入 `superdesktop-app`。處理 shared contract drift並確保 workspace 可共同建置。此波結束前仍不啟動完整 shell。

### Wave E：第一次基本 smoke test

只有在七個功能域全部 `implemented` 且 composition 為 `integrated` 後執行。smoke test 使用普通 GPUI preview window，不隱藏 Explorer、不註冊正式 AppBar、不啟動 guardian、不改工作區。

必測流程為：

1. 啟動 preview shell。
2. 在桌面選取與啟動 sample item。
3. 操作工作列與 fake window switching。
4. 開關開始功能表並搜尋 sample app。
5. 開啟通知區 flyout。
6. 從桌面與工作列啟動 SuperExplorer request；分別驗證成功與修復提示。
7. 修改設定並確認 UI snapshot 更新。
8. 關閉 preview shell且程序正常退出。

此波先修正跨功能與 composition 問題，直到 smoke 全部通過。smoke 通過只代表 preview 可用，不代表 Shell mode 安全。

### Wave F：平台整合與 hardening

集中完成真實 window tracker、AppBar、DPI/topology、Windows association、desktop watcher、notification providers、guardian handle lease、FFI no-unwind、Safe Mode、transactional takeover與 rollback。這時才執行 production-path negative fixtures、resource baseline、deadline、forced-crash與 lifecycle evidence。

任何未通過的 platform gate只阻擋對應 production provider或 Shell mode，不回復已完成的 preview UI。UI 以 typed unavailable/repair state維持可操作。

### Wave G：release verification

集中執行 UI automation、視覺比較、協助工具、在地化、RTL/簡中 fallback、IME、效能、soak、Windows 11 recovery、Windows 10 22H2 與實體 mixed-DPI 雙螢幕確認。外部環境缺失的 leaf 保持 `blocked`，其他可執行工作繼續。

## 6. Multi-agent ownership

| 角色 | 功能波次 ownership | 禁止事項 |
| --- | --- | --- |
| Primary integrator | Program graph、shared contracts 合併、composition、smoke、最終 evidence | 不得把 compile-only 宣稱為 smoke 或 production pass |
| Core owner | `shell-core`、settings contract、fake provider traits | 不得修改 GPUI domain views |
| Desktop owner | `desktop-ui`、desktop-specific adapter | 不得修改 taskbar/start/shared core |
| Taskbar owner | `taskbar-ui` 的工作列、視窗切換、通知區 | 不得修改 desktop/bridge/shared core |
| Start/Settings owner | GPUI Start 與 Settings domain modules | 不得修改 taskbar/desktop/shared core |
| Explorer Bridge owner | `explorer-bridge` 與 bridge-specific adapter | 不得修改 `D:\SuperExplorer` |
| Platform/Lifecycle owner | hardening wave 的 `platform-win`、guardian、takeover | 不得在功能波次要求完整 capability gate |
| Independent reviewer | 波次邊界與最終 P0/P1 唯讀複核 | 不得修改 production code |

同時最多四個 agent，包含 Primary。Primary 固定保留一個 slot；任何時刻最多三個 subagent。不得讓兩個 agent 同時修改相同檔案或 shared crate。每個 subagent 一次只負責一個 L2 work package。

## 7. Evidence 與 checkbox 規則

feature waves 的 mandatory evidence 縮減為功能實作所需的可判定證據：owned diff、compile command、純邏輯 test、resource/schema validation、handoff hash。不得要求 feature owner 在第一次 smoke 前提供 headful screenshot、UIA、AppBar、Shell Hook、forced-crash或 resource soak。

所有延後的 smoke、integration、security 與 release 驗證必須成為獨立 mandatory leaves，不得刪除或改成 optional/N/A。原本綁在每個 feature L2 的 headful／production gate移到 Wave E、F、G，並保留 requirement/scenario/gate traceability。這是重新排序，不是降低最終驗收標準。

task validator 必須拒絕下列情況：

- 七個功能域尚未全部 implemented 就執行或通過 basic smoke。
- 以 compile-only evidence 標記 integrated 或 production-verified。
- 以 fake provider 證據標記 Windows production provider passed。
- platform gate failed/blocked 時仍啟用其 Shell mutation。
- release leaf 以無 eligibility 的 N/A、循環 superseded 或無效 replacement 結案。

## 8. 現有工作處置

- Wave 0、Wave 1 與已接受的 Wave 2 L2 1.1、1.2、2.1、2.2 commits 保留。
- `G-DPI-MONITOR` 已通過的 evidence 保留。
- `G-TASKBAR stop` 不再阻擋自有 GPUI 開始功能表與工作列 UI；它只阻擋 ExplorerPatcher Start invocation provider與依賴該 provider的 Shell takeover。
- 未提交的 Guardian 3.1 程式碼與 artifacts 保留為 hardening WIP，不視為完成，不納入功能 contract hash。
- 已勾選 task 只有在其原 evidence仍有效時保留；若語意從 production gate 改為 research result，必須增加 corrective lineage，不得改寫歷史 evidence。

## 9. 完成標準

M0 preview milestone 在七個功能域 implemented、composition integrated、Wave E smoke 全通過時成立。M0 production milestone仍要求 Wave F/G 的所有 mandatory安全、復原、相容性、效能與 release gates通過。

因此使用者會先得到可以實際操作與持續迭代的完整 preview，再集中消除底層風險；專案不再因單一 guardian、Start ABI 或 capability spike細節阻塞所有產品功能，但也不會在那些風險尚未通過前宣稱正式 Shell mode可用。

## 10. 已核准決策

- 採功能優先、選擇性安全閘門，不採全面 gate-first。
- 桌面、工作列、開始功能表、視窗切換、通知區、SuperExplorer 整合與設定介面全部完成後，才執行第一次基本 smoke test。
- 功能實作期間仍執行 compile、format與純邏輯 unit tests；它們不算 smoke test。
- 自有 GPUI 開始功能表納入 M0 preview，解除對 ExplorerPatcher Start ABI 的功能開發依賴。
- 完整安全與 release gate 延後集中執行，但不刪除、不降級、不以 N/A 逃逸。
