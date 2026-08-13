## 1. Desktop Window 與 Rendering

### 1.1 建立 Per-monitor GPUI Desktop Surface

**目的：** 為每個 monitor 建立 DPI-aware、可重建的 GPUI desktop window。
**輸入：** Capability go、shell-core contract、monitor adapter。
**產出：** `desktop-ui` windows、monitor bindings、headful fixtures。
**依賴：** `build-superdesktop-shell-core` contract hash。
**Owner／Wave：** Desktop UI owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-DPI-MONITOR`；`evidence/artifacts/1.1/`。
**完成門檻：** 每個 monitor 恰有一個 surface，DPI/topology 變更不留下 orphan window。

- [ ] 1.1.1 實作 monitor identity 到 GPUI desktop window 的 lifecycle mapping。
- [ ] 1.1.2 實作 DPI-aware bounds、z-order 與 non-activating desktop semantics。
- [ ] 1.1.3 實作 monitor add/remove、primary change 與 DPI change reconciliation。
- [ ] 1.1.4 建立單螢幕與虛擬多螢幕 headful lifecycle tests。
- [ ] 1.1.5 保存 window identity、geometry 與 teardown resource evidence。

### 1.2 實作 Wallpaper Pipeline

**目的：** 支援核准桌布模式並限制 decode/cache 資源。
**輸入：** Desktop surface、settings wallpaper contract。
**產出：** Wallpaper loader/render modes/cache、visual fixtures。
**依賴：** 1.1。
**Owner／Wave：** Desktop rendering owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-PERF`；`evidence/artifacts/1.2/`。
**完成門檻：** Fill/Fit/Stretch/Tile/Center/Span 與 solid color 正確，壞檔 fallback 且 cache 有界。

- [ ] 1.2.1 實作 solid color 與圖片 decode error fallback。
- [ ] 1.2.2 實作 Fill、Fit、Stretch 與 Center geometry。
- [ ] 1.2.3 實作 Tile 與跨 monitor Span geometry。
- [ ] 1.2.4 實作 bounded decoded-image cache 與 invalidation。
- [ ] 1.2.5 產生各模式/DPI visual fixtures 與 geometry assertions。

## 2. Shell Namespace 與 Item Layout

### 2.1 合併 User/Public Desktop Shell Items

**目的：** 以 stable Shell identity 去重並呈現 User/Public Desktop items。
**輸入：** Platform Shell namespace adapter、core identity contract。
**產出：** Namespace provider、merge rules、item view models。
**依賴：** 1.1。
**Owner／Wave：** Desktop platform owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-ARCH`；`evidence/artifacts/2.1/`。
**完成門檻：** 同名不同 identity 不誤合併，同 identity 去重，hidden/system 規則可驗證。

- [ ] 2.1.1 實作 User Desktop 與 Public Desktop known-folder enumeration。
- [ ] 2.1.2 將 PIDL/COM 結果轉成 owned stable Shell identity 與 capabilities。
- [ ] 2.1.3 實作 identity-based merge、display-name 與 icon descriptor mapping。
- [ ] 2.1.4 加入同名、Unicode、hidden 與 system item 獨立 fixtures。
- [ ] 2.1.5 驗證 UI crate 不持有 COM/PIDL 並保存 architecture evidence。

### 2.2 實作 Icon Grid、Selection 與 Position Persistence

**目的：** 提供 Win10 風格圖示網格、多選與跨 DPI/重啟位置保存。
**輸入：** 1.1 surface、2.1 item model、settings store。
**產出：** Grid layout、selection model、position mapper。
**依賴：** 1.1、2.1。
**Owner／Wave：** Desktop UI owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-DPI-MONITOR`；`evidence/artifacts/2.2/`。
**完成門檻：** Layout deterministic；Ctrl/Shift/rubber-band 正確；位置依 logical coordinates 保存並 clamp。

