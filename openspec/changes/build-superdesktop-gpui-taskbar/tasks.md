## 1. Per-monitor AppBar 與 Layout

### 1.1 建立 GPUI AppBar Lifecycle

**目的：** 為每個 monitor 建立可 reserve/restore work area 的 GPUI taskbar。
**輸入：** Capability go、core contract、AppBar adapter。
**產出：** Taskbar windows、AppBar ownership、lifecycle tests。
**依賴：** `build-superdesktop-shell-core` contract hash。
**Owner／Wave：** Taskbar platform owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`、`G-DPI-MONITOR`；`evidence/artifacts/1.1/`。
**完成門檻：** 每個 monitor 恰一 AppBar，topology/DPI 變更正確 reserve，teardown 恢復 baseline。

- [x] 1.1.1 實作 monitor identity 到 GPUI taskbar HWND 的 lifecycle mapping。
- [x] 1.1.2 實作 AppBar register/query/position/remove effect adapter。
- [x] 1.1.3 實作 monitor add/remove、primary change 與 DPI change reconciliation。
- [x] 1.1.4 加入 register failure 與重複 teardown recovery tests。
- [x] 1.1.5 保存各 monitor work-area before/after 與 HWND evidence。

### 1.2 實作一至三列 Win10 參考 Layout

**目的：** 模仿凍結 ExplorerPatcher 工作列的一至三列排列並預設雙列。
**輸入：** 1.1 surfaces、settings row count、reference profile。
**產出：** DPI-aware layout engine、visual fixtures。
**依賴：** 1.1。
**Owner／Wave：** Taskbar UI owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`、`G-A11Y-I18N`；`evidence/artifacts/1.2/`。
**完成門檻：** 1/2/3 列皆無重疊，預設雙列，窄寬度有 deterministic overflow。

- [x] 1.2.1 實作一、二、三列高度與 row allocation。
- [x] 1.2.2 實作 Start、task groups、pinned entry 與 status region slot ordering。
- [x] 1.2.3 實作 DPI scaling、text truncation 與 hit-target geometry。
- [x] 1.2.4 實作窄寬度 overflow 與 priority policy。
- [x] 1.2.5 產生 reference-size visual fixtures 與 geometry assertions。

## 2. Window Tracking 與 Task Semantics

### 2.1 實作權威 Window Tracker

**目的：** 以 Shell Hook 增量事件與 EnumWindows snapshot 維護可見 task state。
**輸入：** Core reconciliation、Shell Hook/EnumWindows adapters。
**產出：** Window tracker、filters、event traces。
**依賴：** 1.1、core reconciliation。
**Owner／Wave：** Window tracking owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`、`G-SAFETY`；`evidence/artifacts/2.1/`。
**完成門檻：** Normal windows 正確呈現；invisible/tool/cloaked/owned transient 排除；overflow 可收斂。

- [x] 2.1.1 對 Shell Hook callback 套用 Wave 2 凍結的 no-unwind wrapper 並轉成 owned event。
- [x] 2.1.2 實作 EnumWindows authoritative snapshot 與 stable WindowId/ApplicationId mapping。
- [x] 2.1.3 實作 invisible、tool、cloaked 與 owned transient filters。
- [x] 2.1.4 實作 overflow refresh 與 stale event suppression。
- [x] 2.1.5 執行 create/destroy/title/icon/attention event-storm tests。
- [x] 2.1.6 保存 raw hook trace、snapshot diff 與 max queue depth。

### 2.2 實作 Grouping、Order 與 Pinning

**目的：** 讓 application groups 與使用者 pin order 在 refresh/重啟後穩定。
**輸入：** 2.1 tracker、settings pin order。
**產出：** Grouping/order model、persisted pins、fixtures。
**依賴：** 2.1。
**Owner／Wave：** Taskbar model owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`；`evidence/artifacts/2.2/`。
**完成門檻：** Identity 相同正確 group，identity 不同不誤合併，pin order round-trip 穩定。

- [x] 2.2.1 實作 ApplicationId grouping 與 fallback identity policy。
- [x] 2.2.2 實作 stable group/window ordering 與 attention placement。
- [x] 2.2.3 實作 pin/unpin/reorder commands 與 settings persistence。
- [x] 2.2.4 加入 identity collision、process restart 與 snapshot reorder tests。
- [x] 2.2.5 加入 pin order restart round-trip test。

### 2.3 實作 Task Click 與 Keyboard/UIA Semantics

