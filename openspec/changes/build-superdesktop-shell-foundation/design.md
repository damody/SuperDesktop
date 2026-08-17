## Context

SuperDesktop 是全新 Windows-only Rust 專案。原始設計位於 `docs/superpowers/specs/2026-08-13-superdesktop-windows-10-shell-design.md`；核准的 `C-W11-REFERENCE-001` 修正以 `docs/superpowers/specs/2026-08-17-superdesktop-windows11-reference-release-design.md` 覆蓋其 release platform。M0 實作全 GPUI 桌面、預設雙列工作列、SuperExplorer 程序橋接、交易式 Shell 接管、guardian 復原，以及 Windows 11 reference-profile 測試與證據架構。

現有 `D:\SuperDesktop\PExplorer` 是 LGPL C++/Win32 參考實作，只能用於觀察行為與 API 流程。`D:\SuperExplorer` 已是大型 Rust/GPUI 專案且工作樹含有使用者變更，因此本 change 不修改也不以 path dependency 連結它。M0 的 UI/互動與 release 基準凍結為 Windows 11 build 26200.9168 + ExplorerPatcher 26100.8457.70.3；參考圖 SHA-256 為 `48B5F990B9E155C5C2719D8F8B41D88ED4420A46C3B6018278511F9C349B387E`。Windows 10 compatibility 為 not-claimed。

本 change 會影響目前使用者工作階段的桌面與工作區，因此 Shell 接管、復原、測試資料邊界與證據完整性是 blocking gate，而不是上線後才補的品質工作。

## Goals / Non-Goals

**Goals:**

- 建立獨立 Cargo workspace 與清楚 crate 邊界，讓核心狀態機可在無 Win32/GPUI 的情況下測試。
- 以 GPUI 繪製所有 SuperDesktop 擁有的可見表面。
- 建立安全預覽模式與可逆、可觀測的交易式 Shell 模式。
- 建立每螢幕桌面及一至三列工作列，M0 預設為使用者參考圖所示的雙列高密度配置。
- 透過既有 `EXPLORER_INITIAL_PATH` 合約啟動 SuperExplorer，而不修改其儲存庫。
- 對 stale event、queue overflow、設定損毀、程序 crash 與平台能力缺失提供明確復原。
- 建立從 requirement、scenario、gate、task 到證據紀錄的可追溯驗證鏈。

**Non-Goals:**

- 不在 M0 實作自訂開始功能表/搜尋、完整第三方通知區、跳躍清單、即時縮圖、桌面重新命名/拖放/原生右鍵選單、虛擬桌面或登入 Shell 登錄安裝。
- 不重寫 Windows 核心、DWM、登入畫面、系統設定或內建應用程式。
- 不複製或機械式翻譯 PExplorer 原始碼。
- 不新增 SuperExplorer IPC，也不修改 SuperExplorer 的程式碼、建置或工作樹。
- 不宣稱 ExplorerPatcher reference profile 等同任何未公開 Explorer 像素或內部行為，也不宣稱 Windows 10 compatibility。

## Decisions

### 1. 使用獨立 workspace 與單向依賴

workspace 建立 `superdesktop-app`、`shell-core`、`platform-win`、`desktop-ui`、`taskbar-ui`、`explorer-bridge`、`settings-store`、`superdesktop-guardian` 與 `superdesktop-test-support`。UI 只依賴 `shell-core` 的 owned value 與 command；HWND、PIDL、COM interface 只能存在於 `platform-win`。composition root 位於 `superdesktop-app`。

此方案優於單一 crate，因為 Shell/COM apartment affinity、GPUI rendering 與純狀態轉移具有不同失敗與測試模型。它也優於直接重用 SuperExplorer crate，因為可避免其髒工作樹與內部 API 成為 SuperDesktop 的隱含公開合約。

### 2. 所有產品可見介面使用 GPUI

桌面、工作列、錯誤提示與復原提示皆使用 GPUI。Windows 原生開始體驗與日後的 Shell context menu 屬於外部提供的表面，不是 SuperDesktop UI。`platform-win` 可建立 message-only HWND、AppBar HWND 或平台後端所需原生視窗，但不得以 GDI/Win32 control 組裝產品介面。

替代方案「全 Win32」較容易接上 Shell API，但無法符合使用者指定的全 GPUI；「完全不使用 Win32」則無法可靠完成 AppBar、Shell Hook、COM/OLE 與 Shell 接管。

### 3. 單一權威狀態快照與型別化 effect