- [ ] 2.2.1 實作 Win10 參考尺寸的 DPI-aware icon/label grid。
- [ ] 2.2.2 實作單選、Ctrl toggle、Shift range 與 rubber-band selection。
- [ ] 2.2.3 實作 drag reposition、collision resolution 與 logical-coordinate persistence。
- [ ] 2.2.4 實作 monitor 缺失、DPI change 與 work-area change 的 deterministic clamp。
- [ ] 2.2.5 建立重啟、DPI 轉換與 monitor remap tests。
- [ ] 2.2.6 保存各 DPI geometry、selection 與 persisted settings evidence。

### 2.3 實作 Desktop 固定 SuperExplorer 入口

**目的：** 在每個 desktop surface 提供 truthful、可及且不宣稱「本機」的固定入口。
**輸入：** 1.1 surface、Wave 3 bridge DTO/fake adapter、reference profile。
**產出：** Fixed entry view、activation binding、repair states、headful tests。
**依賴：** 1.1、2.2、core contract hash。
**Owner／Wave：** Desktop UI owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-EXPLORER-BRIDGE`、`G-A11Y-I18N`；`evidence/artifacts/2.3/`。
**完成門檻：** 每個 surface 皆有入口；pointer/keyboard/UIA 發同一 default command；failure 顯示 repair state。

- [ ] 2.3.1 實作「SuperExplorer」固定入口的 stable identity、icon、label 與 layout。
- [ ] 2.3.2 綁定 pointer、Enter 與 UIA invoke 到同一 default bridge command。
- [ ] 2.3.3 以 fake bridge 驗證 launched、validation/spawn failure、cancel 與 timeout states。
- [ ] 2.3.4 執行多 surface/DPI existence、focus、accessible name/role/action headful tests。

## 3. Desktop Interaction 與 Watcher Recovery

### 3.1 實作鍵盤、滑鼠與 M0 Actions

**目的：** 提供 M0 選取、導覽、activation、位置拖曳，並確保延後功能不被暴露。
**輸入：** 2.1 items、2.2 selection、platform effects。
**產出：** Input handlers、typed activation commands、延後功能 negative-availability UI/tests。
**依賴：** 2.1、2.2。
**Owner／Wave：** Desktop interaction owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-A11Y-I18N`、`G-SAFETY`；`evidence/artifacts/3.1/`。
**完成門檻：** Pointer/keyboard/UIA 的 M0 路徑等價；rename、context menu、delete/recycle、refresh command 與 file-transfer drag/drop 不可操作也不產生 mutation。

- [ ] 3.1.1 實作 Enter、方向鍵與 selection keyboard commands。
- [ ] 3.1.2 實作 pointer double-click、rubber-band 與 position-only drag commands。
- [ ] 3.1.3 預留 rename/context/delete/refresh 的 typed unavailable states 而不執行 effect。
- [ ] 3.1.4 驗證 F2、Delete、F5 與 context-menu gesture 不暴露未實作控制或造成檔案 mutation。
- [ ] 3.1.5 驗證 drag 只改 icon position，不啟動 file transfer。
- [ ] 3.1.6 執行 keyboard-only、pointer 與 UIA M0 action headful tests。

### 3.2 實作資料夾到 SuperExplorer Command Routing

