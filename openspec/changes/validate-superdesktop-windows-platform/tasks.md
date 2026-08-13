## 1. 參考環境與 GPUI Native Window

### 1.1 凍結 Capability Spike Profile

**目的：** 建立後續 spike 可重現的 OS、ExplorerPatcher、工具鏈與資源快照。
**輸入：** 已封存 Bootstrap change 的 accepted workspace contract hash（含固定 Windows binding/features、offline provenance與 audited unsafe boundary）、目前 Windows 11＋ExplorerPatcher 環境、參考截圖。
**產出：** Immutable capability profile 與 hashes。
**依賴：** `bootstrap-superdesktop-workspace` 已通過。
**Owner／Wave：** Platform owner／Wave 2。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/1.1/`。
**完成門檻：** Bootstrap workspace contract逐 input有效且無 dependency/lint/ownership drift；OS build、EP version/binary、設定、GPUI revision、reference image與resource snapshot完整；目前 session admission probe在任何AppBar/Hook mutation前 passed。

- [x] 1.1.1 擷取 OS build、session、display adapter 與 monitor identity。
- [x] 1.1.2 擷取 ExplorerPatcher version、binary hash 與影響 UI 的設定。
- [x] 1.1.3 將參考截圖保存到持久 evidence 並驗證既定 SHA-256。
- [x] 1.1.4 記錄 GPUI-CE revision、Rust toolchain與 1.1 read-only profile/admission probe binary hash；不得提前建立 1.2 native-window spike。
- [x] 1.1.5 在任何 AppBar/Hook spike 前執行 Safe Mode、interactive user token/session 與 supported-session admission probe。
- [x] 1.1.6 保存 admission probe 前後 Explorer、AppBar 與 work-area zero-mutation snapshot；未 passed 則停止本 change。
- [x] 1.1.7 以固定 archive revision relocation verifier 驗證 bootstrap archive workspace contract逐 input hash、Windows binding/features、lock/vendor provenance、全域 unsafe deny與 `platform-win` 唯一 audited exception；只允許舊 active-change 前綴映射到封存根目錄，任一其他漂移即 stop 並交回 Primary。

### 1.2 驗證 GPUI HWND 與 Message Bridge

**目的：** 證明 GPUI surface 可安全取得 native HWND 並接收必要 Win32 訊息。
**輸入：** 1.1 profile、workspace skeleton。
**產出：** Headful spike binary、raw message trace、resource snapshot。
**依賴：** 1.1。
**Owner／Wave：** GPUI platform owner／Wave 2。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/1.2/`。
**完成門檻：** 同一個 GPUI-owned borrowed HWND 的 identity、message delivery 與 teardown 全數通過；不得建立第二個測試 HWND或由 bridge 銷毀 HWND。1.2 只允許 capability preview，3.2 通過前不得供 production 或 Shell takeover 使用。

**B-W2-1.2-001 architecture successor：** GPUI example 位於 `desktop-ui`，僅以 dev-dependency composition 使用 `platform-win/common`；禁止 `platform-win -> gpui`，並由 Primary 更新 architecture contract後才開始實作。

- [ ] 1.2.1 建立最小 GPUI native-window spike，從 live `gpui::Window` 輸出 borrowed HWND、PID/thread/session、GPUI WindowId 與 generation identity；不得另建替代 HWND。
- [ ] 1.2.2 驗證 DPI、display-change 與 activation 訊息可轉為 owned event。
- [ ] 1.2.3 驗證視窗關閉後 callback 與 HWND 不再被使用。
- [ ] 1.2.4 保存 headful trace、handles before/after 與 binary hash。

## 2. Shell API 可逆性與顯示器能力

### 2.1 驗證 AppBar 與 Shell Hook 可逆性

**目的：** 證明 reserve/restore 與 hook register/unregister 不留下系統 mutation。
**輸入：** 1.2 native HWND spike。
**產出：** AppBar/Shell Hook spike、work-area 與 hook traces。
**依賴：** 1.2。
**Owner／Wave：** Windows shell owner／Wave 2。
**Gate／Evidence：** `G-SHELL-TAKEOVER-CAPABILITY`；`evidence/artifacts/2.1/`。
**完成門檻：** 正常、失敗注入與重複 teardown 後 work area、hook 與 handles 回到 baseline。

- [ ] 2.1.1 實作單螢幕 AppBar register/query/remove spike。
- [ ] 2.1.2 驗證 AppBar remove 後 work area 精確恢復。
- [ ] 2.1.3 實作 Shell Hook register/unregister 與事件 trace。
- [ ] 2.1.4 注入中途失敗並驗證冪等 teardown。
- [ ] 2.1.5 保存每個 subcheck 的 before/after raw snapshot。

### 2.2 驗證 Monitor、DPI 與 Start Host Probe

**目的：** 證明平台可取得穩定 monitor identity、per-monitor DPI、topology 變更與可信 Start host 結果。
**輸入：** 1.2 native message bridge、凍結 profile。
**產出：** Monitor/DPI/Start probe reports。
**依賴：** 1.2。
**Owner／Wave：** Windows shell owner／Wave 2。
**Gate／Evidence：** `G-DPI-MONITOR`、`G-TASKBAR`；`evidence/artifacts/2.2/`。
**完成門檻：** Identity 跨 refresh 穩定，虛擬 topology event 可見，Start probe truthful 且不假成功。

