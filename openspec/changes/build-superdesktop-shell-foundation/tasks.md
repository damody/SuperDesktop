# SuperDesktop M0 Program Roll-up Tasks

此 program change 不實作產品程式碼，也不代替 child tasks。每個 leaf 只驗證一個 roll-up 結果；使用者已明確要求開始 apply，完成 Wave 0 readiness 與規劃基準後依序執行。所有 leaf 為 mandatory，外部環境缺失只能 blocked。

## 1. Program Readiness

### 1.1 驗證 Dependency DAG 與多代理協議

**目的：** 在 apply 前證明八個 child changes、ownership、handoff 與外部 prerequisites 可直接執行。
**輸入：** 九個 OpenSpec changes、`EXECUTION.md`、核准設計。
**產出：** Dependency/coverage/ownership report 與 planning review disposition。
**依賴：** 無。
**Owner／Wave：** Primary integrator／Wave 0 readiness 與規劃基準。
**Gate／Evidence：** `G-TRACE`；program `evidence/artifacts/1.1/`。
**完成門檻：** DAG 無循環、每個 shared file/contract 唯一 owner、所有 P0/P1 已修正，外部 prerequisites 狀態已明列。

- [x] 1.1.1 驗證八個 child change 各自具 proposal、design、specs、tasks 與 apply-required status complete。
- [x] 1.1.2 驗證 Wave 1→2→3→4A/4B/4C→5→6 dependency DAG 無循環或隱藏 predecessor。
- [x] 1.1.3 驗證每個 shared contract/file、program artifact 與 evidence index 有唯一 writer。
- [x] 1.1.4 驗證所有 program requirement/scenario/gate 映射到 child coverage manifest。
- [x] 1.1.5 記錄 monolith-to-program B 級 lineage 與舊 task replacement mapping。
- [x] 1.1.6 驗證 Windows 10 22H2 與實體 mixed-DPI 環境的 availability/schedule；缺失時預先標記 Wave 6 external blocked。
- [x] 1.1.7 由 Independent reviewer 簽署無未解 P0/P1 的 planning disposition。

## 2. Foundation Roll-up

### 2.1 驗證並封存 Workspace Bootstrap

**目的：** Roll up `bootstrap-superdesktop-workspace` 的架構、離線建置與 evidence governance 結果。
**輸入：** Bootstrap change-local tasks、evidence、handoff manifest。
**產出：** Bootstrap archive revision 與 program hash record。
**依賴：** 1.1；使用者明確要求開始 apply。
**Owner／Wave：** Primary integrator／Wave 1 exit。
**Gate／Evidence：** `G-ARCH`、`G-TRACE`；program `evidence/artifacts/2.1/`。
**完成門檻：** Strict/tasks validation、全部 mandatory evidence、兩個 gates、handoff 與 archive revision 分項通過。

- [x] 2.1.1 記錄 child `openspec validate --strict` 成功輸出。
- [x] 2.1.2 記錄 child 詳細 tasks validator 成功輸出。
- [x] 2.1.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [x] 2.1.4 驗證 `G-ARCH` disposition passed。
- [x] 2.1.5 驗證 `G-TRACE` disposition passed。
- [x] 2.1.6 驗證 workspace/toolchain/source/evidence handoff manifest hashes。
- [x] 2.1.7 封存 child 並記錄 immutable archive revision。

### 2.2 驗證並封存 Windows/GPUI Capability Gate

**目的：** Roll up capability spike、platform-common hash、Safe Mode 與 go disposition。
**輸入：** Capability change-local tasks/evidence、2.1 archive revision。
**產出：** Capability archive revision、platform-common API/ABI hash、GO record。
**依賴：** 2.1。
**Owner／Wave：** Primary integrator／Wave 2 exit。
**Gate／Evidence：** `G-ARCH`、`G-SHELL-TAKEOVER-CAPABILITY`、`G-DPI-MONITOR` capability、`G-GUARDIAN-RECOVERY-CAPABILITY`；program `evidence/artifacts/2.2/`。
**完成門檻：** Strict/tasks validation、mandatory evidence、每個 capability gate、common hash 與 GO 分項通過。

- [x] 2.2.1 記錄 child `openspec validate --strict` 成功輸出。
- [x] 2.2.2 記錄 child 詳細 tasks validator 成功輸出。
- [x] 2.2.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [x] 2.2.4 驗證 HWND/AppBar/Shell Hook/Start/DPI capability dispositions passed。
- [x] 2.2.5 驗證 guardian lease、FFI wrapper與 Safe Mode/unsupported-session dispositions passed。
- [x] 2.2.6 驗證 platform-common API/ABI SHA-256 handoff manifest。
- [x] 2.2.7 驗證 signed capability GO disposition。
- [ ] 2.2.8 封存 child 並記錄 immutable archive revision。

## 3. Product Contract Roll-up

