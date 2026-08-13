## Context

這是 M0 dependency graph 的第一個 production change。儲存庫目前只有設計/OpenSpec 文件，尚無 Cargo workspace。本 change 不依賴外部硬體，也不得建立產品功能占位 UI。

## Goals / Non-Goals

**Goals:** 建立固定 toolchain/GPUI-CE、九 crate skeleton、Windows identity、架構 checker、證據 schema、離線可重現建置與來源/授權邊界。

**Non-Goals:** 不實作 reducer、桌面、工作列、Shell API、SuperExplorer 啟動或 takeover。

## Decisions

- `shell-core` 不依賴 GPUI/Win32；UI 不公開 Win32/COM；composition 只在 `superdesktop-app`。
- GPUI-CE 使用 `https://github.com/damody/gpui-ce-explorer.git` commit `8945e2981b9fd00ca887e042d8adb9acc241b168` 的乾淨來源；SuperExplorer vendor 目前的未提交 patch 不是依賴輸入。
- 所有 leaf mandatory；replacement 必須存在、無循環、同 coverage 且先通過。
- Contract manifest 不只保存自身 hash；validator 必須逐列解析 repository-relative canonical path、拒絕逃逸、重算每個 input hash，並讓後續變更以 stale→passed record replacement 發布新 contract。
- Evidence record 必須包含 procedure、expected 與 actual，並由 JSON Schema 與語意 validator 同時驗證；adjustment lineage 只能引用 immutable `task_id#subcheck` record identity，逐一驗證 backlink、coverage 與 passed replacement。C 級 adjustment 另須有效使用者核准 record。
- Corrective contract 必須重新涵蓋被取代 contract 的完整 input set，不能只雜湊 corrective scripts；每份 replacement manifest 必須逐 input 通過同一 production verifier，且 replacement record 直接引用該 manifest。
- `evidence/schema.json` 與 `coverage-schema.json` 必須交由真正支援其宣告 draft 的 JSON Schema engine 驗證；fixture 只能 mutation/copy 真實資料後走同一 production validation path，不得依 fixture 名稱或 `fault.kind` 直接回傳預期錯誤。
- Adjustment ledger 採 append-only supersession graph：successor 必須指向存在且未形成循環的 predecessor，完整涵蓋 predecessor 的 effective stale set，並以 immutable stale/replacement record IDs 一一閉合；舊式 artifact-path lineage 必須由明確 successor supersede 後才失效。
- Wave 1 必須把 Wave 2 所需的 Windows binding、features 與 offline provenance 凍結為 workspace contract；全域仍 `unsafe_code = "deny"`，只有 `platform-win` 可使用 crate-local audited unsafe exception，且每個 unsafe block 必須有可稽核 SAFETY invariant 並由 architecture checker 拒絕其他 crate 的 unsafe 例外。
- UI 公開 API boundary 掃描必須遞迴涵蓋所有 Rust module，包含 public re-export、trait 與跨行 signature；負面 fixture 必須證明巢狀模組與 `pub use` 無法繞過。
- 依賴使用固定 revision/lockfile，並在 isolated `CARGO_HOME`、network-disabled 條件執行 `--locked --offline`。
- Cargo 1.97 會忽略 `[profile.test] panic`，因此 dev/release 明確固定 `panic = "unwind"`；test profile 不寫入無效鍵，而由 machine assertion 證明未設定 `abort` 或其他 panic 覆寫，保留 Cargo 測試 harness 的 unwind 語義。
- PExplorer 只作研究；SuperExplorer 只作外部程序，不引入來源 dependency。
- B 級修正同步更新 design/spec/tasks 並使相依證據 stale；框架、gate 或 scope 改變屬 C 級。

## Risks / Trade-offs

- **[GPUI source 無法離線取得]** → 在本 change 即建立 source/hash manifest；失敗阻擋下游。
- **[Skeleton 被誤當功能完成]** → 不提供產品 UI，capability 只承諾基礎與 gate。

## Migration Plan

建立 workspace → evidence validator → architecture checker → offline build → 獨立 review。Rollback 只移除尚未被下游使用的新增 skeleton；下游開始後需走 B 級修正。

## Open Questions

無；精確 GPUI capability 由下一個 spike change 驗證。
