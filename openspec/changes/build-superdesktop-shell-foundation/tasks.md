# SuperDesktop M0 實作任務

所有完成的 L3 leaf 必須在 `evidence/index.jsonl` 使用相同 `task_id` 建立唯一紀錄，或引用 immutable shared record 並提供唯一 `subcheck`。紀錄格式依 `design.md` 第 10 節；本文件所有現有 leaf 都是 mandatory，只能以 `passed`，或在 replacement 存在、無循環、仍為 mandatory、維持相同 requirement/scenario/gate coverage 且 replacement 已有有效 `passed` evidence 後，以 `superseded` 結案。此前原 leaf 不得勾選完成，也不允許 `not-applicable`。日後新增的 conditional leaf 必須在建立時明列客觀 eligibility、replacement coverage 與 gate disposition，才可用具證據的 `not-applicable`。`failed`、`blocked`、`stale` 或未執行皆不得勾選完成。

## 1. 治理、證據與 Workspace 基礎

### 1.1 建立證據與調整治理

**目的：** 建立所有後續 gate 共用、可機器驗證且保留 lineage 的證據格式。
**輸入：** 核准設計、`design.md` 第 10–11 節、六份 delta spec。
**產出：** `evidence/schema.json`、`evidence/index.jsonl`、`evidence/adjustments.md`、驗證腳本與使用說明。
**依賴：** 無。
**Owner／Wave：** Primary integrator；Wave 1。
**Gate／Evidence：** `G-TRACE`；`evidence/artifacts/1.1/` 與 task_id 對應索引。
**完成門檻：** Schema 能拒絕缺欄、重複 task_id、無 replacement 的 superseded、無依據的 not-applicable 與 stale-as-complete，且範例索引通過驗證。

- [ ] 1.1.1 定義 evidence JSON schema、mandatory/conditional leaf、terminal disposition、shared record/subcheck 與 hash 欄位。
- [ ] 1.1.2 實作 evidence index validator，拒絕 mandatory not-applicable、dangling/cyclic/incomplete replacement、重複、缺欄、stale、blocked 或 failed completion。
- [ ] 1.1.3 建立 A/B/C adjustment ledger 格式與 evidence lineage 規則。
- [ ] 1.1.4 建立 passed 與 mandatory-not-applicable 負面 evidence fixtures。
- [ ] 1.1.5 建立 dangling、cyclic、non-mandatory、coverage-mismatch 與 incomplete replacement 負面 fixtures。
- [ ] 1.1.6 執行 evidence validator 自測並保存輸出與 fixture hash。

### 1.2 建立 Windows-only Rust Workspace

**目的：** 建立可重現、具有產品 identity 且依賴來源固定的 Cargo workspace。
**輸入：** 核准設計的 workspace 單元、SuperExplorer 已驗證 GPUI-CE revision 資訊。
**產出：** 根 `Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、各 crate manifest、Windows resource/manifest 與依賴來源文件。
**依賴：** 1.1。
**Owner／Wave：** Primary integrator；Wave 1。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/1.2/`。
**完成門檻：** Fresh workspace metadata 可解析、Windows target check 通過、lockfile 固定、非 Windows target 明確拒絕且產品 binary identity 正確。

- [ ] 1.2.1 建立九個設計 crate 的 workspace manifest 與最小 library/binary targets。
- [ ] 1.2.2 固定 Rust toolchain、Windows target policy、GPUI-CE revision 與 dependency lock。
- [ ] 1.2.3 建立 SuperDesktop 與 guardian 的 Windows manifest、VERSIONINFO 及產品 icon 資源。
- [ ] 1.2.4 加入非 Windows compile guard 與 Windows-only CI/local gate 命令。
- [ ] 1.2.5 記錄 GPUI-CE、Windows bindings、PExplorer 與 SuperExplorer 的來源/授權邊界。
- [ ] 1.2.6 在乾淨 Cargo cache 條件下執行 metadata/check 並保存 lockfile 與 binary metadata hash。
- [ ] 1.2.7 建立 isolated `CARGO_HOME` 的 dependency source/hash manifest 與完整預取或 vendor 程序。
- [ ] 1.2.8 在停用網路的 isolated 環境執行 `cargo check --locked --offline` 並保存結果。

### 1.3 強制架構依賴方向

