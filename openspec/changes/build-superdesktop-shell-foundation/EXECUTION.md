# SuperDesktop M0 多代理執行協議

本文件定義整個 program 的自主執行方式。除 C 級變更外，執行期間不要求使用者逐項確認。使用者已明確核准開始 apply；產品 workspace 與 Rust 產品碼可依 wave gate 建立，但在 lifecycle capability 與 admission gate 通過前，仍不得啟動 Shell mode 或執行會改變桌面/工作列的 headful test。

## 決策權限

- **A 級技術細化：** 只能調整 leaf 拆分、順序、owner、命令或證據收集方式；不得改範圍、requirement、public contract、gate、threshold、平台、安全/權限邊界或必要證據。Task owner 可提出並記錄，Primary integrator 驗證分類後繼續。
- **B 級矛盾修正：** 只能在已核准範圍與公開承諾內修正錯誤假設。Primary integrator 暫停受影響分支，同步修正 design/spec/tasks、標記舊 evidence stale、建立 replacement lineage後繼續；不需逐項詢問使用者。矛盾本身不會覆蓋 C 級判準。
- **C 級重大變更：** 任何範圍、公開承諾、blocking gate、threshold、必要證據、平台/框架、權限、外部寫入、破壞性操作或外部授權變更都優先視為 C，無論是增加、降低或聲稱在修矛盾。受影響分支停止並請使用者決定，其他無關分支可繼續。
- Exact Windows 11＋ExplorerPatcher reference profile 漂移或缺少實體 mixed-DPI 雙螢幕時，對應 release leaf 保持 `blocked`；不得用 N/A、虛擬結果或推測代替。

## 固定角色

| 角色 | Ownership | 不得做的事 |
| --- | --- | --- |
| Primary integrator | Program graph、shared contracts、跨 change 整合、B/C disposition、最終驗證 | 不得把未通過或 blocked 工作宣告完成 |
| Workspace/Build owner | Bootstrap、toolchain、dependency、offline build、identity | 不得改 UI 或 lifecycle contract |
| Windows platform owner | Capability spike、Win32 adapters、HWND/AppBar/DPI/probes | 不得繞過 capability stop gate |
| Core owner | `shell-core`、settings contracts、fake adapters | 不得持有 HWND/PIDL/COM 或實作 UI |
| Desktop owner | `desktop-ui` 與 desktop platform effects | 不得修改 taskbar/guardian owned files |
| Taskbar owner | `taskbar-ui`、window tracker、AppBar UI | 不得修改 bridge process lifecycle |
| Explorer bridge owner | `explorer-bridge` 與 SuperExplorer process launch | 不得修改 `D:\SuperExplorer` repository |
| Lifecycle/Guardian owner | Shell admission/takeover/recovery、guardian、FFI safety | 不得修改登入 Shell registry/policy |
| Verification owners | UI/display/a11y/perf/safety test harness 與 gate evidence | 不得修正 production code而不回原 owner |
| Independent reviewer | P0/P1、coverage、evidence lineage 複核 | 不得兼任 remediation owner |

所有代理都不是 repository 中的唯一工作者：只能修改自己 ownership 內的檔案，不得回復他人變更；shared contract 只能由 Core owner 或 Primary integrator 合併。

## 代理類型與併發上限

- **Primary agent（固定 1 個）：** 唯一 coordinator/integrator；保留一個 slot，負責派工、驗證交接、合併 shared contract、執行權限敏感操作與決定 A/B disposition。
- **Core implementer（最多 1 個）：** 用於 Wave 2、3、5 的 Win32/GPUI ABI、core 或 lifecycle 高風險工作；不得同時派第二個代理修改相同 shared crate。
- **Worker（最多 3 個）：** Wave 4 分別固定為 Desktop、Taskbar、Explorer bridge owner，可平行處理互不重疊的 modules。
- **Mechanical worker（最多 2 個）：** 只處理 fixtures、manifests、scripts、evidence packaging 與文件；不得改 public contract 或 production behavior。
- **Architecture reviewer（最多 1 個）：** 唯讀，於 wave exit 與最終 release 複核；不得修正 production code。

