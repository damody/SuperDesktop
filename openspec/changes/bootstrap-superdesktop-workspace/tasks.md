## 1. Workspace 與依賴邊界

### 1.1 建立 Windows-only Cargo Workspace

**目的：** 建立可編譯且依賴方向固定的 SuperDesktop workspace 骨架。
**輸入：** 核准設計、空白 repository、已安裝 Rust toolchain。
**產出：** Root Cargo manifests、crate skeletons、Windows target guard。
**依賴：** 無。
**Owner／Wave：** Workspace owner／Wave 1。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/1.1/`。
**完成門檻：** Windows workspace metadata 與 dependency graph 符合設計，非 Windows target 明確拒絕。

- [x] 1.1.1 建立 workspace root 與九個核准 crate 的最小 manifests。
- [x] 1.1.2 為非 Windows target 加入可判定的 compile-time refusal。
- [x] 1.1.3 建立 crate dependency-direction 檢查腳本與允許清單。
- [x] 1.1.4 執行 workspace metadata 與 dependency graph 測試並保存輸出。

### 1.2 固定 Toolchain 與依賴來源

**目的：** 讓相同 revision 能以固定來源與 lockfile 重建。
**輸入：** 1.1 workspace、Wave 0 核准候選 `https://github.com/damody/gpui-ce-explorer.git` commit `8945e2981b9fd00ca887e042d8adb9acc241b168`。
**產出：** Toolchain pin、Cargo.lock、source provenance manifest。
**依賴：** 1.1。
**Owner／Wave：** Build owner／Wave 1。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/1.2/`。
**完成門檻：** Online 與 isolated offline build 使用相同來源 hash 並成功。

- [x] 1.2.1 固定 Rust toolchain、target 與 workspace build profiles。
- [x] 1.2.2 加入 machine assertion：要求 dev/release 明確 `panic = "unwind"`，且 test profile 不設定 `abort` 或其他 Cargo 會忽略的 panic 覆寫，保留測試 harness unwind 語義。
- [x] 1.2.3 由乾淨遠端來源固定 GPUI-CE commit `8945e2981b9fd00ca887e042d8adb9acc241b168` 並產生 Cargo.lock，不使用 SuperExplorer vendor 未提交 patch。
- [x] 1.2.4 產生 dependency source URL、revision、license 與 hash manifest。
- [x] 1.2.5 在 isolated CARGO_HOME 準備已驗證的 vendored 或 mirrored sources。
- [x] 1.2.6 停用網路執行 `cargo check --locked --offline` 並保存完整輸出。

## 2. Identity、授權與證據治理

### 2.1 建立 Windows 產品 Identity

**目的：** 提供可由測試驗證的 binary、AppUserModelID 與資源 identity。
**輸入：** 1.1 workspace、產品命名決策。
**產出：** Windows resource metadata、identity constants、驗證測試。
**依賴：** 1.1。
**Owner／Wave：** Windows packaging owner／Wave 1。
**Gate／Evidence：** `G-ARCH`；`evidence/artifacts/2.1/`。
**完成門檻：** App、guardian 與測試 binary identity 唯一、穩定且可機器驗證。

- [x] 2.1.1 定義 SuperDesktop app、guardian 與 test-support 的 binary identity。
- [x] 2.1.2 定義 AppUserModelID、版本與 Windows resource metadata。
- [x] 2.1.3 建立 identity collision 與缺欄位的負面測試。
- [x] 2.1.4 執行 identity verifier 並保存 binary metadata evidence。

### 2.2 建立來源與授權邊界

**目的：** 允許閱讀 PExplorer 行為但阻止未核准衍生碼進入 production。
**輸入：** Dependency manifest、PExplorer 與 SuperExplorer repository 邊界。
**產出：** Source-boundary policy、license inventory、audit script。
**依賴：** 1.2。
**Owner／Wave：** Compliance owner／Wave 1。
**Gate／Evidence：** `G-ARCH`、`G-SAFETY`；`evidence/artifacts/2.2/`。
**完成門檻：** 所有 dependency 有授權，PExplorer/SuperExplorer 邊界可由 audit 判定。

- [x] 2.2.1 記錄 PExplorer 僅供行為與 API 研究的來源政策。
- [x] 2.2.2 建立 production source provenance 與 third-party license inventory。
- [x] 2.2.3 建立來源邊界掃描與未揭露 dependency 負面 fixture。
- [x] 2.2.4 執行授權及來源 audit 並保存 reviewer disposition。

### 2.3 建立 Evidence Schema 與 Validator

**目的：** 讓每個 mandatory leaf 都只能以可追溯證據結案。
**輸入：** Program evidence contract、task IDs、調整分級規則。
**產出：** JSON schema、append-only index、validator、negative fixtures。
**依賴：** 1.1。
**Owner／Wave：** Evidence owner／Wave 1。
**Gate／Evidence：** `G-TRACE`；`evidence/artifacts/2.3/`。
**完成門檻：** Validator 接受有效 passed record，拒絕 N/A、stale、blocked 與無效 replacement。

- [x] 2.3.1 定義 `<change-name>/<L3-id>` task_id、change-local index、subcheck、artifact、hash、capability/requirement/scenario ID、gate、reviewer 與 timestamp schema。
- [x] 2.3.2 定義版本化 task-to-capability/requirement/scenario/gate coverage manifest schema 與穩定 slug 規則。
- [x] 2.3.3 實作 append-only evidence index validator、coverage manifest lookup 與 artifact hash 驗證。
- [x] 2.3.4 加入 mandatory `not-applicable`、blocked、stale 與 missing artifact 負面 fixtures。
- [x] 2.3.5 加入 unknown/missing/drifted requirement/scenario/gate coverage 負面 fixtures。
- [x] 2.3.6 加入 dangling、cyclic、non-mandatory、coverage-drift 與 unpassed replacement fixtures。
- [x] 2.3.7 實作 A/B/C adjustment lineage 與受影響 evidence stale propagation。
- [x] 2.3.8 執行全部 validator fixtures 並保存逐 fixture 結果。

### 2.4 修正 Wave 1 Exit Contract 與 Validator 缺口

**目的：** 關閉獨立 Wave 1 exit review 發現的 contract、schema、adjustment lineage 與巢狀 UI API boundary P1。
**輸入：** 1.1–2.3 已完成 evidence、Wave 1 exit review、B-W1-EXIT-001。
**產出：** 可逐 input 驗證的 contract、schema-complete replacement records、record-identity adjustment lineage、遞迴 public API checker與負面 fixtures。
**依賴：** 2.3。
**Owner／Wave：** Workspace owner／Wave 1 corrective。
**Gate／Evidence：** `G-ARCH`、`G-SAFETY`、`G-TRACE`；`evidence/artifacts/2.4/`。
**完成門檻：** 四個 P1 都有 passed replacement evidence；舊漂移 evidence 保留為 stale；全部正負 gate、26 原 leaf 與本 L2 mandatory leaf 通過 validator，獨立 reviewer 無剩餘 P0/P1。

- [ ] 2.4.1 實作 contract manifest verifier，逐列 canonicalize repository-relative path、拒絕逃逸/缺失/漂移，並重發受影響 contract replacement records。
- [ ] 2.4.2 擴充 evidence/coverage JSON Schema 與 validator，強制 procedure、expected、actual 及真實 schema validation，補齊 append-only replacement records與缺欄/type/pattern fixtures。
- [ ] 2.4.3 將 adjustment stale/replacement lineage 改為 immutable record IDs，驗證雙向連結、同 coverage、passed、完整受影響集合與 C approval，加入 malformed/dangling fixture。
- [ ] 2.4.4 強化 UI public API boundary 為遞迴 module/re-export/trait/跨行 signature 檢查，加入巢狀 `pub use HWND` 負面 fixture並重跑正負 architecture gate。
- [ ] 2.4.5 執行完整 contract/evidence/architecture matrix、strict OpenSpec 與獨立 Wave 1 exit 複審，保存 corrective handoff 與無未解 P0/P1 disposition。