`shell-core` 擁有 monitor、desktop、taskbar、selection、focus、application identity、settings revision 與 recovery phase 的單一邏輯快照。Windows callback 只能送出型別化 event；GPUI 操作只能送出型別化 command。effect 執行結果帶 `request_id` 與 `generation` 回到 reducer。generation 不符的結果不得修改狀態。

高頻事件先依穩定 identity coalesce。queue 有明確容量與 overflow event；overflow 後使用權威來源重新對帳，而不是嘗試猜測遺失事件。

### 4. 預覽模式預設安全，Shell 模式交易式接管

預覽模式不隱藏 Explorer、不設定 AppBar 工作區、不註冊工作階段級 Shell 所有權，也不修改 Shell 登錄值。Shell 模式依序完成 guardian lease、診斷/COM/GPUI、所有螢幕表面、AppBar、Hook/Hotkey、開始主機能力探測及互動健康檢查，最後才切換 Explorer 表面。

接管前失敗只清理 SuperDesktop 自己的資源。接管後異常由 guardian 執行 idempotent recovery：解除 AppBar、恢復 work area、顯示可用 Explorer 或在缺失時啟動 `explorer.exe`。M0 不修改登入 Shell 登錄值。

**Blocking gate `G-SHELL-TAKEOVER`：** Wave 5 必須在凍結 Windows 11＋ExplorerPatcher reference profile 完成每階段失敗注入、正常退出與重複復原並產生 provisional disposition；Wave 6 使用同一 exact profile、candidate 與 binaries 執行啟動、核心互動、正常退出、forced-crash 與 installer reboot/rollback confirmation，通過後 final gate 才可標為 passed。

**Blocking gate `G-GUARDIAN-RECOVERY`：** 以 guardian 所持主程序 handle 變成 signaled 的 monotonic timestamp 為 T0；在凍結 ExplorerPatcher reference profile 進行 10 次獨立 forced-crash run，每次都必須在 T0 後 10 秒內恢復可接受 pointer/keyboard 輸入的 Explorer Shell 與正確 work area，並產生唯一 terminal result。必須保存每次 T0、Explorer-ready timestamp、work-area snapshot 與程序 identity；任一 run 超時或失敗即使 gate 失敗，並禁止發行 Shell 模式。

Guardian 的控制依據不得來自可被替換的 journal 或 PATH 搜尋。主程序啟動 guardian 時必須以明確 handle inheritance list 傳入 process handle 與 one-time lease channel，並以 PID、process creation time、session ID、主程式 file identity 及 nonce fencing 綁定 owner。journal 只保存稽核資料，不能授權復原。啟動 Explorer 時使用由 Windows directory API 解析、canonicalize 並驗證為 Microsoft Windows Explorer 的絕對路徑，對程序建立 API 提供 explicit application name，限制 inherited handles，使用目前互動使用者 token 與受控 working directory/environment。錯誤 session、偽造/陳舊 lease、PATH/CWD substitution 或不符 file identity 的 target 必須拒絕。

同一 Windows session 在任何 AppBar 或 Explorer surface mutation 前，只能取得一個 session-scoped 原子 Shell owner lease。lease 以 PID、creation time、session ID 與 nonce fencing；非 owner 不得清理 owner 的 AppBar 或切換 Explorer。owner crash 後只有完成 identity validation 的 guardian 可以復原或將 lease 交給新 owner。

### 5. 桌面使用 Shell identity，不以顯示名稱作為 identity

桌面模型合併 User Desktop 與 Public Desktop known folders。platform adapter 解析 owned Shell identity、display name、icon descriptor、location 與 capability。圖示位置以 `{monitor_identity, shell_item_identity, logical_position, layout_revision}` 儲存，DPI 變更時重新投影並 clamp 到可見工作區。

M0 支援選取、Ctrl/Shift 複選、框選、鍵盤導覽、Enter/雙擊啟動、桌布模式及 watcher overflow refresh。重新命名、拖放、context menu、排序及 Recycle Bin mutation 保留為後續 change，UI 不得顯示為可用。

### 6. 工作列以穩定 application/window identity 建模

`platform-win` 透過 Shell Hook 取得增量事件，並以週期性 `EnumWindows` 作權威對帳。資格規則排除 invisible、tool、cloaked、owned transient 與明確排除視窗。模型分離 `WindowId` 與 `ApplicationId`，以便穩定分組又保留個別視窗狀態。