**目的：** 防止 UI、核心與平台責任在實作期間重新耦合。
**輸入：** 1.2 workspace、設計中的 crate dependency graph。
**產出：** architecture checker、允許/禁止依賴規則與 contract test。
**依賴：** 1.2。
**Owner／Wave：** Architecture owner；Wave 1。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/1.3/`。
**完成門檻：** Checker 能證明 UI 不公開 Win32/COM、`shell-core` 不依賴 GPUI/Win32、composition 只在 app crate，並可由負面 fixture 證明 gate 有效。

- [ ] 1.3.1 定義 crate dependency allowlist 與 Windows/GPUI type boundary 規則。
- [ ] 1.3.2 實作 Cargo metadata 與 source scan architecture checker。
- [ ] 1.3.3 建立故意違反 UI-to-Win32 與 core-to-GPUI 邊界的負面 fixture。
- [ ] 1.3.4 執行正負 architecture gate 並索引證據。

### 1.4 完成 Blocking Windows/GPUI Capability Spike

**目的：** 在大量 production 實作前證明固定 GPUI-CE revision 與 Windows 10 Shell 必要橋接可行。
**輸入：** 1.2 固定候選 dependency、Windows 10 22H2 reference machine、最小 spike harness。
**產出：** `spikes/windows-shell-capabilities/`、source/binary hashes、OS metadata、raw results、resource snapshots 與 go/stop disposition。
**依賴：** 1.2、1.3。
**Owner／Wave：** Windows platform owner；Wave 1，所有相依 production work 的 blocking predecessor。
**Gate／Evidence：** `G-ARCH`、`G-SHELL-TAKEOVER`、`G-DPI-MONITOR`；`evidence/artifacts/1.4/`。
**完成門檻：** 最小 GPUI native HWND/message bridge、AppBar reserve/restore、Shell Hook、per-monitor DPI/topology、Windows 10 Start host probe/invocation 與 guardian process-handle lease 均有通過證據；任一必要 subcheck 失敗時停止相依 package 並啟動 B/C 流程。

- [ ] 1.4.1 建立固定 revision 的最小 GPUI 視窗並驗證 native HWND/message bridge。
- [ ] 1.4.2 驗證 AppBar register/reserve/update/remove 與原 work area restore。
- [ ] 1.4.3 驗證 Shell Hook 增量事件與 unregister 後無 callback。
- [ ] 1.4.4 驗證 per-monitor DPI、monitor identity 與 topology change event。
- [ ] 1.4.5 驗證 Windows 10 Start host probe 與 invocation。
- [ ] 1.4.6 驗證 guardian inherited process handle、one-time lease channel 與 crash signal。
- [ ] 1.4.7 執行 spike resource snapshot/soak 並記錄 go/stop disposition。

## 2. 核心狀態、平台能力與設定

### 2.1 實作 Shell Core 型別與 Reducer

**目的：** 建立不依賴 GPUI/Win32 的單一權威狀態快照與可測試轉移。
**輸入：** 所有 capability spec、1.3 架構規則。
**產出：** `shell-core` identity、command、event、effect、snapshot、recovery phase 與 reducer。
**依賴：** 1.3。
**Owner／Wave：** Core owner；Wave 2。
**Gate／Evidence：** `G-ARCH`、`G-TRACE`；`evidence/artifacts/2.1/`。
**完成門檻：** 所有 M0 user-visible state 只能由 reducer 改變，型別不含平台 handle，單元測試涵蓋合法/非法轉移與 exactly-once terminal。

- [ ] 2.1.1 定義 MonitorId、ShellItemId、WindowId、ApplicationId、RequestId、Generation 與 CorrelationId。
- [ ] 2.1.2 定義 desktop/taskbar/settings/recovery snapshot 與不可變更新介面。
- [ ] 2.1.3 定義 typed command、platform event、effect 與 terminal result enums。
- [ ] 2.1.4 實作 reducer 的合法狀態轉移與非法轉移錯誤。
- [ ] 2.1.5 實作 correlation exactly-once terminal registry。
- [ ] 2.1.6 建立 reducer、非法轉移及 terminal 競態單元測試。

### 2.2 實作 Generation、有界 Queue 與權威對帳協定

**目的：** 在事件風暴、遺失、重排與延遲 callback 下維持權威狀態。
**輸入：** 2.1 typed events、`shell-settings-and-reconciliation` spec。
**產出：** cancellation/generation registry、bounded queues、coalescer、overflow/reconciliation protocol。
**依賴：** 2.1。
**Owner／Wave：** Core owner；Wave 2。
**Gate／Evidence：** `G-DESKTOP`、`G-TASKBAR`；`evidence/artifacts/2.2/`。
**完成門檻：** 重複、missing、out-of-order、stale、cancelled 與 overflow sequence property tests 全數通過。

- [ ] 2.2.1 實作 request generation、cancellation 與 late-result rejection。
- [ ] 2.2.2 實作 desktop/window bounded event queue 及明確 overflow event。
- [ ] 2.2.3 實作依 stable identity 的安全 coalescing 規則。
- [ ] 2.2.4 定義 `EnumWindows` 與 desktop namespace 全量對帳 effect/terminal。
- [ ] 2.2.5 建立 duplicate/missing/reordered/stale sequence property tests。
- [ ] 2.2.6 建立 queue saturation 與 overflow recovery deterministic tests。

### 2.3 實作 Windows 執行緒、COM 與能力探測骨架

**目的：** 建立 apartment-safe 平台執行環境與 Shell 接管先決能力報告。
**輸入：** 1.2 manifests、2.1 contracts、Windows 10 reference requirements。
**產出：** `platform-win` thread ownership、message-only HWND、COM/OLE lifecycle、capability report 與 resource snapshot。
**依賴：** 1.2、1.4、2.1。
**Owner／Wave：** Windows platform owner；Wave 2。
**Gate／Evidence：** `G-ARCH`、`G-SHELL-TAKEOVER`；`evidence/artifacts/2.3/`。
**完成門檻：** Apartment-affine value 不跨執行緒，能力報告可區分 required/optional/unavailable，初始化/關閉與 failure unwind 無資源洩漏。

- [ ] 2.3.1 建立 STA/MTA owner threads、message pump 與 typed endpoint。
- [ ] 2.3.2 建立 message-only HWND 與 callback-to-owned-event adapter。
- [ ] 2.3.3 實作 COM/OLE 初始化、取消、deadline 與 RAII shutdown。
- [ ] 2.3.4 實作 AppBar、Shell Hook、Start host、monitor/DPI 與 guardian prerequisite probe。
- [ ] 2.3.5 實作 Windows Safe Mode、interactive session 與 supported-session fail-closed probe。
- [ ] 2.3.6 實作 thread/window/COM owned resource snapshot。
- [ ] 2.3.7 實作所有 `extern "system"` callback 的 `catch_unwind` no-unwind wrapper 與 typed fatal event。
- [ ] 2.3.8 實作 callback input/handle ownership validation 與 at-most-once release guard。
- [ ] 2.3.9 執行 callback panic injection，證明 Rust unwind 不穿越 Win32 ABI。
- [ ] 2.3.10 執行重複 callback、late callback 與 shutdown race tests。
- [ ] 2.3.11 執行 Safe Mode/unsupported session 接管拒絕與 zero-mutation test。
- [ ] 2.3.12 執行 platform init/fail/unwind/soak contract tests 並保存資源差異。

### 2.4 實作版本化設定 Store

**目的：** 可靠保存 M0 設定並從欄位錯誤、損毀與中斷寫入復原。
**輸入：** `shell-settings-and-reconciliation` spec、2.1 settings snapshot。
**產出：** versioned schema、migration、atomic writer、quarantine reader 與 fake store。
**依賴：** 2.1、1.1。
**Owner／Wave：** Persistence owner；Wave 2。
**Gate／Evidence：** `G-SAFETY`、`G-TRACE`；`evidence/artifacts/2.4/`。
**完成門檻：** Round-trip、invalid-field、corrupt-file、interrupted-write、migration 與 target-boundary tests 通過，且原始損毀資料被保留。

- [ ] 2.4.1 定義含 execution-mode preference 的 settings v1 schema、defaults、validation 與 serialization。
- [ ] 2.4.2 實作 temp-write、flush、驗證與 atomic replace。
- [ ] 2.4.3 實作欄位級 fallback 與 timestamped quarantine。
- [ ] 2.4.4 實作 schema migration registry 與未知新版拒絕策略。
- [ ] 2.4.5 建立 interrupted-write、corruption、invalid-row 與 round-trip tests。
- [ ] 2.4.6 驗證所有測試寫入限制在 canonicalized fixture root。
- [ ] 2.4.7 執行 execution-mode preference round-trip/migration，並證明未明確指定時仍預設 preview。

## 3. GPUI 桌面能力

### 3.1 實作螢幕、DPI 與 Desktop Surface Host

**目的：** 為每個使用中螢幕提供 bounds 正確、可隨 topology 更新的 bottommost GPUI surface。
**輸入：** 2.3 platform runtime、2.1 monitor model。
**產出：** monitor enumerator/event adapter、desktop window host、logical/physical conversion 與 hot-plug handling。
**依賴：** 1.4、2.1、2.3。
**Owner／Wave：** Windows platform owner；Wave 3。
**Gate／Evidence：** `G-DPI-MONITOR`、`G-DESKTOP`；`evidence/artifacts/3.1/`。
**完成門檻：** 單/雙螢幕、mixed DPI、hot-plug 與 topology reorder integration tests 產生正確 surface/work-area snapshot。

- [ ] 3.1.1 實作穩定 monitor identity、topology enumeration 與 display-change events。
- [ ] 3.1.2 實作 per-monitor DPI awareness 與 logical/physical geometry conversion。
- [ ] 3.1.3 建立每螢幕 bottommost GPUI desktop host 與 lifecycle。
- [ ] 3.1.4 實作 hot-plug/reorder 時 surface 建立、更新與移除。
- [ ] 3.1.5 建立 fake topology contract tests。
- [ ] 3.1.6 在真實 mixed-DPI 雙螢幕擷取 host geometry evidence。

### 3.2 實作桌布 Pipeline

**目的：** 以 GPUI 安全呈現 Windows 桌布模式並處理不可用來源。
**輸入：** 3.1 desktop hosts、2.4 wallpaper settings。
**產出：** wallpaper loader/cache、fill/fit/stretch/center/tile/span layout 與 fallback。
**依賴：** 3.1、2.4。
**Owner／Wave：** Desktop UI owner；Wave 3。
**Gate／Evidence：** `G-DESKTOP`、`G-PERF`；`evidence/artifacts/3.2/`。
**完成門檻：** 六種模式 geometry 與 visual fixtures 通過，無效/大型影像不阻塞 UI 且 cache 有界。

- [ ] 3.2.1 實作非同步 wallpaper decode 與有界 cache。
- [ ] 3.2.2 實作 fill、fit、stretch、center 與 tile geometry。
- [ ] 3.2.3 實作 topology-aware span geometry。
- [ ] 3.2.4 實作 unreadable/invalid image 的語意背景 fallback 與 redacted diagnostics。
- [ ] 3.2.5 建立各模式 DPI visual fixtures 與 geometry tests。
- [ ] 3.2.6 執行大型影像切換與 cache budget benchmark。

### 3.3 實作 Desktop Shell Namespace 與 Watcher

**目的：** 合併 User/Public Desktop，提供 owned identity/icon/name/capability 並能從 watcher overflow 復原。
**輸入：** 2.3 COM runtime、2.2 reconciliation protocol。
**產出：** namespace enumerator、Shell item DTO、icon service、watcher 與 full refresh adapter。
**依賴：** 2.2、2.3。
**Owner／Wave：** Windows Shell owner；Wave 3。
**Gate／Evidence：** `G-DESKTOP`、`G-SAFETY`；`evidence/artifacts/3.3/`。
**完成門檻：** 同名不同 identity、rename、create/delete storm、overflow、stale refresh 與 resource soak tests 通過。

- [ ] 3.3.1 實作 User/Public Desktop known-folder enumeration 與合併規則。
- [ ] 3.3.2 實作 stable Shell identity、display name、icon descriptor 與 capability DTO。
- [ ] 3.3.3 實作非同步 icon resolution、cache 與 invalidation。
- [ ] 3.3.4 實作 desktop filesystem watcher、rename pairing 與 event coalescing。
- [ ] 3.3.5 實作 watcher overflow 的全量 namespace refresh。
- [ ] 3.3.6 建立同名不同 identity integration test。
- [ ] 3.3.7 建立 Unicode 顯示名稱與 icon resolution test。
- [ ] 3.3.8 建立 hidden/system item capability test。
- [ ] 3.3.9 建立 rename storm/coalescing test。
- [ ] 3.3.10 建立 watcher overflow/full-refresh test。
- [ ] 3.3.11 建立 stale refresh rejection test。

### 3.4 實作 GPUI 桌面互動與位置保存

**目的：** 交付 M0 可操作桌面、協助工具語意與跨重啟/DPI 的位置恢復。
**輸入：** 3.1 hosts、3.2 wallpaper、3.3 items、2.4 settings、2.1 reducer。
**產出：** `desktop-ui` grid/selection/focus/activation、position persistence 與 deterministic fixtures。
**依賴：** 3.1、3.2、3.3、2.4。
**Owner／Wave：** Desktop UI owner；Wave 4。
**Gate／Evidence：** `G-DESKTOP`、`G-A11Y-I18N`；`evidence/artifacts/3.4/`。
**完成門檻：** Pointer/keyboard/UIA selection 與 activation、同名 identity、restart/DPI position restore、stale refresh preservation 全數通過。

- [ ] 3.4.1 實作 item layout、hit testing、single/Ctrl/Shift selection 與 rubber-band state。
- [ ] 3.4.2 實作方向鍵 focus navigation、Enter 與雙擊 activation command。
- [ ] 3.4.3 實作僅重新定位桌面項目、不啟動檔案資料傳輸的 pointer drag。
- [ ] 3.4.4 實作 stable accessibility IDs、roles、names、states、actions 與可見 focus。
- [ ] 3.4.5 實作 monitor/identity/logical-coordinate/layout-revision position persistence。
- [ ] 3.4.6 實作 DPI/topology 變更後位置投影與 visible-bound clamp。
- [ ] 3.4.7 建立 pointer、keyboard、UIA、restart、DPI 與 overflow selection integration tests。

### 3.5 實作一般檔案 Windows Association 啟動

**目的：** 讓非資料夾桌面 Shell 項目依 Windows association 啟動，並以 bounded、exactly-once 流程回報失敗。
**輸入：** 3.3 owned Shell identity、2.1 request/generation/correlation、3.4 desktop command surface。
**產出：** `platform-win` association effect/adapter、5 秒 admission deadline、cancellation owner、GPUI failure/retry presentation 與 integration tests。
**依賴：** 2.3、3.3、3.4。
**Owner／Wave：** Windows Shell owner 與 Desktop UI owner；Wave 5，平台 adapter 路徑由 Windows Shell owner 專有，UI prompt 路徑由 Desktop UI owner 專有。
**Gate／Evidence：** `G-DESKTOP`、`G-A11Y-I18N`、`G-SAFETY`；`evidence/artifacts/3.5/`。
**完成門檻：** 真實關聯成功、無關聯失敗、取消、5 秒逾時、late callback、shutdown cleanup 與 pointer/keyboard/UIA recovery 全數通過。

- [ ] 3.5.1 定義一般 Shell item association request、terminal result 與 5 秒 monotonic deadline。
- [ ] 3.5.2 實作 owned Shell identity 的 Windows association platform adapter。
- [ ] 3.5.3 實作 cancellation owner、exactly-once terminal 與 late callback suppression。
- [ ] 3.5.4 實作 association failure/timeout 的 GPUI 錯誤與重試提示。
- [ ] 3.5.5 執行真實檔案關聯成功 integration test。
- [ ] 3.5.6 執行無關聯/adapter failure recovery integration test。
- [ ] 3.5.7 執行 cancel-vs-success 與 timeout-vs-late callback race tests。
- [ ] 3.5.8 執行 shutdown resource cleanup 與 UIA recovery prompt test。

## 4. GPUI 工作列能力

### 4.1 實作 Windows Window Tracker

**目的：** 取得符合資格且可權威對帳的真實頂層視窗資料。
**輸入：** 2.3 platform runtime、2.2 reconciliation protocol。
**產出：** Shell Hook adapter、EnumWindows snapshot、eligibility classifier、title/icon/application identity service。
**依賴：** 1.4、2.2、2.3。
**Owner／Wave：** Windows Shell owner；Wave 3。
**Gate／Evidence：** `G-TASKBAR`；`evidence/artifacts/4.1/`。
**完成門檻：** Eligible/excluded helper window matrix、lost hook、PID/HWND reuse、title/icon churn 與 reconciliation tests 通過。

- [ ] 4.1.1 實作 Shell Hook 註冊、事件轉換與解除註冊。
- [ ] 4.1.2 實作 `EnumWindows` 權威 snapshot 與 stable WindowId。
- [ ] 4.1.3 實作 invisible/tool/cloaked/owned-transient/exclusion eligibility classifier。
- [ ] 4.1.4 實作 title/icon cache、invalidation 與 bounded resource ownership。
- [ ] 4.1.5 實作 ApplicationId 解析與 unresolved fallback identity。
- [ ] 4.1.6 建立真實 helper windows 的 eligibility、reuse、lost-event 與 churn tests。

### 4.2 實作 Taskbar Model 與視窗命令

**目的：** 提供穩定 task/group 順序與 Windows 10 基本切換語意。
**輸入：** 4.1 window DTO/events、2.1 reducer。
**產出：** group/order/pin state、activate/minimize/restore/launch effects 與 platform adapter。
**依賴：** 2.1、4.1。
**Owner／Wave：** Taskbar model owner；Wave 4。
**Gate／Evidence：** `G-TASKBAR`；`evidence/artifacts/4.2/`。
**完成門檻：** 多視窗群組、標題/圖示 churn、foreground/minimize、pin launch 與失敗回復 sequence tests 通過且無非預期重排。

- [ ] 4.2.1 實作 WindowId/ApplicationId 分離的 group 與 child state。
- [ ] 4.2.2 實作穩定 group/task insertion、removal 與 pin order 規則。
- [ ] 4.2.3 實作 foreground/running/attention/minimized state transition。
- [ ] 4.2.4 實作 active-click minimize、inactive/minimized-click activate/restore effects。
- [ ] 4.2.5 實作無視窗 pinned application launch 與 terminal correlation。
- [ ] 4.2.6 建立排序不變量、群組、點擊語意與平台失敗 tests。

### 4.3 實作 Per-monitor AppBar 與 GPUI 工作列版面

**目的：** 交付預設雙列、可設定一至三列且不遮蔽一般視窗的多螢幕工作列。
**輸入：** 3.1 topology、4.2 model、2.4 settings、2.3 AppBar capability。
**產出：** AppBar lifecycle、`taskbar-ui` layout/render/hit-testing、row settings 與 multi-monitor coordination。
**依賴：** 1.4、2.3、2.4、3.1、4.2。
**Owner／Wave：** Taskbar UI owner；Wave 4。
**Gate／Evidence：** `G-TASKBAR`、`G-DPI-MONITOR`；`evidence/artifacts/4.3/`。
**完成門檻：** 1/2/3 列、主要/次要螢幕、mixed DPI、maximize work area、hot-plug 與 rollback tests 通過。

- [ ] 4.3.1 實作 per-monitor AppBar negotiate/register/update/remove lifecycle。
- [ ] 4.3.2 實作預設雙列 task layout、圖示、ellipsized title、running underline 與 active background。
- [ ] 4.3.3 實作一至三列 validation、layout 與 hit target。
- [ ] 4.3.4 實作 task overflow/available-width layout，且不宣稱完整 tray overflow。
- [ ] 4.3.5 實作 mixed-DPI/hot-plug AppBar geometry reconciliation。
- [ ] 4.3.6 執行真實 maximize/work-area、row matrix 與雙螢幕 headful tests。
- [ ] 4.3.7 實作每個主要工作列固定顯示的 SuperExplorer 入口與「本機」command binding。
- [ ] 4.3.8 執行固定入口存在、無執行中視窗、pointer/keyboard/UIA activation headful tests。
- [ ] 4.3.9 執行固定入口 launch failure 至 GPUI repair prompt integration test。

### 4.4 實作開始入口、核心狀態區與時鐘

**目的：** 提供 M0 左右工作列區域的真實可用或 truthful unavailable 狀態。
**輸入：** 2.3 Start capability、4.3 layout、系統 clock/status adapters。
**產出：** Start invocation adapter、system status DTO、clock/date UI 與 accessibility semantics。
**依賴：** 1.4、2.3、4.3。
**Owner／Wave：** Taskbar UI owner；Wave 4。
**Gate／Evidence：** `G-SHELL-TAKEOVER`、`G-A11Y-I18N`；`evidence/artifacts/4.4/`。
**完成門檻：** Windows 10 preview/Shell Start invocation、unavailable state、time/date rollover、繁中/英文及 keyboard/UIA tests 通過。

- [ ] 4.4.1 實作 Windows 10 Start host probe 與 invocation result。
- [ ] 4.4.2 實作 preview/non-reference unavailable state 與 accessible semantics。
- [ ] 4.4.3 實作 time/date provider、minute/day/time-zone change refresh。
- [ ] 4.4.4 實作網路連線與音量/靜音狀態 provider DTO。
- [ ] 4.4.5 實作輸入語言、電源/電池與通知數量 provider DTO。
- [ ] 4.4.6 實作個別 provider 的 truthful unavailable presentation。
- [ ] 4.4.7 建立 Start success/failure 與 unavailable tests。
- [ ] 4.4.8 建立 clock rollover、system-status update、keyboard 與 UIA tests。

## 5. SuperExplorer、Guardian 與 Shell 接管

### 5.1 實作 SuperExplorer Process Bridge

**目的：** 以現有程序合約可靠開啟資料夾與本機，並在失敗時保持 Shell 可操作。
**輸入：** `superexplorer-process-bridge` spec、2.1 correlation、2.4 settings。
**產出：** executable resolver、launch validator/spawner、typed terminal results 與 fake launcher。
**依賴：** 2.1、2.4。
**Owner／Wave：** Explorer bridge owner；Wave 3。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`、`G-SAFETY`；`evidence/artifacts/5.1/`。
**完成門檻：** Resolver precedence、folder/This PC environment、parent-env isolation、spawn race、missing executable、取消、5 秒逾時、late callback、handle cleanup 與 no-fallback tests 通過。

