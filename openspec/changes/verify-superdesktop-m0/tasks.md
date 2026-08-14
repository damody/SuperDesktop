## 1. Release Baseline 與 Build Gate

### 1.1 凍結 Reference Profile 與 Production Lineage

**目的：** 在測試前固定 ExplorerPatcher reference 與所有 production child contract/evidence lineage。
**輸入：** 已封存 production changes、目前 Windows 11＋ExplorerPatcher 環境、參考截圖。
**產出：** Immutable reference profile、contract hash manifest、baseline disposition。
**依賴：** Bootstrap、platform、core、desktop、taskbar、bridge、lifecycle changes 已完成。
**Owner／Wave：** Release verification owner／Wave 6。
**Gate／Evidence：** `G-ARCH`、`G-TRACE`；`evidence/artifacts/1.1/`。
**完成門檻：** OS/EP/settings/image hashes 相符，所有 production contract hashes 與 evidence lineage 無 stale/blocked/N/A。

- [x] 1.1.1 驗證 Windows build、ExplorerPatcher version 與 binary hash。
- [x] 1.1.2 匯出並雜湊影響 taskbar/desktop UI 的 ExplorerPatcher 設定。
- [x] 1.1.3 驗證持久 reference image 的既定 SHA-256。
- [ ] 1.1.4 匯總所有 production child archive revision 與 public contract hashes。
- [x] 1.1.5 執行 evidence validator 並拒絕 stale、blocked、N/A 或無效 replacement。
- [x] 1.1.6 在任何候選 capture 前固定 ±2px scaled geometry、SSIM≥0.95、exact state 與動態矩形 masks，保存 immutable baseline contract hash。

### 1.2 執行 Build、Quality 與 Offline Gate