按鈕順序只有在使用者釘選/重排或應用程式群組首次加入/最後移除時改變；標題、圖示、foreground、attention 或 minimize 更新不得造成重排。點擊 active window 會最小化，點擊 minimized/非 active window 會還原並啟用，點擊無視窗的 pinned app 會啟動它。

工作列是 per-monitor model；M0 必須完成主要與次要螢幕 AppBar。預設兩列，可設定一至三列。完整 notification compatibility 與進階 taskbar 功能不在本 change。

### 7. SuperExplorer 只使用既有程序合約

resolver 順序為：使用者設定的絕對執行檔、`D:\SuperExplorer\target\release\SuperExplorer.exe`、與 SuperDesktop 相鄰的 `SuperExplorer.exe`。候選必須存在且為檔案。檔案系統資料夾必須是存在的絕對目錄，並以 child process environment 的 `EXPLORER_INITIAL_PATH` 傳入；不傳入 SuperExplorer 不支援的 CLI 參數。固定入口名稱為「SuperExplorer」，未設定該變數時不得宣稱已導覽至「本機」。

每個 launch request 產生 correlation ID，並恰好收到一個 terminal event。缺失、無效或 spawn failure 會顯示 GPUI recovery prompt，且不得靜默 fallback 至 Windows Explorer。

Launch admission 從 dispatcher 接受 request 的 monotonic timestamp 起算，5 秒內必須得到 `launched`、`validation-failed`、`spawn-failed`、`cancelled` 或 `timed-out` 其中一個 terminal event。Shutdown 或使用者取消由 `explorer-bridge` cancellation owner 發出；若 child process 已成功建立，取消不強制終止外部 SuperExplorer，但必須關閉本端 process/thread handle 並保持第一個 terminal result 權威。逾時後到達的 spawn/callback 只能成為 late diagnostic，不得反轉 UI 結果。

一般桌面檔案與 shortcut 不經 `explorer-bridge`；`platform-win` 使用 owned Shell identity 與 Windows association adapter 執行。該 request 也受 request/generation、5 秒 admission deadline、取消與 exactly-once terminal 約束。成功交由 Windows association host 後即完成；失敗必須回到 GPUI 可復原錯誤，且不阻塞桌面。

### 8. 設定採版本化 schema、原子取代與隔離復原

設定包括 execution-mode preference、taskbar row count、pin order、wallpaper mode、desktop coordinates、monitor mapping、SuperExplorer path、theme 與 accessibility preference。execution-mode preference 只影響提示或明確使用者選擇，不得讓未帶 Shell mode 明確參數的啟動自動接管 Shell。寫入採暫存檔、flush、驗證後原子取代。讀取時能安全降級的單欄錯誤只回復該欄；結構或解析損毀則重新命名為 timestamped quarantine，記錄診斷並使用安全預設值。

設定檔不得儲存不必要的敏感資料。診斷預設 redact 完整使用者路徑、檔案內容、clipboard 與 credential。

### 9. 能力探測決定是否允許接管，不偽造成功

凍結 ExplorerPatcher reference profile 的 Shell 模式在接管前探測目前 Start host、AppBar、Shell Hook、monitor/DPI 及 guardian recovery prerequisites。必要能力缺失時拒絕接管並停留於可用 Explorer。其他相容 profile 可以將相依控制顯示為 accessibly unavailable，但不得宣稱該能力完成。

在 production implementation 前執行 blocking capability spike：以候選固定 GPUI-CE revision，在凍結的 Windows 11＋ExplorerPatcher reference profile 建立最小 GPUI HWND、驗證 native HWND/message bridge、AppBar reserve/restore、per-monitor DPI/topology、Shell Hook、ExplorerPatcher Start host probe/invocation 及 guardian process-handle lease。Spike 必須產生可重現 source revision、binary hash、OS build、raw result 與 resource snapshot；任一必要能力失敗時停止所有相依 work package，依 B 級修正設計/規格/任務或依 C 級取得框架/範圍變更核准。完整啟動、互動、回復與 installer reboot/rollback 由 release verification change 在同一 exact profile 驗證。

所有 `extern "system"`/FFI callback 都是 no-unwind boundary。workspace release profile 保持可捕捉的 unwind policy；callback wrapper 使用 `catch_unwind` 將 panic 轉為 typed fatal event，進入有序停止或 guardian recovery，且不得讓 Rust unwind 穿越 Win32 ABI。Callback 的 input validation、handle ownership、重複 callback 與 shutdown race 都必須有負面測試。

### 10. 驗證與證據是交付物

