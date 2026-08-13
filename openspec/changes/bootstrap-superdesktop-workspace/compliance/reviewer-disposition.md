# 來源與授權邊界稽核處置（L2 2.2）

- 稽核執行者：Workspace/Build owner
- 日期：2026-08-14（Asia/Taipei）
- 處置：**pre-review passed**；交由 Primary integrator 在 Wave 1 exit 進行獨立驗收。

已完成的機器稽核結果如下：

- `cargo metadata --locked --offline` 列出的 399 個套件全部具有 license 欄位、來源記錄，以及第三方 vendored `.cargo-checksum.json` SHA-256。
- production Cargo manifests 與 `crates/**/*.rs` 沒有 `D:\SuperExplorer`、`D:\SuperDesktop\PExplorer`、相對 SuperExplorer path、或 `vendor/gpui-ce` 本地依賴痕跡。
- `PExplorer`／`ReactOS` 衍生碼標記也未出現在 production Rust 原始碼。
- 兩個負面 fixture 分別被拒絕為 `SUPEREXPLORER_PATH_DEPENDENCY` 與 `PEXPLORER_DERIVED_SOURCE`。

此稽核可驗證依賴來源、鎖定版本、vendor checksum 與明確的禁止邊界；它不是對所有可能語意相似性進行的自動判定。因此 PExplorer 僅可作行為／API／Win32 訊息研究，不可作為產品原始碼輸入；最終整合者必須在 Wave 1 exit 檢查 diff 與此證據後再接受。