### 3.1 驗證並封存 Shell Core

**目的：** Roll up pure state/reconciliation/settings 與 Wave 4 共用 DTO contract。
**輸入：** Core change-local tasks/evidence、2.2 GO/common hash。
**產出：** Core archive revision 與 public contract hash。
**依賴：** 2.2。
**Owner／Wave：** Primary integrator／Wave 3 exit。
**Gate／Evidence：** `G-ARCH`、`G-TRACE`、core reconciliation；program `evidence/artifacts/3.1/`。
**完成門檻：** Strict/tasks validation、mandatory evidence、core gates、contract/handoff hash 與 archive 分項通過。

- [x] 3.1.1 記錄 child `openspec validate --strict` 成功輸出。
- [x] 3.1.2 記錄 child 詳細 tasks validator 成功輸出。
- [x] 3.1.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [x] 3.1.4 驗證 reducer/reconciliation/settings/fixture contract dispositions passed。
- [x] 3.1.5 驗證包含 bridge DTO 的 shell-core public contract SHA-256 manifest。
- [x] 3.1.6 驗證 core handoff manifest 已由所有 Wave 4 consumers 接受。
- [ ] 3.1.7 封存 child 並記錄 immutable archive revision。

### 3.2 驗證並封存 GPUI Desktop

**目的：** Roll up desktop surface、namespace、fixed entry、activation、watcher 與 a11y 結果。
**輸入：** Desktop change-local tasks/evidence、3.1 contract hash。
**產出：** Desktop archive revision 與 public/effect/binary/evidence hashes。
**依賴：** 3.1；可與 3.3、3.4 平行。
**Owner／Wave：** Primary integrator／Wave 4A exit。
**Gate／Evidence：** `G-DESKTOP`、`G-A11Y-I18N`、`G-DPI-MONITOR`、`G-EXPLORER-BRIDGE` contract；program `evidence/artifacts/3.2/`。
**完成門檻：** Strict/tasks validation、mandatory evidence、明列 gates、handoff hash 與 archive 分項通過。

- [x] 3.2.1 記錄 child `openspec validate --strict` 成功輸出。
- [x] 3.2.2 記錄 child 詳細 tasks validator 成功輸出。
- [x] 3.2.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [x] 3.2.4 驗證 `G-DESKTOP`、desktop `G-A11Y-I18N` 與 `G-DPI-MONITOR` dispositions passed。
- [x] 3.2.5 驗證 desktop fixed-entry/folder bridge contract disposition passed。
- [x] 3.2.6 驗證 desktop public/effect/binary/evidence SHA-256 handoff manifest。
- [ ] 3.2.7 封存 child 並記錄 immutable archive revision。

### 3.3 驗證並封存 GPUI Taskbar

**目的：** Roll up AppBar、window tracking、layout、Start、fixed entry 與 a11y 結果。
**輸入：** Taskbar change-local tasks/evidence、3.1 contract hash。
**產出：** Taskbar archive revision 與 public/effect/binary/evidence hashes。
**依賴：** 3.1；可與 3.2、3.4 平行。
**Owner／Wave：** Primary integrator／Wave 4B exit。
**Gate／Evidence：** `G-TASKBAR`、`G-A11Y-I18N`、`G-DPI-MONITOR`、`G-EXPLORER-BRIDGE` contract；program `evidence/artifacts/3.3/`。
**完成門檻：** Strict/tasks validation、mandatory evidence、明列 gates、handoff hash 與 archive 分項通過。

- [x] 3.3.1 記錄 child `openspec validate --strict` 成功輸出。
- [x] 3.3.2 記錄 child 詳細 tasks validator 成功輸出。
- [x] 3.3.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [x] 3.3.4 驗證 `G-TASKBAR`、taskbar `G-A11Y-I18N` 與 `G-DPI-MONITOR` dispositions passed。
- [x] 3.3.5 驗證 Start 與 fixed SuperExplorer entry contract dispositions passed。
- [x] 3.3.6 驗證 taskbar public/effect/binary/evidence SHA-256 handoff manifest。
- [ ] 3.3.7 封存 child 並記錄 immutable archive revision。

### 3.4 驗證並封存 SuperExplorer Bridge

**目的：** Roll up resolver、launch、deadline/cancel、cleanup、repair 與 repository-integrity 結果。
**輸入：** Bridge change-local tasks/evidence、3.1 contract hash。
**產出：** Bridge archive revision 與 public/effect/binary/evidence hashes。
**依賴：** 3.1；可與 3.2、3.3 平行。
**Owner／Wave：** Primary integrator／Wave 4C exit。
**Gate／Evidence：** `G-EXPLORER-BRIDGE`、`G-SAFETY`；program `evidence/artifacts/3.4/`。
**完成門檻：** Strict/tasks validation、mandatory evidence、兩個 gates、handoff hash 與 archive 分項通過。