每個 child change 在自己的 `evidence/index.jsonl` 保存 append-only records；program change 只彙總各 child index 的 immutable hash 與 archive revision。每個 atomic task 使用全域唯一 `<change-name>/<L3-id>` `task_id`，或共用 immutable record 加唯一 `subcheck`。紀錄包含 artifact/command/manual procedure、expected、actual、exit status 或 reviewer、hash、gate、adjustment ID 與 timestamp；大型原始證據放在該 change 的 `evidence/artifacts/<L2-or-task-id>/`，索引只保存相對路徑與 hash。本 tasks.md 內所有 leaf 都是 M0 mandatory leaf，不允許以 `not-applicable` 結案；若日後新增 conditional leaf，必須事前在 task 文字與 schema 中定義客觀 eligibility、替代 coverage 與 gate disposition。`superseded` replacement 必須存在、無循環、仍為 mandatory、trace 至相同 requirement/scenario/gate，且 replacement 已有有效 `passed` evidence；在此前原 leaf 不得勾選完成。取消 mandatory leaf、將它改為 optional 或降低 coverage 屬於 C 級變更。

Blocking gates：

- `G-ARCH`：crate 邊界、Windows-only target、固定依賴與授權稽核。
- `G-SHELL-TAKEOVER`：交易式接管、失敗 unwind 與正常復原。
- `G-GUARDIAN-RECOVERY`：主程序 crash 後工作階段可操作。
- `G-DESKTOP`：桌面互動、identity、watcher overflow 與持久化。
- `G-TASKBAR`：真實視窗追蹤、穩定排序、群組與多螢幕 AppBar。
- `G-EXPLORER-BRIDGE`：既有合約、失敗提示與 exactly-once terminal。
- `G-A11Y-I18N`：鍵盤、UIA/AccessKit、焦點、高對比、繁中與英文。
- `G-DPI-MONITOR`：100/125/150/175/200% 及混合 DPI 雙螢幕。
- `G-PERF`：冷啟動 ≤ 2 秒、idle CPU median < 0.5%、event-to-visible p95 < 100 ms、working set < 150 MiB。
- `G-SAFETY`：測試/復原只改動受控 fixture root，診斷不洩漏敏感資料。
- `G-TRACE`：proposal、design、scenario、task、gate 與 evidence 完整追溯。

### 11. 實作期間的調整分級

- **A — 任務微調：** 可以調整 leaf 拆分、順序、owner、命令或證據收集方式，但不得改變範圍、requirement、gate、threshold 或公開合約。必須在 `evidence/adjustments.md` 記錄。
- **B — 設計/規格修正：** 在已核准範圍內修正錯誤假設。必須暫停受影響 work package，同步更新 design/spec/tasks，使相依證據標記 stale，保留舊證據 lineage，重新驗證後才能繼續。
- **C — 實質變更：** 改變範圍、公開承諾、blocking gate、threshold、必要證據、平台/框架、權限、外部寫入或破壞性操作。必須先取得使用者核准。

任何分級都不得靜默降低 blocking gate。實作證據若推翻假設，只能依上述流程更正，不得以「實作困難」刪除 requirement。

## Risks / Trade-offs

- **[GPUI 無法直接涵蓋部分 Shell HWND 行為]** → 在 `platform-win` 建立最薄原生 adapter；先以 capability spike 與 headful contract gate 驗證 HWND/AppBar/monitor 能力。
- **[接管失敗造成無工作列或錯誤 work area]** → 交易式啟動、獨立 guardian、每階段 failpoint、idempotent recovery 與 blocking 真機證據。
- **[ExplorerPatcher/Windows 更新造成參考漂移]** → 凍結 OS build、ExplorerPatcher version、設定摘要與參考圖 hash；不同 profile 必須重新建立 baseline，不得靜默覆蓋。
- **[Shell Hook 遺失或事件風暴造成錯誤 task state]** → bounded queue、overflow event、coalescing、週期性 `EnumWindows` reconciliation。
- **[桌面 watcher overflow 或 rename storm]** → full namespace refresh、stable identity selection restore、generation rejection。
- **[第三方或 Shell provider 卡住 UI]** → M0 不啟用未隔離的第三方 tray/context provider；後續能力必須先有 bounded worker/process host。
- **[SuperExplorer 合約能力不足]** → M0 只建立新程序並使用已存在的環境變數合約；既有程序導覽另開協調 change。
- **[PExplorer 授權污染]** → 禁止複製與機械翻譯，保存研究紀錄、依賴與授權稽核證據。
- **[全 GPUI 與高密度兩列配置的效能壓力]** → bounded cache、事件 coalescing、visual/performance fixture 與不可降低的 `G-PERF`。
- **[真機 gate 受硬體或環境限制]** → 允許證據顯示 blocked，但不得把 blocked leaf 標為完成或宣稱 apply complete。