- [ ] 5.1.1 實作三段式 executable resolution 與 absolute existing-file validation。
- [ ] 5.1.2 實作 existing absolute directory validation 與 child-only `EXPLORER_INITIAL_PATH` environment。
- [ ] 5.1.3 實作「本機」無 initial-path 啟動與禁止 unsupported CLI arguments。
- [ ] 5.1.4 定義 launch admission 的 monotonic 起點、5 秒 deadline 與 cancellation owner。
- [ ] 5.1.5 實作 correlation ID、exactly-once terminal 與 late callback suppression。
- [ ] 5.1.6 實作 executable/dir invalid、spawn failure、cancelled、timed-out 與 removal race typed errors。
- [ ] 5.1.7 實作 child 成功後取消時保留外部程序並關閉本端 process/thread handles。
- [ ] 5.1.8 建立 fake child process contract tests，證明不修改 SuperExplorer repo 或父程序環境。
- [ ] 5.1.9 執行 cancel-vs-success race test 並驗證唯一 terminal/handle cleanup。
- [ ] 5.1.10 執行 timeout-vs-late callback race test 並保存 monotonic timing evidence。
- [ ] 5.1.11 執行 shutdown cancellation 與 worker rundown resource test。

### 5.2 實作 GPUI Explorer 修復提示

