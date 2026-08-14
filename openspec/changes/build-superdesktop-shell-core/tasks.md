## 1. Typed State 與 Reducer

### 1.1 建立 Shell State 與 Identity Contract

**目的：** 建立不含 HWND/COM 的唯一狀態權威與穩定 identity。
**輸入：** Bootstrap crates、capability go disposition、核准 core spec。
**產出：** `shell-core` state/identity/event/command types。
**依賴：** `validate-superdesktop-windows-platform` go。
**Owner／Wave：** Core owner／Wave 3。
**Gate／Evidence：** `G-ARCH`、`G-TRACE`；`evidence/artifacts/1.1/`。
**完成門檻：** UI crates 只使用 owned types，identity serialization 與 equality 測試通過。

- [x] 1.1.1 定義 monitor、shell item、window、application、selection 與 lifecycle identities。
- [x] 1.1.2 定義 ShellState、ShellEvent、ShellCommand 與 typed terminal results。
- [x] 1.1.3 加入 architecture test 禁止 core 持有 HWND、PIDL 或 COM interface。
- [x] 1.1.4 加入 identity round-trip、collision 與 stable ordering tests。
- [x] 1.1.5 定義並測試 Wave 4 共用的 bridge launch/result/repair DTO 與 accessibility-safe message keys。

### 1.2 實作 Pure Reducer 與 Generation Fencing

**目的：** 讓相同事件序列產生相同 state/command，且 stale completion 無法修改目前狀態。
**輸入：** 1.1 contracts。
**產出：** Pure reducer、generation/request fencing、determinism tests。
**依賴：** 1.1。
**Owner／Wave：** Core owner／Wave 3。
**Gate／Evidence：** `G-ARCH`、`G-SAFETY`；`evidence/artifacts/1.2/`。
**完成門檻：** Replay deterministic；stale、cancelled 與 duplicate terminal fixtures 不改變 active generation。

- [x] 1.2.1 實作純 reducer 與 effect-command emission。
- [x] 1.2.2 實作 request_id、generation 與 exactly-once terminal bookkeeping。
- [x] 1.2.3 加入 stale-success-after-refresh 測試。
- [x] 1.2.4 加入 cancel-vs-success 與 duplicate-terminal 測試。
- [x] 1.2.5 加入 event-log replay determinism 測試並保存 state hashes。

## 2. Queue 與權威對帳

### 2.1 實作 Bounded Queue 與 Coalescing

**目的：** 在事件洪水下限制記憶體並保留不可合併的 terminal/lifecycle 事件。
**輸入：** 1.1 events、queue bound 決策。
**產出：** Bounded queue、coalescing policy、overflow event。
**依賴：** 1.1。
**Owner／Wave：** Core owner／Wave 3。
**Gate／Evidence：** `G-SAFETY`、`G-PERF`；`evidence/artifacts/2.1/`。
**完成門檻：** Queue 永不超界，coalescing deterministic，terminal/lifecycle event 不遺失。

- [x] 2.1.1 實作 bounded queue 與明確容量常數。
- [x] 2.1.2 實作依 identity 合併的可合併事件策略。
- [x] 2.1.3 保護 terminal、overflow 與 lifecycle event 不被合併或丟棄。
- [x] 2.1.4 執行 event-storm property tests 並保存最大 queue depth。

### 2.2 實作 Overflow Reconciliation

**目的：** Queue/watcher overflow 後以權威 snapshot 收斂而不套用 stale delta。
**輸入：** 1.2 reducer、2.1 overflow event、fake platform snapshot API。
**產出：** Reconciliation state machine 與 fixtures。
**依賴：** 1.2、2.1。
**Owner／Wave：** Core owner／Wave 3。
**Gate／Evidence：** `G-DESKTOP`、`G-TASKBAR`；`evidence/artifacts/2.2/`。
**完成門檻：** Overflow 只觸發一次 active refresh，late delta 被拒絕，selection/order 可依 stable identity 恢復。