系統同時最多四個 agent（包含 Primary agent），因此最多三個 subagent。Primary agent 不因派工而放棄整合責任；每個 subagent 完成後，Primary agent必須驗證交接並更新剩餘 graph。

## 檔案 Ownership 與 Shared-file 規則

| Owner | 專屬路徑 | Shared 路徑處理 |
| --- | --- | --- |
| Workspace/Build | root `Cargo.toml`、`Cargo.lock`、toolchain、build/evidence scripts | 只有此 owner 可改 root manifests；新增 dependency 由 Primary integrator 合併 |
| Windows platform | `crates/platform-win/src/common/`、capability spike | Wave 2 GO 前凍結 common API/ABI hash；變更先回 Core/Platform owner |
| Core | `crates/shell-core/`、`crates/settings-store/`、public DTO/effect contracts | 唯一可修改 shared contract 的 production owner |
| Desktop | `crates/desktop-ui/`、`crates/platform-win/src/desktop/` | 不得改 common/taskbar/bridge/lifecycle modules |
| Taskbar | `crates/taskbar-ui/`、`crates/platform-win/src/taskbar/` | 不得改 common/desktop/bridge/lifecycle modules |
| Explorer bridge | `crates/explorer-bridge/`、`crates/platform-win/src/bridge/` | 不得修改 `D:\SuperExplorer` |
| Lifecycle/Guardian | `crates/superdesktop-app/` composition、`crates/superdesktop-guardian/`、`crates/platform-win/src/lifecycle/` | 只在 Wave 4 contracts 凍結後整合 |
| Test/Verification | `crates/superdesktop-test-support/`、change-local test/evidence scripts | Production 修正必須交回原 owner |

若實際 skeleton 與上述路徑不同，Wave 1 必須先由 Primary integrator 以 A 級修正同步更新本表與 child tasks；不得讓兩個平行代理共同擁有同一檔案。任何 shared file 修改都走「owner 提案 → Primary integrator 合併 → contract hash 更新 → dependent evidence stale」流程。

## 執行波次與硬性依賴

```text
Wave 0  OpenSpec 與 multi-agent plan（建立基準並進入 apply）
   ↓ 使用者明確要求開始 apply
Wave 1  bootstrap-superdesktop-workspace
   ↓ G-ARCH/G-TRACE passed
Wave 2  validate-superdesktop-windows-platform
   ↓ capability GO
Wave 3  build-superdesktop-shell-core
   ↓ frozen public contract hash
Wave 4  desktop ─┬─ taskbar ─┬─ bridge（可平行）
                  └───────────┘
   ↓ 三個 changes 全部 passed
Wave 5  add-superdesktop-shell-takeover-recovery
   ↓ reference takeover/recovery provisional passed；final pending
Wave 6  verify-superdesktop-m0
   ↓ 所有 mandatory release gates passed
Program archive
```

任一 change 只有在 `openspec validate --strict`、詳細 tasks validator、該 change mandatory tasks 與其 wave-exit dispositions 全部通過後才能封存。Wave 5 只產生 reference-profile provisional takeover/recovery dispositions；最終 `G-SHELL-TAKEOVER` 與 `G-GUARDIAN-RECOVERY` 仍須 Wave 6 在同一 exact profile 完成 lifecycle/installer confirmation 後才能 passed。下游不得只依賴「程式可編譯」；必須依賴明列的 contract hash 或 disposition。

## 每波派工方式