- [ ] 2.2.1 實作 monitor identity、bounds、work area 與 DPI probe。
- [ ] 2.2.2 在虛擬顯示器上驗證 add/remove、primary 與 DPI change event。
- [ ] 2.2.3 探測 ExplorerPatcher reference profile 的 Start host 與 invocation path。
- [ ] 2.2.4 驗證 Start host 缺失或拒絕時回傳 typed unavailable result。
- [ ] 2.2.5 保存 topology traces、Start result 與 resource snapshot。

## 3. Guardian 與 ABI 安全 Gate

### 3.1 驗證 Guardian Process-handle Lease

**目的：** 證明 guardian 可用不可偽造的 inherited handle 判定主程序終止。
**輸入：** Bootstrap identity、Windows process API。
**產出：** Lease spike 與 negative-test report。
**依賴：** 1.1。
**Owner／Wave：** Lifecycle platform owner／Wave 2。
**Gate／Evidence：** `G-GUARDIAN-RECOVERY-CAPABILITY`、`G-SAFETY`；`evidence/artifacts/3.1/`。
**完成門檻：** Valid lease 可完成 terminal；forged/stale/wrong-session targets 全被拒絕。

- [ ] 3.1.1 實作 restricted inherited process-handle lease spike。
- [ ] 3.1.2 驗證 PID、creation time、session、nonce 與 executable identity binding。
- [ ] 3.1.3 執行 forged、stale、wrong-session 與 unexpected-handle negative tests。
- [ ] 3.1.4 驗證所有 process/thread handles 在 terminal 後關閉。

### 3.2 驗證 FFI No-unwind Boundary

**目的：** 證明所有 Win32 callback 不會讓 Rust unwind 穿越 ABI。
**輸入：** Callback wrapper contract、panic injection fixture。
**產出：** FFI spike、panic traces、cleanup report。
**依賴：** 1.2。
**Owner／Wave：** Platform safety owner／Wave 2。
**Gate／Evidence：** `G-SAFETY`；`evidence/artifacts/3.2/`。
**完成門檻：** SuperDesktop-owned callback與 pinned `gpui_windows` 主 WndProc 的真實 public callback path 均不得讓 panic 穿越 ABI；Panic 轉成 typed fatal event，backend HWND terminal、GPUI window-closed terminal、double callback/shutdown race 無 UB、double-close 或 resource leak。未通過即維持 preview-only並阻擋 production/Shell 使用。

- [ ] 3.2.1 實作共用 `extern system` catch-unwind wrapper spike。
- [ ] 3.2.2 由真實 public GPUI callback path 對 pinned `gpui_windows` 主 WndProc 注入 callback panic並驗證 typed fatal event；不得以只測外層 subclass 取代。
- [ ] 3.2.3 執行 double callback 與 shutdown-race 測試。
- [ ] 3.2.4 保存 ABI signature、handle lifecycle 與 panic evidence。

### 3.3 驗證 Safe Mode 與 Unsupported Session Fail-closed

**目的：** 以模擬環境負面 fixture 複驗已在 1.1 執行的 admission probe contract。
**輸入：** 1.1 已凍結且在真實 session passed 的 probe contract、zero-mutation snapshot harness。
**產出：** Safe Mode/non-interactive/unsupported-session negative fixtures 與 before/after snapshots。
**依賴：** 1.1；不依賴或先執行任何 AppBar/Hook mutation。
**Owner／Wave：** Platform safety owner／Wave 2。
**Gate／Evidence：** `G-SHELL-TAKEOVER-CAPABILITY`、`G-SAFETY`；`evidence/artifacts/3.3/`。
**完成門檻：** 三種拒絕情境在 mutation 前 terminal，AppBar、Explorer、work area 全部 unchanged。

- [ ] 3.3.1 建立 Safe Mode、interactive token/session 與 supported-session 的可注入 probe fixture adapter。
- [ ] 3.3.2 執行 Safe Mode fail-closed zero-mutation fixture。
- [ ] 3.3.3 執行 non-interactive/wrong-user token fail-closed fixture。
- [ ] 3.3.4 執行 unsupported-session fail-closed fixture。
- [ ] 3.3.5 保存每個 fixture 的 probe result 與 AppBar/Explorer/work-area before/after。

### 3.4 發布 Capability Go／Stop Disposition

**目的：** 在 production changes 開始前產出不可含糊的能力判定。
**輸入：** 1.1 至 3.3 全部 evidence。
**產出：** Signed capability matrix、platform-common API/ABI hash manifest 與 go/stop disposition。
**依賴：** 1.1、1.2、2.1、2.2、3.1、3.2、3.3。
**Owner／Wave：** Primary integrator／Wave 2 exit。
**Gate／Evidence：** `G-ARCH`、`G-SHELL-TAKEOVER-CAPABILITY`、`G-DPI-MONITOR`、`G-GUARDIAN-RECOVERY-CAPABILITY`；`evidence/artifacts/3.4/`。
**完成門檻：** 每個 required subcheck 都有 passed evidence；任一失敗則明確 stop 並建立 B/C disposition。

- [ ] 3.4.1 產生 required capability matrix 與 evidence links。
- [ ] 3.4.2 產生 platform-common public API/ABI hash input manifest 與 SHA-256。
- [ ] 3.4.3 驗證不存在缺失、stale、blocked 或 N/A 的 required subcheck。
- [ ] 3.4.4 由 Primary integrator 簽署 go 或 stop disposition。