- [x] 2.2.1 實作 authoritative snapshot request/response command flow。
- [x] 2.2.2 實作 overflow refresh 去重與 generation rollover。
- [x] 2.2.3 加入 watcher overflow 後 selection restore 測試。
- [x] 2.2.4 加入 window-event overflow 後 stable grouping/order 測試。
- [x] 2.2.5 驗證 late snapshot/delta suppression 並保存 event trace。

## 3. Settings Store

### 3.1 實作 Settings v1 Schema 與 Migration

**目的：** 保存核准欄位並確保 execution-mode preference 不繞過 Shell opt-in。
**輸入：** Settings spec、core identities。
**產出：** Versioned schema、defaults、migration、round-trip tests。
**依賴：** 1.1。
**Owner／Wave：** Settings owner／Wave 3。
**Gate／Evidence：** `G-SAFETY`、`G-TRACE`；`evidence/artifacts/3.1/`。
**完成門檻：** 所有 v1 欄位 round-trip；舊/未知欄位可預測 migration；Shell mode 仍需每次明確 opt-in。

- [x] 3.1.1 定義 wallpaper、desktop positions、monitor mapping、taskbar rows/pins、theme 與 accessibility 欄位。
- [x] 3.1.2 定義 SuperExplorer path 與 execution-mode preference 欄位及安全 defaults。
- [x] 3.1.3 實作 v1 serialization、deserialization 與 unknown-field preservation policy。
- [x] 3.1.4 實作舊版 migration 與 unsupported-future-version refusal。
- [x] 3.1.5 加入完整 round-trip 與 execution-mode opt-in protection tests。

### 3.2 實作原子寫入、Quarantine 與 Fixture Root

**目的：** 確保中斷寫入可恢復、壞檔不覆蓋正常設定、測試不碰使用者資料。
**輸入：** 3.1 schema、platform filesystem adapter。
**產出：** Atomic store、quarantine records、fixture-root guard。
**依賴：** 3.1。
**Owner／Wave：** Settings owner／Wave 3。
**Gate／Evidence：** `G-SAFETY`；`evidence/artifacts/3.2/`。
**完成門檻：** Crash points 保留舊或新完整檔；壞資料有 timestamped quarantine；越界測試寫入被拒絕。

- [x] 3.2.1 實作 temp-write、flush、atomic replace 與 directory durability contract。
- [x] 3.2.2 對各 atomic-write crash point 建立復原測試。
- [x] 3.2.3 實作局部欄位 fallback 與完整檔案 quarantine。
- [x] 3.2.4 加入 malformed、unknown-version 與 partial-file fixtures。
- [x] 3.2.5 實作 canonical fixture-root guard 與 reparse/path-escape tests。
- [x] 3.2.6 保存設定復原、quarantine 與零使用者資料 mutation evidence。

## 4. Core Contract 發布

### 4.1 固定 Public Contract 與 Consumer Fixtures

**目的：** 讓 desktop、taskbar、bridge 與 lifecycle changes 依賴同一版 core contract。
**輸入：** 1 至 3 全部實作與測試。
**產出：** Public API docs、fake adapters、contract hash、consumer fixtures。
**依賴：** 1.1、1.2、2.1、2.2、3.1、3.2。
**Owner／Wave：** Core owner／Wave 3 exit。
**Gate／Evidence：** `G-ARCH`、`G-TRACE`；`evidence/artifacts/4.1/`。
**完成門檻：** Public API hash 固定，四個 consumer fixture 編譯，workspace quality gates 通過。

- [x] 4.1.1 建立 fake platform/effect adapters 與 deterministic fixture builders。
- [x] 4.1.2 建立 desktop、taskbar、bridge、lifecycle consumer compile fixtures。
- [ ] 4.1.3 以 SHA-256 雜湊 canonical public API、DTO/effect schema 與 fake-adapter input manifest，產生 shell-core contract hash。
- [ ] 4.1.4 執行 fmt、check、clippy 與 core tests 並索引 evidence。
- [ ] 4.1.5 產生 core handoff manifest，記錄 contract hash、base/result revision、consumer 與 gate disposition。