**目的：** 將 desktop folder identity 轉成一次 typed bridge command，並以 fake terminal 驗證 UI 狀態。
**輸入：** Selected Shell identity、Wave 3 凍結的 bridge request/result/repair DTO。
**產出：** Folder routing、pending/terminal UI、fake bridge tests。
**依賴：** 3.1、core contract hash。
**Owner／Wave：** Desktop interaction owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-EXPLORER-BRIDGE`；`evidence/artifacts/3.2/`。
**完成門檻：** Enter/double-click folder 各只發一個 command；success/failure/cancel/timeout terminal 正確更新 UI，late result 被拒絕。

- [ ] 3.2.1 實作 folder capability 到 typed bridge launch request 的 mapping。
- [ ] 3.2.2 綁定 Enter 與 double-click 到同一個 folder launch command。
- [ ] 3.2.3 以 fake bridge 驗證 launched 與 validation/spawn failure terminal UI。
- [ ] 3.2.4 以 fake bridge 驗證 cancel/timeout 與 late-terminal suppression。

### 3.3 實作 Windows Association Activation

**目的：** 一般檔案透過 Windows association 啟動並回傳 exactly-once terminal。
**輸入：** Selected Shell identity、platform association adapter。
**產出：** Association effect、typed results、repair prompt。
**依賴：** 3.1。
**Owner／Wave：** Desktop platform owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-SAFETY`；`evidence/artifacts/3.3/`。
**完成門檻：** Success/failure/cancel/timeout 每個 request 恰一 terminal，late callback 不改 UI。

- [ ] 3.3.1 實作 owned Shell identity 到 Windows association request adapter。
- [ ] 3.3.2 實作 success、validation-failed、launch-failed、cancelled、timed-out terminal mapping。
- [ ] 3.3.3 加入 cancel-vs-success 與 timeout-vs-late-callback tests。
- [ ] 3.3.4 實作 GPUI repair prompt 且不 fallback 到 Windows Explorer。
- [ ] 3.3.5 執行真實 fixture file association integration test。

### 3.4 實作 Watcher Overflow 與 Stale Recovery

**目的：** Namespace 變更洪水後回到權威 snapshot 且保留可恢復 selection。
**輸入：** Platform watcher、core reconciliation、2.1 identities。
**產出：** Watcher adapter、overflow mapping、stress tests。
**依賴：** 2.1、2.2、core reconciliation。
**Owner／Wave：** Desktop platform owner／Wave 4A。
**Gate／Evidence：** `G-DESKTOP`、`G-PERF`；`evidence/artifacts/3.4/`。
**完成門檻：** Rename/overflow/stale sequence 最終與權威 enumeration 相同，queue/cache 有界。

- [ ] 3.4.1 對 watcher callback 套用 Wave 2 凍結的 no-unwind wrapper 並轉成 owned event。
- [ ] 3.4.2 實作 overflow 到 full namespace refresh 的單次觸發。
- [ ] 3.4.3 加入外部 rename storm 與 stale completion suppression tests。
- [ ] 3.4.4 驗證 refresh 後依 stable identity 恢復 selection/position。
- [ ] 3.4.5 保存 raw watcher trace、queue depth 與 final snapshot diff。

## 4. Desktop Gate

### 4.1 完成 Desktop Headful Contract Gate

**目的：** 將 rendering、namespace、interaction、association 與 recovery 整合為可判定 gate。
**輸入：** 1 至 3 全部產出、凍結 reference profile。
**產出：** Desktop binary、visual/interaction report、evidence index entries。
**依賴：** 1.1、1.2、2.1、2.2、2.3、3.1、3.2、3.3、3.4。
**Owner／Wave：** Desktop owner／Wave 4A exit。
**Gate／Evidence：** `G-DESKTOP`、`G-A11Y-I18N`、`G-DPI-MONITOR`；`evidence/artifacts/4.1/`。
**完成門檻：** 所有 desktop scenarios 通過，無 stale/blocked/N/A leaf，resource baseline 恢復。

- [ ] 4.1.1 執行單螢幕 100% 至 200% DPI visual geometry matrix。
- [ ] 4.1.2 執行虛擬多螢幕 topology 與 desktop interaction matrix。
- [ ] 4.1.3 執行 accessibility identity、focus 與 action scan。
- [ ] 4.1.4 執行 desktop resource teardown 與 cache-bound verification。
- [ ] 4.1.5 索引全部 artifact hashes 並發布 `G-DESKTOP` disposition。
- [ ] 4.1.6 產生 desktop public/effect schema、binary、evidence-index 與 handoff SHA-256 manifest。