1. **Wave 0（readiness 與基準）：** Primary agent 驗證九個 OpenSpec changes、dependency DAG、task coverage 與本協議；Architecture reviewer 唯讀複核。完成 readiness evidence 與規劃基準 commit 後，依使用者核准立即進入 Wave 1 apply。
2. **Wave 1：** 一個 Workspace/Build worker 依 bootstrap 的 L2 順序執行；一個 Mechanical worker 可平行準備 evidence fixtures，但不得先做依賴尚未完成的 build leaf。GPUI-CE 候選固定為 `https://github.com/damody/gpui-ce-explorer.git` commit `8945e2981b9fd00ca887e042d8adb9acc241b168` 的乾淨來源；不得依賴 `D:\SuperExplorer\vendor\gpui-ce` 目前未提交 patch。Primary agent 驗證 gate 並封存。
3. **Wave 2：** 一個 Core implementer 獨占 capability spike 與 `platform-win/common`；GO 前發布 platform-common API/ABI hash、Safe Mode/unsupported-session probe 與 FFI wrapper contract。Architecture reviewer 可在 evidence 完整後唯讀檢查。只有明確 GO 才進 Wave 3。
4. **Wave 3：** 一個 Core implementer 獨占 `shell-core/settings-store`；Mechanical worker 可建立 fake fixtures。Public contract hash 發布後，Wave 4 才能開始。
5. **Wave 4：** 三個 Worker 分別執行 Desktop、Taskbar、Bridge；每個只改專屬路徑，並由各 domain owner 對自己的 callbacks 套用 Wave 2 凍結的 no-unwind wrapper。Taskbar 只以 Wave 3 凍結的 bridge DTO/fake adapter 驗證 UI；真實 end-to-end 留到 Wave 5/6。需要 shared contract 變更時該分支暫停，由 Primary agent建立 corrective change；其他兩分支繼續不受影響的工作。
6. **Wave 5：** 一個 Core implementer 執行 lifecycle/guardian與 callback inventory audit；Desktop、Taskbar、Bridge owners只處理被退回的本域 callback/production 修正。Architecture reviewer 在 recovery evidence 完整後唯讀複核。Wave 5 只發布 reference-profile provisional dispositions。
7. **Wave 6：** 最多三個 verification workers 依 Display/UI、Accessibility/Localization、Reliability/Performance/Safety 分域執行；exact reference-profile lifecycle/installer 與實體 mixed-DPI 必須由 Primary agent 明確派工。Independent reviewer 最後複核，remediation 回原 owner。

每個代理一次只領取一個 L2 work package；只有前一 L2 的完成門檻與 evidence 通過，才領取同一路徑的下一個 L2。這避免把整個 change 當成單一長任務而失去可恢復交接點。

已封存 change 不直接改寫。Wave 4/5/6 若發現 contract 或 production 缺陷，由 Primary integrator 建立獨立 corrective change，引用原 archive revision與被取代 contract hash；修正通過後發布新 hash，所有 dependent evidence 標 stale 並重跑。Program roll-up 永遠指向最新有效 corrective lineage。

## 全域 Task Identity

tasks.md 內的數字只在各 change 內唯一。Evidence 的全域 `task_id` 必須使用 `<change-name>/<L3-id>`，例如 `build-superdesktop-gpui-taskbar/3.1.3`。各 tasks 內的 `evidence/artifacts/1.1/` 是相對於該 change 的路徑，實際位置為 `openspec/changes/<change-name>/evidence/artifacts/1.1/`；每個 change 另有自己的 append-only `evidence/index.jsonl`。任何只寫 `3.1.3` 的 record 都視為 ambiguous 並由 validator 拒絕。

## 代理交接與驗收

每次交接必須包含：完成的全域 task IDs、修改檔案、執行命令與 exit status、evidence paths/hashes、未解 findings、任何 stale/replacement lineage，以及下個 owner 可直接使用的 contract hash。只有 commentary 或「大致完成」不構成交接。

每次交接寫入 change-local `evidence/handoffs/<L2-id>.json`，包含 change/L2、producer、consumer、base revision、result revision、ownership diff、commands/exit status、gate dispositions、contract-hash algorithm與輸入 manifest、evidence hashes、stale/replacement lineage、reviewer及 accepted revision。各 child owner 只寫 change-local evidence、adjustments 與 handoff；Primary integrator 是 program artifacts、roll-up index、archive sync 的唯一 writer。

Primary agent 必須按以下順序接受交接：檢查 ownership diff → 執行 change-local tests → 驗證 evidence hashes/schema → 檢查 public contract drift → 更新 dependency graph → 才勾選 tasks。交接失敗時只退回有問題的 L2，不回復其他代理已完成的修改。
