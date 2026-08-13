## Context

這是 M0 dependency graph 的第一個 production change。儲存庫目前只有設計/OpenSpec 文件，尚無 Cargo workspace。本 change 不依賴外部硬體，也不得建立產品功能占位 UI。

## Goals / Non-Goals

**Goals:** 建立固定 toolchain/GPUI-CE、九 crate skeleton、Windows identity、架構 checker、證據 schema、離線可重現建置與來源/授權邊界。

**Non-Goals:** 不實作 reducer、桌面、工作列、Shell API、SuperExplorer 啟動或 takeover。

## Decisions

- `shell-core` 不依賴 GPUI/Win32；UI 不公開 Win32/COM；composition 只在 `superdesktop-app`。
- GPUI-CE 使用 `https://github.com/damody/gpui-ce-explorer.git` commit `8945e2981b9fd00ca887e042d8adb9acc241b168` 的乾淨來源；SuperExplorer vendor 目前的未提交 patch 不是依賴輸入。
- 所有 leaf mandatory；replacement 必須存在、無循環、同 coverage 且先通過。
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