**目的：** 將 SuperExplorer 缺失或啟動失敗呈現為可修復且不洩漏敏感資料的產品狀態。
**輸入：** 5.1 errors、2.4 executable setting、desktop/taskbar command surface。
**產出：** GPUI recovery prompt、設定/重試 action、redacted diagnostic mapping。
**依賴：** 5.1、3.4、4.3。
**Owner／Wave：** UI integration owner；Wave 5。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`、`G-A11Y-I18N`、`G-SAFETY`；`evidence/artifacts/5.2/`。
**完成門檻：** Missing/invalid/spawn-failed prompt 能以 pointer/keyboard/UIA 修復，桌面/工作列保持回應，且一般 log 不含完整 profile path。

- [ ] 5.2.1 建立失敗類型至繁中/英文訊息與 action 的 mapping。
- [ ] 5.2.2 實作可選擇執行檔、取消與重試的 GPUI prompt。
- [ ] 5.2.3 實作 prompt focus trap、keyboard flow、accessible role/name/action。
- [ ] 5.2.4 實作路徑 redaction 與本機 debug opt-in boundary。
- [ ] 5.2.5 執行 missing/removal/spawn failure 的 headful recovery tests。

### 5.3 實作 Guardian Lease 與冪等復原

**目的：** 在主程序 crash 後獨立恢復 Explorer、work area 與唯一 terminal 狀態。
**輸入：** 2.3 capability/runtime、AppBar lifecycle、Windows process APIs。
**產出：** `superdesktop-guardian` binary、不可偽造 inherited lease protocol、非授權 recovery journal、安全 Explorer target resolver 與 recovery adapter。
**依賴：** 2.3、4.3。
**Owner／Wave：** Lifecycle owner；Wave 5。
**Gate／Evidence：** `G-GUARDIAN-RECOVERY`、`G-SAFETY`；`evidence/artifacts/5.3/`。
**完成門檻：** Normal release、10-run timed main crash、guardian retry、existing/missing Explorer、forged/stale lease、PATH/CWD/reparse substitution、wrong-session/token 與 handle inheritance tests 均達到規範終態，且不作用於其他 session/target。

- [ ] 5.3.1 定義 inherited process handle、one-time channel、PID/creation-time/session/file-identity/nonce lease protocol。
- [ ] 5.3.2 定義不具授權效力的 audit journal、terminal protocol 與 monotonic recovery timestamps。
- [ ] 5.3.3 實作 guardian 對主程序正常結束與異常 process-handle signal 的辨識。
- [ ] 5.3.4 實作 lease identity、interactive session/token 與 owner target validation。
- [ ] 5.3.5 實作遺留 AppBar/work-area 的 owner-fenced 修復。
- [ ] 5.3.6 實作 Windows directory 解析、canonicalization、Microsoft Explorer 驗證與 explicit application name。
- [ ] 5.3.7 實作受控 working directory/environment 與 explicit inherited-handle list 的 Explorer spawn。
- [ ] 5.3.8 實作顯示既有 Explorer 或在缺失時啟動單一 Explorer Shell。
- [ ] 5.3.9 實作重複復原、stale lease 與 concurrent guardian 的冪等控制。
- [ ] 5.3.10 執行 owned-session helper 的正常 lease release test。
- [ ] 5.3.11 執行 10 次 forced-crash reference run，保存每次 T0、Explorer-ready、work-area 與 identity raw evidence。
- [ ] 5.3.12 驗證 10 次 forced-crash run 均在 T0 後 10 秒內可接受 pointer/keyboard 輸入。
- [ ] 5.3.13 執行 Explorer 缺失時單一安全啟動 test。
- [ ] 5.3.14 執行重複 recovery 與 concurrent guardian 冪等 test。
- [ ] 5.3.15 執行 forged nonce/PID creation time/file identity 與 stale journal rejection tests。
- [ ] 5.3.16 執行 PATH/CWD/reparse explorer substitution rejection tests。
- [ ] 5.3.17 執行 wrong-session/token 與過度 handle inheritance rejection tests。

### 5.4 實作交易式 Shell Takeover Coordinator

**目的：** 只有在所有必要能力與表面健康時才切換 Shell，並對每階段失敗完整 unwind。
**輸入：** 2.3 probes、3.1 desktop hosts、4.3 AppBars、4.4 Start probe、5.3 guardian。
**產出：** session-scoped owner lease、takeover state machine、failpoints、health check、Explorer surface switch/restore adapter。
**依賴：** 3.4、4.4、5.3。
**Owner／Wave：** Primary integrator；Wave 5。
**Gate／Evidence：** `G-SHELL-TAKEOVER`、`G-GUARDIAN-RECOVERY`；`evidence/artifacts/5.4/`。
**完成門檻：** 單一 owner fencing、同時接管、owner crash/transfer、non-owner cleanup rejection、每個前置階段 failpoint、成功接管、正常退出、強制終止與重複 cleanup 在 Windows 10 reference session 全數達到規範終態。

- [ ] 5.4.1 實作 preview/shell mode command-line parsing 與禁止 registry mutation guard。
- [ ] 5.4.2 實作 AppBar/Explorer mutation 前的 session-scoped 原子 owner lease 與 fencing token。
- [ ] 5.4.3 實作 non-owner mutation/cleanup rejection 與 owner identity revalidation。
- [ ] 5.4.4 實作六階段 takeover state machine 與每階段 timeout/failpoint。
- [ ] 5.4.5 實作 desktop/taskbar input health check 與 required capability gate。
- [ ] 5.4.6 實作 Explorer surface session switch 與 normal restore adapter。
- [ ] 5.4.7 執行同一 session 兩個主程序 simultaneous takeover test。
- [ ] 5.4.8 執行 owner crash、guardian recovery 與新 owner lease transfer test。
- [ ] 5.4.9 執行 non-owner cleanup rejection test。
- [ ] 5.4.10 執行每階段失敗注入並驗證 Explorer/work area 未受破壞。
- [ ] 5.4.11 執行 Windows 10 reference 的成功 Shell 接管 test。
- [ ] 5.4.12 執行 Windows 10 reference 的正常退出與 work-area restore test。
- [ ] 5.4.13 執行 Windows 10 reference 的 forced-crash guardian recovery test，引用 5.3.11–5.3.12 timing evidence。

## 6. 組合、協助工具、視覺與效能驗證

### 6.1 組合 Preview 與 Shell 應用程式

**目的：** 以單一 composition root 連接 reducer、platform、desktop、taskbar、settings、bridge 與 guardian。
**輸入：** 2–5 階段所有 production contracts。
**產出：** `superdesktop-app` startup/shutdown composition、diagnostics、panic hook 與 visual fixture modes。
**依賴：** 3.4、4.4、5.2、5.4。
**Owner／Wave：** Primary integrator；Wave 6。
**Gate／Evidence：** `G-ARCH`、`G-SHELL-TAKEOVER`、`G-TRACE`；`evidence/artifacts/6.1/`。
**完成門檻：** Preview 與 Shell binary routes 使用相同 core contracts、startup failure 有序 unwind、normal close 無 owned resource 增長，且 production 不使用 fake service。

- [ ] 6.1.1 組合 diagnostics、settings、core dispatcher、platform endpoints 與 GPUI app lifecycle。
- [ ] 6.1.2 組合 desktop/taskbar surfaces 與 SuperExplorer recovery prompt。
- [ ] 6.1.3 組合 preview mode，證明不修改 Explorer/work area/registry。
- [ ] 6.1.4 組合 shell mode 與 guardian/takeover coordinator。
- [ ] 6.1.5 實作 panic hook、controlled failure UI 與 ordered shutdown telemetry。
- [ ] 6.1.6 執行 production composition 不使用 fake service 的 architecture test。
- [ ] 6.1.7 執行 startup-failure ordered unwind test。
- [ ] 6.1.8 執行 repeated startup/shutdown lifecycle resource test。

### 6.2 完成協助工具、繁中/英文與高對比

**目的：** 確保 M0 全部可操作控制能以鍵盤與 UI Automation 使用，且在必要語系/主題下不失真。
**輸入：** 3.4 desktop UI、4.3/4.4 taskbar UI、5.2 prompt。
**產出：** semantic tokens、localization resources、AccessKit/UIA contract 與 headful evidence。
**依賴：** 6.1。
**Owner／Wave：** Accessibility/localization owner；Wave 6。
**Gate／Evidence：** `G-A11Y-I18N`；`evidence/artifacts/6.2/`。
**完成門檻：** Keyboard-only、stable role/name/state/action、visible focus、高對比、繁中/英文及 IME matrix 全數通過。

- [ ] 6.2.1 建立繁體中文與英文 resource catalog，移除 M0 可見硬編碼字串。
- [ ] 6.2.2 建立 light/dark/high-contrast semantic token mapping。
- [ ] 6.2.3 稽核所有控制的 stable accessibility identity/role/name/state/action。
- [ ] 6.2.4 執行 desktop/taskbar/prompt keyboard-only traversal 與 activation tests。
- [ ] 6.2.5 執行 UI Automation/AccessKit contract 與 visible-focus tests。
- [ ] 6.2.6 執行繁體中文 headful layout 與 interaction matrix。
- [ ] 6.2.7 執行英文 headful layout 與 interaction matrix。
- [ ] 6.2.8 執行 high-contrast semantic-color 與 focus matrix。
- [ ] 6.2.9 執行 IME 不破壞 desktop/taskbar 操作的 headful matrix。
- [ ] 6.2.10 執行簡體中文字形、截斷與 fallback headful matrix。
- [ ] 6.2.11 執行 RTL/bidi desktop/taskbar/prompt geometry 與 interaction matrix。

### 6.3 完成 DPI、多螢幕與視覺 Gate

**目的：** 證明桌面與雙列工作列在必要 DPI/topology 下具有穩定 geometry、hit target 與 Windows 10 參考視覺。
**輸入：** 6.1 integrated app、6.2 tokens/resources。
**產出：** deterministic visual fixtures、reference captures、geometry reports 與 diff index。
**依賴：** 6.1、6.2。
**Owner／Wave：** Visual verification owner；Wave 7。
**Gate／Evidence：** `G-DPI-MONITOR`、`G-DESKTOP`、`G-TASKBAR`；`evidence/artifacts/6.3/`。
**完成門檻：** 100/125/150/175/200% 與 mixed-DPI 雙螢幕所有必測狀態有 hash/metadata，無重疊、裁切、錯誤 work area 或不可點擊控制。

- [ ] 6.3.1 建立 deterministic desktop/taskbar/prompt visual fixture states。
- [ ] 6.3.2 擷取並驗證 100% DPI visual/geometry matrix。
- [ ] 6.3.3 擷取並驗證 125% DPI visual/geometry matrix。
- [ ] 6.3.4 擷取並驗證 150% DPI visual/geometry matrix。
- [ ] 6.3.5 擷取並驗證 175% DPI visual/geometry matrix。
- [ ] 6.3.6 擷取並驗證 200% DPI visual/geometry matrix。
- [ ] 6.3.7 擷取 mixed-DPI 雙螢幕 matrix。
- [ ] 6.3.8 執行 display hot-plug matrix。
- [ ] 6.3.9 執行 monitor topology reorder matrix。
- [ ] 6.3.10 量測 taskbar rows、文字截斷與 hit targets。
- [ ] 6.3.11 量測 desktop bounds 與 AppBar work area。
- [ ] 6.3.12 建立 Windows 10 22H2 reference profile metadata 與可重現 captures。
- [ ] 6.3.13 執行 Windows 11 application launch 與 desktop/taskbar interaction compatibility test。
- [ ] 6.3.14 執行 Windows 11 normal-exit Explorer/work-area recovery test。
- [ ] 6.3.15 執行 Windows 11 forced-crash guardian recovery test，且不替代 Windows 10 gate。

### 6.4 完成效能、壓力與資源 Gate

**目的：** 以原始樣本驗證效能 threshold、事件風暴復原與長時間資源穩定性。
**輸入：** 6.1 integrated app、2.2 queue telemetry、platform resource snapshots。
**產出：** benchmark runner、raw samples、environment metadata、stress/soak reports。
**依賴：** 6.1、6.3。
**Owner／Wave：** Performance owner；Wave 7。
**Gate／Evidence：** `G-PERF`、`G-SAFETY`；`evidence/artifacts/6.4/`。
**完成門檻：** 冷啟動 ≤2s、idle CPU median <0.5%、event-to-visible p95 <100ms、working set <150MiB，且 event storm/soak 後狀態權威、queue bounded、資源無持續成長。

- [ ] 6.4.1 建立 reference machine/build/environment metadata collector。
- [ ] 6.4.2 執行冷啟動多次原始樣本並計算 threshold disposition。
- [ ] 6.4.3 執行穩定後 idle CPU 多次原始樣本並計算 median。
- [ ] 6.4.4 執行 Shell event-to-visible frame 原始樣本並計算 p95。
- [ ] 6.4.5 執行 working set resource soak 與 threshold disposition。
- [ ] 6.4.6 執行 thread/handle resource soak 與 leak disposition。
- [ ] 6.4.7 執行 GDI/User object resource soak 與 leak disposition。
- [ ] 6.4.8 執行 wallpaper/icon/cache budget soak 與 disposition。
- [ ] 6.4.9 執行 window/title/icon 與 desktop rename/create/delete event storms。
- [ ] 6.4.10 驗證 overflow 後 reconciliation 收斂且保存 raw queue telemetry。

## 7. 最終 Gate、追溯與交付審查

### 7.1 執行完整品質與安全 Gate

**目的：** 在相同 revision 上完成格式、編譯、lint、測試、架構、授權與安全驗證。
**輸入：** 完整 production/test workspace 與 1.1 evidence tooling。
**產出：** 全部 gate raw logs、exit status、binary/lock hashes 與安全稽核報告。
**依賴：** 6.2、6.3、6.4。
**Owner／Wave：** Primary integrator；Wave 8。
**Gate／Evidence：** `G-ARCH`、`G-SAFETY`、`G-TRACE`；`evidence/artifacts/7.1/`。
**完成門檻：** 每個必要命令獨立通過；license/source review 無未核准衍生碼；diagnostic redaction 與 fixture escape 負面測試通過。

- [ ] 7.1.1 執行 `cargo fmt --all -- --check` 並保存完整結果。
- [ ] 7.1.2 執行 workspace Windows target `cargo check --locked` 並保存完整結果。
- [ ] 7.1.3 執行 workspace all-target clippy warnings-as-errors 並保存完整結果。
- [ ] 7.1.4 執行 workspace tests `--locked` 並保存完整結果。
- [ ] 7.1.5 執行 architecture checker 正負 gate 並保存完整結果。
- [ ] 7.1.6 執行 dependency source/hash audit。
- [ ] 7.1.7 執行 dependency license audit 並由 reviewer 簽核。
- [ ] 7.1.8 執行 PExplorer/SuperExplorer source-boundary audit 並由 reviewer 簽核。
- [ ] 7.1.9 執行 diagnostic secret 與完整路徑洩漏 scan。
- [ ] 7.1.10 執行 destructive fixture escape 與 reparse-point 負面測試。

### 7.2 驗證 Requirement、Task 與 Evidence 追溯

**目的：** 證明所有 proposal/design/spec 承諾都有 scenario、leaf、gate 與有效 evidence。
**輸入：** OpenSpec artifacts、tasks、evidence index、adjustment ledger 與所有 gate reports。
**產出：** `evidence/traceability.md`、機器可讀 trace map 與 stale/orphan report。
**依賴：** 7.1。
**Owner／Wave：** Verification owner；Wave 8。
**Gate／Evidence：** `G-TRACE`；`evidence/artifacts/7.2/`。
**完成門檻：** 無缺少 scenario/task/evidence 的 requirement、無孤兒完成 leaf、無 stale completion、無未處置 B/C adjustment。

- [ ] 7.2.1 建立 capability/requirement/scenario/gate/task/evidence trace map。
- [ ] 7.2.2 驗證每個勾選 leaf 都有唯一有效 task_id 或 shared subcheck。
- [ ] 7.2.3 驗證每個 blocking gate 都有 raw evidence 與明確 disposition。
- [ ] 7.2.4 驗證所有 A/B/C adjustment 已依規則處置且 lineage 完整。
- [ ] 7.2.5 掃描 stale、orphan、missing hash、missing reviewer 與 placeholder。
- [ ] 7.2.6 產生最終 traceability report 並保存 validator 輸出。

### 7.3 進行獨立架構與發行審查

**目的：** 在宣稱 M0 完成前由未負責主要實作的人檢查架構、安全、生命週期與測試完整性。
**輸入：** 所有 artifacts、程式碼、gate reports、traceability report 與已知差異。
**產出：** `evidence/final-review.md`、finding disposition、M0 handoff 與 parity matrix。
**依賴：** 7.2。
**Owner／Wave：** Independent reviewer 負責審查與簽核；Primary integrator 協調原 implementation/gate owners 修正並重跑；Wave 9。
**Gate／Evidence：** 全部 blocking gates；`evidence/artifacts/7.3/`。
**完成門檻：** 無未解決 P0/P1；P2/P3 有明確 disposition；所有 M0 acceptance criteria 有證據；延後能力在 parity matrix 中如實標示。

- [ ] 7.3.1 由獨立 reviewer 審查 ABI/type boundary、COM/threading、lifecycle 與 recovery。
- [ ] 7.3.2 由獨立 reviewer 審查安全、授權、診斷隱私與 destructive target guards。
- [ ] 7.3.3 由獨立 reviewer 審查 requirement/scenario、測試矩陣、gate 與 leaf atomicity。
- [ ] 7.3.4 Primary integrator 將 P0/P1 finding 分派給原 implementation owner，重開受影響 task 並使相依證據 stale。
- [ ] 7.3.5 原 gate owner 重跑受影響 gate 並保存 replacement evidence lineage。
- [ ] 7.3.6 完成 Windows 10 M0 parity matrix、已知差異與後續 change 邊界。
- [ ] 7.3.7 簽核 final review，確認所有 apply-required task 與 blocking gate 均達有效終態。