**目的：** 證明 release source 可用固定工具鏈、來源與 lockfile 重建。
**輸入：** 1.1 baseline、bootstrap source manifest。
**產出：** Build logs、test reports、release binary hashes。
**依賴：** 1.1。
**Owner／Wave：** Build verification owner／Wave 6。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/1.2/`。
**完成門檻：** fmt/check/clippy/tests/release/offline 全部 exit 0，offline source hashes 相符。

- [x] 1.2.1 執行 `cargo fmt --check` 並保存輸出。
- [x] 1.2.2 執行 workspace `cargo check --locked` 並保存輸出。
- [x] 1.2.3 執行 workspace clippy warnings-as-errors 並保存輸出。
- [x] 1.2.4 執行 workspace tests 並保存逐 test 結果。
- [x] 1.2.5 執行 release build 並記錄所有 product binary hashes。
- [x] 1.2.6 在 network-disabled isolated CARGO_HOME 執行 `--locked --offline` build。
- [x] 1.2.7 驗證 isolated sources 與 bootstrap provenance manifest hashes 相符。

## 2. Reference UI、DPI 與 Monitor Gate

### 2.1 執行 ExplorerPatcher Reference Visual/Interaction Matrix

**目的：** 依凍結 profile 驗證桌面與工作列的 Win10 風格幾何、狀態及互動。
**輸入：** 1.1 profile、release binaries、deterministic fixtures。
**產出：** Screenshots、geometry/state assertions、interaction traces。
**依賴：** 1.1、1.2。
**Owner／Wave：** UI verification owner／Wave 6。
**Gate／Evidence：** `G-DESKTOP`、`G-TASKBAR`；`evidence/artifacts/2.1/`。
**完成門檻：** Desktop grid、雙列 taskbar、Start、SuperExplorer entry、tasks、status/time 符合容差且互動全過。

- [x] 2.1.1 執行 desktop wallpaper/icon-grid/selection reference capture。
- [ ] 2.1.2 執行預設雙列 taskbar geometry/reference capture。
- [x] 2.1.3 驗證 Start、SuperExplorer fixed entry、task buttons 與 status region ordering。
- [x] 2.1.4 驗證 active、minimized、attention、group 與 unavailable states。
- [x] 2.1.5 執行 pointer、keyboard 與 UIA reference interaction paths。
- [x] 2.1.6 索引 screenshots、geometry data、容差與 artifact hashes。
- [x] 2.1.7 從 desktop 與 taskbar 固定入口以 pointer、keyboard 與 UIA 分別執行真實 SuperExplorer end-to-end launch。
- [x] 2.1.8 驗證 preview 與 Shell 兩種模式各自的 Start probe/invocation evidence 均存在且通過。

### 2.2 執行單螢幕 DPI Matrix

**目的：** 驗證 100%、125%、150%、175%、200% DPI 下的 desktop/taskbar geometry 與 interaction。
**輸入：** Release binaries、DPI automation harness。
**產出：** 每 DPI 的 captures、geometry、hit-target 與 work-area results。
**依賴：** 2.1。
**Owner／Wave：** Display verification owner／Wave 6。
**Gate／Evidence：** `G-DPI-MONITOR`；`evidence/artifacts/2.2/`。
**完成門檻：** 五個 DPI 全部通過，沒有重疊、缺字、錯 hit target 或錯 work area。

- [ ] 2.2.1 執行 100% DPI visual/geometry/interaction subcheck。
- [ ] 2.2.2 執行 125% DPI visual/geometry/interaction subcheck。
- [ ] 2.2.3 執行 150% DPI visual/geometry/interaction subcheck。
- [ ] 2.2.4 執行 175% DPI visual/geometry/interaction subcheck。
- [ ] 2.2.5 執行 200% DPI visual/geometry/interaction subcheck。
- [ ] 2.2.6 比較五組 work area、logical positions 與 hit-target bounds。

### 2.3 執行虛擬 Mixed-DPI Topology Gate

**目的：** 自動驗證 mixed-DPI add/remove、primary change、hot-plug 與狀態對帳。
**輸入：** Virtual display harness、release binaries。
**產出：** Topology event traces、geometry snapshots、reconciliation results。
**依賴：** 2.2。
**Owner／Wave：** Display verification owner／Wave 6。
**Gate／Evidence：** `G-DPI-MONITOR`、`G-SHELL-TAKEOVER`；`evidence/artifacts/2.3/`。
**完成門檻：** 所有虛擬 topology transitions 通過且 AppBar/work area/desktop positions 收斂。

- [x] 2.3.1 建立兩個不同 DPI 的虛擬顯示器 topology fixture。
- [x] 2.3.2 驗證 monitor add/remove 與 taskbar/Desktop surface lifecycle。
- [x] 2.3.3 驗證 primary monitor change 與 pinned/layout ownership。
- [x] 2.3.4 驗證 runtime DPI change 與 logical-position reconciliation。
- [x] 2.3.5 驗證 hot-plug storm 後 final authoritative snapshot。
- [x] 2.3.6 驗證 teardown 後所有 work area 回到 baseline。

### 2.4 執行實體 Mixed-DPI Release Confirmation

**目的：** 在真實雙螢幕與不同 DPI 上確認驅動、座標、輸入與 work-area 行為。
**輸入：** 兩個實體顯示器、release candidate、測試程序。
**產出：** Physical topology captures、manual interaction record、release disposition。
**依賴：** 2.3；外部實體硬體。
**Owner／Wave：** Release verification owner／Wave 6 external。
**Gate／Evidence：** `G-DPI-MONITOR`；`evidence/artifacts/2.4/`。
**完成門檻：** 兩個實體 display 使用不同 scale，所有指定行為通過；缺硬體時保持 blocked 且不得發行。

- [ ] 2.4.1 記錄兩個實體顯示器 EDID/identity、bounds、scale 與 driver metadata。
- [ ] 2.4.2 驗證各螢幕 taskbar、work area 與 desktop layout。
- [ ] 2.4.3 驗證跨螢幕 pointer、keyboard focus 與 drag interaction。
- [ ] 2.4.4 驗證 primary change 與實體 hot-plug recovery。
- [ ] 2.4.5 保存照片/screenshots/raw geometry 並簽署 confirmation disposition。

## 3. Windows 10 相容性與 Lifecycle Gate

### 3.1 執行 Windows 10 22H2 相容性矩陣

**目的：** 證明 Windows 10 22H2 可啟動、操作並安全回復，而非作為視覺 baseline。
**輸入：** Windows 10 22H2 x64 環境、release candidate。
**產出：** OS metadata、launch/interaction/recovery evidence。
**依賴：** 1.2；外部 Windows 10 環境。
**Owner／Wave：** Compatibility owner／Wave 6 external。
**Gate／Evidence：** `G-ARCH`、`G-DESKTOP`、`G-TASKBAR`、`G-SHELL-TAKEOVER`、`G-GUARDIAN-RECOVERY`；`evidence/artifacts/3.1/`。
**完成門檻：** Preview、Shell opt-in、desktop/taskbar/bridge、normal recovery、forced-crash recovery 全數通過，並結合 Wave 5 provisional lineage 發布兩個 final dispositions；缺環境保持 blocked。

- [ ] 3.1.1 記錄 Windows 10 22H2 build、session、display 與 binary hashes。
- [ ] 3.1.2 驗證 preview launch 與 zero-mutation。
- [ ] 3.1.3 驗證明確 Shell opt-in 與 desktop/taskbar interaction。
- [ ] 3.1.4 驗證 SuperExplorer default 與 folder launch。
- [ ] 3.1.5 驗證正常退出後 Explorer/work-area recovery。
- [ ] 3.1.6 執行 forced-crash 並驗證十秒 guardian recovery contract。
- [ ] 3.1.7 產生 Windows 10 implemented/deferred/unavailable capability matrix，禁止 placeholder 或未實作控制算 passed。
- [ ] 3.1.8 結合 Wave 5 provisional takeover lineage 與 Windows 10 takeover/normal-exit evidence，發布 final `G-SHELL-TAKEOVER` disposition。
- [ ] 3.1.9 結合 Wave 5 provisional recovery lineage 與 Windows 10 forced-crash evidence，發布 final `G-GUARDIAN-RECOVERY` disposition。

### 3.2 執行 Reference OS Lifecycle Regression

**目的：** 在 Windows 11＋ExplorerPatcher reference profile 複驗 preview、takeover、normal/crash recovery。
**輸入：** 1.1 profile、release candidate、lifecycle harness。
**產出：** Lifecycle timelines、identity/work-area snapshots。
**依賴：** 2.1、lifecycle child evidence。
**Owner／Wave：** Lifecycle verification owner／Wave 6。
**Gate／Evidence：** `G-SHELL-TAKEOVER`、`G-GUARDIAN-RECOVERY`、`G-SAFETY`；`evidence/artifacts/3.2/`。
**完成門檻：** Preview zero-mutation、normal recovery 與 10/10 forced-crash deadline regression 全過。

- [x] 3.2.1 驗證 preview 與 Safe Mode/unsupported-session fail-closed fixtures。
- [x] 3.2.2 驗證 normal takeover/exit 的 transaction timeline。
- [x] 3.2.3 重跑十次 forced-crash recovery 並保存 raw timestamps。
- [x] 3.2.4 驗證 owner-race、guardian anti-spoof 與 FFI panic regression。
- [x] 3.2.5 驗證所有 run 後 Explorer/work area/registry 回 baseline。

## 4. Accessibility、Localization 與 Stability

### 4.1 執行 Accessibility 與 Input Gate

**目的：** 驗證所有核心操作可由 keyboard 與 UIA/AccessKit 完成。
**輸入：** Release candidate、accessibility automation harness。
**產出：** Focus traces、accessibility tree、action results。
**依賴：** 2.1。
**Owner／Wave：** Accessibility owner／Wave 6。
**Gate／Evidence：** `G-A11Y-I18N`；`evidence/artifacts/4.1/`。
**完成門檻：** 每個 interactive control 具有正確 name/role/state/action，focus order 與 keyboard-only flow 通過。

- [x] 4.1.1 掃描 desktop controls 的 accessibility identity 與 actions。
- [x] 4.1.2 掃描 taskbar/Start/SuperExplorer controls 的 accessibility identity 與 actions。
- [x] 4.1.3 執行 desktop keyboard-only selection/activate flow，並驗證 rename/delete/refresh/context menu 不被暴露或造成 mutation。
- [x] 4.1.4 執行 taskbar keyboard-only navigation/group/Start/bridge flow。
- [x] 4.1.5 執行高對比 visual、focus indicator 與 hit-target test。

### 4.2 執行 Localization、Fallback、Bidi 與 IME Gate

**目的：** 驗證繁中/英文資源及簡中 fallback、RTL/bidi、截斷與 IME 版面安全。
**輸入：** Locale fixtures、deterministic strings、IME harness。
**產出：** Locale captures、geometry assertions、IME traces。
**依賴：** 4.1。
**Owner／Wave：** Localization owner／Wave 6。
**Gate／Evidence：** `G-A11Y-I18N`；`evidence/artifacts/4.2/`。
**完成門檻：** zh-TW/en 全字串可用；zh-CN fallback 與 RTL/bidi 無缺字/重疊；IME 組字不丟焦點或文字。

- [x] 4.2.1 執行繁中資源完整性、截斷與 geometry test。
- [x] 4.2.2 執行英文資源完整性、截斷與 geometry test。
- [x] 4.2.3 執行簡中字形/fallback/截斷 headful test。
- [x] 4.2.4 執行 RTL/bidi layout、reading order 與 interaction test。
- [x] 4.2.5 執行繁中 IME composition、commit、cancel 與 focus-stability test。

### 4.3 執行 Stress 與資源穩定性 Gate

**目的：** 在事件洪水、timeout/crash 與長時間運作下證明 queue/cache/OS resources 有界。
**輸入：** Stress harness、release candidate、resource counters。
**產出：** Raw traces、resource time series、stability disposition。
**依賴：** 3.2、4.1。
**Owner／Wave：** Reliability owner／Wave 6。
**Gate／Evidence：** `G-SAFETY`、`G-PERF`；`evidence/artifacts/4.3/`。
**完成門檻：** 每種 stress fixture 獨立通過，所有指定 resource 回穩且無無界成長。

- [x] 4.3.1 執行 desktop watcher overflow/rename storm soak。
- [x] 4.3.2 執行 window-event storm/reconciliation soak。
- [x] 4.3.3 執行 monitor hot-plug/DPI-change storm soak。
- [x] 4.3.4 執行 bridge cancel/timeout/late-callback soak。
- [x] 4.3.5 執行 guardian crash-loop protection soak。
- [x] 4.3.6 驗證 working set bound 與穩定區間。
- [x] 4.3.7 驗證 thread count bound 與穩定區間。
- [x] 4.3.8 驗證 process/kernel handle count bound 與穩定區間。
- [x] 4.3.9 驗證 GDI object count bound 與穩定區間。
- [x] 4.3.10 驗證 USER object count bound 與穩定區間。
- [x] 4.3.11 驗證 icon/wallpaper/event cache bounds 與穩定區間。

## 5. Performance、安全與發行判定

### 5.1 執行 Performance Gate

**目的：** 以原始樣本判定四個固定 M0 效能門檻。
**輸入：** 1.1 reference environment、release candidate、benchmark harness。
**產出：** Raw timestamps/counters、statistics、performance disposition。
**依賴：** 4.3。
**Owner／Wave：** Performance owner／Wave 6。
**Gate／Evidence：** `G-PERF`；`evidence/artifacts/5.1/`。
**完成門檻：** Cold start ≤2s、idle CPU median <0.5%、event-to-visible p95 <100ms、working set <150MiB。

- [x] 5.1.1 記錄 benchmark 工具版本、背景程序、暖機與樣本數設定。
- [x] 5.1.2 量測冷啟動並保存逐 run timestamps。
- [x] 5.1.3 量測 idle CPU 並保存原始 counter samples 與 median。
- [x] 5.1.4 量測 shell event-to-visible latency 並保存逐 event timestamps 與 p95。
- [x] 5.1.5 量測 M0 working set 並保存原始 time series 與 peak/steady result。
- [x] 5.1.6 驗證四個 threshold 並發布 `G-PERF` disposition。

### 5.2 執行安全、授權與來源 Audit

**目的：** 確認 opt-in、fail-closed、資料邊界、隱私、dependency/license 與來源政策。
**輸入：** Release source/binaries、bootstrap provenance、security fixtures。
**產出：** 分項 audit reports、negative-test results。
**依賴：** 1.2、3.2。
**Owner／Wave：** Safety/compliance owner／Wave 6。
**Gate／Evidence：** `G-SAFETY`、`G-ARCH`；`evidence/artifacts/5.2/`。
**完成門檻：** 每項 audit 獨立通過，無使用者資料 mutation、credential leak 或來源/授權違反。

- [x] 5.2.1 稽核 Shell mode 明確 opt-in 與 preview/Safe Mode fail-closed。
- [x] 5.2.2 稽核 fixture-root、canonical path、reparse 與受保護檔案 mutation boundary。
- [x] 5.2.3 執行 path、argument、environment 與 executable-substitution injection tests。
- [x] 5.2.4 稽核 credential、clipboard、environment 與 log redaction。
- [x] 5.2.5 稽核 dependency inventory 與逐 dependency license compatibility。
- [x] 5.2.6 稽核 PExplorer/SuperExplorer source boundary 與 repository mutation。

### 5.3 完成 Traceability 與 Independent Review

**目的：** 證明每個 requirement/scenario/gate/task 都有有效 evidence，且無未解 P0/P1。
**輸入：** 1 至 5 全部 evidence、program traceability map。
**產出：** Coverage report、review findings、remediation lineage、release disposition。
**依賴：** 1.1 至 5.2；外部 blocked confirmation 必須先解除。
**Owner／Wave：** Independent reviewer 複核；Primary integrator／原 gate owner 修正／Wave 6 exit。
**Gate／Evidence：** 所有 blocking gates、`G-TRACE`；`evidence/artifacts/5.3/`。
**完成門檻：** Coverage 100%、replacement 有效、無 blocked/stale、P0/P1 歸零，reviewer 簽署 release disposition。

- [x] 5.3.1 產生 requirement/scenario/gate/task/evidence 雙向 coverage report。
- [x] 5.3.2 執行 dangling/cyclic/incomplete/coverage-drift replacement negative validation。
- [ ] 5.3.3 由 Independent reviewer 執行完整 P0/P1 架構、安全與 evidence review。
- [ ] 5.3.4 由 Primary integrator 分派 P0/P1 給原 gate owner 並同步修正 design/spec/tasks。
- [ ] 5.3.5 由原 gate owner 重跑所有受影響且標 stale 的 gates。
- [ ] 5.3.6 由 Independent reviewer 複核 remediation lineage 與重跑結果。
- [ ] 5.3.7 驗證 Windows 10 與實體 mixed-DPI mandatory leaves 均已 passed。
- [ ] 5.3.8 發布無 P0/P1、無 blocked/stale/N/A 的 M0 release disposition。