**目的：** 實作 activate、minimize、restore 與 group selection 的基本 Windows 語意。
**輸入：** 2.1 tracker、2.2 groups、foreground adapter。
**產出：** Interaction commands、focus/UIA contracts、headful tests。
**依賴：** 2.1、2.2。
**Owner／Wave：** Taskbar interaction owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`、`G-A11Y-I18N`；`evidence/artifacts/2.3/`。
**完成門檻：** Inactive/active/minimized/group cases 符合 spec，pointer/keyboard/UIA 等價。

- [x] 2.3.1 實作 inactive window activate 與 foreground request。
- [x] 2.3.2 實作 active window minimize 與 minimized window restore。
- [x] 2.3.3 實作 multi-window group selection surface 與 escape/focus return。
- [x] 2.3.4 實作 task accessible name/role/state/action 與 keyboard navigation。
- [x] 2.3.5 執行 pointer、keyboard 與 UIA action matrix。

## 3. 固定入口與狀態區

### 3.1 實作固定 SuperExplorer 入口

**目的：** 工作列永遠顯示 truthful 的 SuperExplorer 固定入口並路由到 bridge contract。
**輸入：** Taskbar layout、Wave 3 凍結的 bridge launch/result/repair DTO 與 fake adapter。
**產出：** Pinned entry view、activation binding、repair prompt。
**依賴：** 1.2、shell-core contract；可先以 fake bridge 實作。
**Owner／Wave：** Taskbar UI owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`、`G-EXPLORER-BRIDGE`、`G-A11Y-I18N`；`evidence/artifacts/3.1/`。
**完成門檻：** Entry 在所有列數/DPI 存在；pointer/keyboard/UIA 可啟動；failure 顯示 repair UI。

- [x] 3.1.1 實作固定 SuperExplorer entry 的 icon、label 與 stable identity。
- [x] 3.1.2 綁定 pointer activation 到 core bridge command。
- [x] 3.1.3 綁定 keyboard 與 UIA invoke 到相同 command。
- [x] 3.1.4 實作 validation/spawn/timeout failure 的 GPUI repair prompt。
- [x] 3.1.5 建立所有 row-count/DPI 的 existence 與 activation tests。

### 3.2 實作 Start Host 與 Truthful Failure

**目的：** 呼叫凍結 ExplorerPatcher profile 的 Start host，無法使用時不假裝成功。
**輸入：** Capability spike Start probe、Start effect adapter。
**產出：** Start control、typed availability、repair UI。
**依賴：** 1.2、capability go。
**Owner／Wave：** Taskbar platform owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`、`G-SAFETY`；`evidence/artifacts/3.2/`。
**完成門檻：** Available host 可呼叫；missing/refused host 顯示 truthful unavailable 且不 mutation 系統設定。

- [x] 3.2.1 實作 Start host identity revalidation 與 invocation adapter。
- [x] 3.2.2 實作 Start control pointer、keyboard 與 UIA activation。
- [x] 3.2.3 實作 missing/refused/stale-host typed failure UI。
- [x] 3.2.4 執行 reference profile preview mode 的 Start probe/invocation headful test。
- [x] 3.2.5 建立 Shell-mode Start invocation fixture；真實執行由 Wave 5 接管健康/整合 gate 完成。

### 3.3 實作時鐘與核心狀態區

**目的：** 呈現可驗證的時鐘、日期與核心系統狀態，不冒充未實作 tray provider。
**輸入：** Clock/status adapters、reference layout。
**產出：** Status region、localized formats、availability states。
**依賴：** 1.2。
**Owner／Wave：** Taskbar UI owner／Wave 4B。
**Gate／Evidence：** `G-TASKBAR`、`G-A11Y-I18N`；`evidence/artifacts/3.3/`。
**完成門檻：** 時鐘/日期更新正確，狀態 available/unavailable truthful，未支援 tray 不顯示假 icon。

- [x] 3.3.1 實作時鐘與日期 tick/update scheduling。
- [x] 3.3.2 實作 locale-aware zh-TW/en formatting 與 deterministic test clock。
- [x] 3.3.3 實作 volume/network 等核准核心狀態的 availability mapping。
- [x] 3.3.4 加入 provider unavailable 與 no-fake-tray tests。

## 4. Taskbar Gate

### 4.1 完成 Taskbar Headful Contract Gate

**目的：** 整合 AppBar、layout、window tracking、fixed entry、Start 與狀態區。
**輸入：** 1 至 3 全部產出、凍結 reference profile。
**產出：** Taskbar binary、visual/interaction report、gate disposition。
**依賴：** 1.1、1.2、2.1、2.2、2.3、3.1、3.2、3.3。
**Owner／Wave：** Taskbar owner／Wave 4B exit。
**Gate／Evidence：** `G-TASKBAR`、`G-DPI-MONITOR`、`G-A11Y-I18N`；`evidence/artifacts/4.1/`。
**完成門檻：** 所有 taskbar scenarios 通過，work area 可恢復，無 stale/blocked/N/A leaf。

- [x] 4.1.1 執行 1/2/3 列與 100% 至 200% DPI visual geometry matrix。
- [x] 4.1.2 執行 window lifecycle、grouping 與 task-click interaction matrix。
- [x] 4.1.3 執行 SuperExplorer fixed entry 與 Start host headful matrix。
- [x] 4.1.4 執行 accessibility focus、identity 與 action scan。
- [x] 4.1.5 執行 AppBar teardown、handle 與 cache-bound verification。
- [x] 4.1.6 索引全部 artifact hashes 並發布 `G-TASKBAR` disposition。
- [x] 4.1.7 產生 taskbar public/effect schema、binary、evidence-index 與 handoff SHA-256 manifest。