## Migration Plan

1. 建立 workspace、固定候選 toolchain/GPUI-CE、產品 identity、診斷、架構 gate 與 evidence schema。
2. 完成 blocking capability spike；通過後才允許相依 production work package 開始。
3. 實作 `shell-core` typed state、generation、bounded event/coalescing 與 fake platform contracts。
4. 實作 preview composition、desktop/taskbar GPUI surface 與 deterministic fixtures。
5. 實作 Windows monitor/DPI、desktop namespace/watcher、window tracking/AppBar/Start probe。
6. 實作 SuperExplorer bridge、Windows association adapter、settings persistence 與 GPUI recovery prompt。
7. 實作 guardian、session owner lease、交易式 Shell takeover、FFI panic boundary、failpoint、normal/crash rollback。
8. 完成真機、DPI、雙螢幕、a11y/i18n、stress、performance、license、安全與 traceability gate。

本 change 不部署登入 Shell。rollback 是停止 SuperDesktop、由 guardian 恢復 Explorer/work area，並保留診斷與隔離設定。任何 installer 或 registry-based migration 必須由後續 change 規範。

## Program Change 分解與依賴

本 change 只作 program coordination，不直接承載 200 多個 production leaf。實作分成八個 apply-ready change：

| 順序 | Change | 完成結果 | 依賴 |
| --- | --- | --- | --- |
| 1 | `bootstrap-superdesktop-workspace` | Workspace、架構、證據與離線建置基礎 | 無 |
| 2 | `validate-superdesktop-windows-platform` | GPUI/Win32 capability spike 與 go/stop | 1 |
| 3 | `build-superdesktop-shell-core` | reducer、queue/reconciliation、設定 | 1、2 |
| 4A | `build-superdesktop-gpui-desktop` | GPUI 桌面與一般檔案 association | 2、3 |
| 4B | `build-superdesktop-gpui-taskbar` | 多螢幕雙列工作列與固定 SuperExplorer 入口 | 2、3 |
| 4C | `integrate-superexplorer-process-bridge` | 既有程序合約與修復提示 | 2、3 |
| 5 | `add-superdesktop-shell-takeover-recovery` | 單一 owner、交易式接管、guardian | 4A、4B、4C |
| 6 | `verify-superdesktop-m0` | 跨 OS/硬體/效能/安全與最終追溯 | 全部 |

4A、4B、4C 可平行執行，但 contract 變更必須先回到 `build-superdesktop-shell-core` 的 contract owner。每個 child change 自行通過 strict validation 與獨立 gate；program change 只在全部 child change 完成、archive 且最終證據通過時結案。

多代理執行採 `EXECUTION.md` 的固定 ownership 與交接契約。A 級技術細化由 task owner 自主處理；B 級矛盾由 Primary integrator 同步修正 design/spec/tasks、標 stale 並建立 lineage 後繼續；只有 C 級範圍、blocking gate、平台、安全/權限或外部授權變更需要使用者核准。這避免 apply 過程把一般工程判斷反覆升級成使用者確認，同時保留已核准邊界。

目前開發機就是 UI/互動與 lifecycle reference environment：Windows 11 build 26200.9168、ExplorerPatcher 26100.8457.70.3、單一使用中螢幕。虛擬顯示器可完成自動化 topology gate；真實 mixed-DPI 雙螢幕仍是 release-candidate confirmation。缺少外部環境時 production changes 仍可完成，但 confirmation leaf 保持 blocked，不能使用 `not-applicable` 或降低 threshold。

## Open Questions

沒有規格未決問題。GPUI-CE 候選已凍結為 `https://github.com/damody/gpui-ce-explorer.git` commit `8945e2981b9fd00ca887e042d8adb9acc241b168` 的乾淨來源；capability spike只驗證此候選，不引用 `D:\SuperExplorer\vendor\gpui-ce` 的未提交 patch。AppBar/Start host 的最小可行 HWND 接法與 monitor identity 細節會在已規範的 capability spike leaf 中以 B 級修正流程收斂；若需要改變框架、公開合約或 blocking gate，則升級為 C 級並請使用者核准。完整 program 開始前仍須在 Wave 0 readiness record 明列 exact reference profile 與實體 mixed-DPI 環境的 availability；profile drift 或實體顯示器缺失必然阻止 final release。