- [x] 3.4.1 記錄 child `openspec validate --strict` 成功輸出。
- [x] 3.4.2 記錄 child 詳細 tasks validator 成功輸出。
- [x] 3.4.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [x] 3.4.4 驗證 `G-EXPLORER-BRIDGE` disposition passed。
- [x] 3.4.5 驗證 bridge `G-SAFETY` 與 SuperExplorer repository-integrity dispositions passed。
- [x] 3.4.6 驗證 bridge public/effect/binary/evidence SHA-256 handoff manifest。
- [ ] 3.4.7 封存 child 並記錄 immutable archive revision。

## 4. Lifecycle 與 Release Roll-up

### 4.1 驗證並封存 Shell Takeover/Recovery

**目的：** Roll up composition、single-owner、transaction、guardian、shutdown 與 reference provisional gates。
**輸入：** Lifecycle change-local tasks/evidence、3.2/3.3/3.4 archive/handoff hashes。
**產出：** Lifecycle archive revision、provisional dispositions 與 public/effect/binary/evidence hashes。
**依賴：** 3.2、3.3、3.4。
**Owner／Wave：** Primary integrator／Wave 5 exit。
**Gate／Evidence：** `G-SHELL-TAKEOVER-PROVISIONAL`、`G-GUARDIAN-RECOVERY-PROVISIONAL`、`G-SAFETY`；program `evidence/artifacts/4.1/`。
**完成門檻：** Strict/tasks validation、mandatory evidence、三個 dispositions、handoff hash 與 archive 分項通過；final Windows 10 gates仍未判定。

- [x] 4.1.1 記錄 child `openspec validate --strict` 成功輸出。
- [x] 4.1.2 記錄 child 詳細 tasks validator 成功輸出。
- [x] 4.1.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [x] 4.1.4 驗證 reference `G-SHELL-TAKEOVER-PROVISIONAL` disposition passed。
- [x] 4.1.5 驗證 reference `G-GUARDIAN-RECOVERY-PROVISIONAL` disposition passed。
- [x] 4.1.6 驗證 lifecycle `G-SAFETY` disposition passed。
- [x] 4.1.7 驗證 lifecycle public/effect/binary/evidence SHA-256 handoff manifest。
- [ ] 4.1.8 封存 child 並記錄 immutable archive revision。

### 4.2 驗證並封存 M0 Release Verification

**目的：** Roll up reference、Windows 10、DPI、實體螢幕、a11y/i18n、stability、performance、安全與 traceability 最終結果。
**輸入：** Verification change-local tasks/evidence、4.1 archive revision、外部 Windows 10 與實體 mixed-DPI evidence。
**產出：** Verification archive revision 與最終 M0 release disposition。
**依賴：** 4.1；外部 prerequisites 必須已解除 blocked。
**Owner／Wave：** Primary integrator 接受；Independent reviewer 簽署／Wave 6 exit。
**Gate／Evidence：** `G-ARCH`、`G-DESKTOP`、`G-TASKBAR`、`G-EXPLORER-BRIDGE`、`G-SHELL-TAKEOVER`、`G-GUARDIAN-RECOVERY`、`G-DPI-MONITOR`、`G-A11Y-I18N`、`G-PERF`、`G-SAFETY`、`G-TRACE`；program `evidence/artifacts/4.2/`。
**完成門檻：** Strict/tasks validation與每個列名 gate 分項通過；Windows 10、實體 mixed-DPI、P0/P1、handoff、archive 皆具有效終態。

- [ ] 4.2.1 記錄 child `openspec validate --strict` 成功輸出。
- [ ] 4.2.2 記錄 child 詳細 tasks validator 成功輸出。
- [ ] 4.2.3 驗證 child mandatory evidence index 無 failed/blocked/stale/N/A/invalid replacement。
- [ ] 4.2.4 驗證 reference visual、`G-DESKTOP`、`G-TASKBAR` 與 `G-EXPLORER-BRIDGE` passed。
- [ ] 4.2.5 驗證 Windows 10 `G-SHELL-TAKEOVER` 與 `G-GUARDIAN-RECOVERY` final dispositions passed。
- [ ] 4.2.6 驗證虛擬與實體 `G-DPI-MONITOR` passed。
- [ ] 4.2.7 驗證 `G-A11Y-I18N` passed。
- [ ] 4.2.8 驗證 `G-PERF` passed。
- [ ] 4.2.9 驗證 `G-SAFETY` 與 `G-ARCH` passed。
- [ ] 4.2.10 驗證 `G-TRACE`、coverage、replacement 與 corrective lineage passed。
- [ ] 4.2.11 驗證 Independent reviewer 報告無未解 P0/P1。
- [ ] 4.2.12 封存 verification child 並記錄 immutable archive revision。
- [ ] 4.2.13 發布所有列名 gates passed、無 blocked/stale/N/A 的 M0 release disposition。
