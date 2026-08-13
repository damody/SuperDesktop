## ADDED Requirements

### Requirement: Windows platform substrate 必須可離線編譯且限制 unsafe 邊界
Workspace SHALL 固定 Wave 2 所需的直接 Windows binding 版本與 features，納入 lockfile、vendor、license/provenance 與 isolated offline build contract。Workspace SHALL 全域拒絕 unsafe code；只有 `platform-win` MAY 使用 crate-local audited unsafe exception，且每個 unsafe block MUST 具體記錄 SAFETY invariant。Architecture checker SHALL 拒絕任何其他 crate 的 unsafe 例外或 Windows binding direct dependency。

#### Scenario: 非 platform crate 嘗試開啟 unsafe
- **WHEN** 任一非 `platform-win` crate 降低 workspace unsafe lint或直接依賴 Windows binding
- **THEN** architecture gate MUST 失敗

#### Scenario: platform-win 使用未稽核 unsafe block
- **WHEN** `platform-win` 出現沒有具體 SAFETY invariant 的 unsafe block
- **THEN** architecture gate MUST 失敗

### Requirement: Workspace 必須是 Windows-only 且依賴方向固定
系統 SHALL 建立設計中的九個 crate，並以機器 gate 禁止 `shell-core` 依賴 GPUI/Win32、UI 公開 Win32/COM、或非 app crate 擁有 composition root。

#### Scenario: 合規 workspace
- **WHEN** architecture checker 掃描 production workspace
- **THEN** crate graph 符合 allowlist，Windows target check 通過且非 Windows target 明確拒絕

#### Scenario: 邊界負面 fixture
- **WHEN** fixture 讓 UI 公開 HWND 或 core 依賴 GPUI
- **THEN** architecture checker 以非零結果拒絕

#### Scenario: 巢狀模組重新匯出 Windows 型別
- **WHEN** UI crate 在巢狀 Rust module 以 `pub use`、public trait 或跨行 signature 暴露 HWND/COM 型別
- **THEN** architecture checker 必須以非零結果拒絕，不能只掃描 `src` 根目錄或單行宣告

### Requirement: Toolchain 與 dependency source 必須固定且離線可重現
系統 SHALL 固定 Rust、GPUI-CE、Windows bindings 與 lockfile，並保存 source/hash manifest；isolated network-disabled environment MUST 通過 `cargo check --locked --offline`。Dev 與 release profile SHALL 明確使用 `panic = "unwind"`；因 Cargo 忽略 test profile 的 panic setting，test profile MUST 不設定 `abort` 或其他 panic 覆寫，並由 machine assertion 驗證其測試 harness unwind 語義。

#### Scenario: 離線建置
- **WHEN** 使用已驗證 source cache/vendor 與 isolated `CARGO_HOME` 停用網路建置
- **THEN** workspace check 成功且輸出對應固定 lock/source hashes

#### Scenario: Source 缺失或 hash 漂移
- **WHEN** 任一 dependency source 缺失或 hash 不符
- **THEN** gate 失敗，不能只用連網建置取代

#### Scenario: Panic policy 漂移
- **WHEN** dev/release 不再是 unwind，或 test profile 設定 `abort`／其他無效 panic 覆寫
- **THEN** machine assertion 必須失敗，且 gate 不得把 Cargo 忽略的 test profile 設定視為有效證據

### Requirement: Windows 產品 Identity 必須可驗證
系統 SHALL 為 SuperDesktop 與 guardian 建立獨立 manifest、VERSIONINFO、檔名與 icon resource。

#### Scenario: 檢查已建 binary
- **WHEN** product identity test 讀取 binary resource
- **THEN** company/product/original filename 與預期 SuperDesktop/guardian identity 相符

### Requirement: 來源與授權邊界必須阻止未核准衍生碼
系統 MUST NOT 複製或機械翻譯 PExplorer，也不得 path-link SuperExplorer 內部 crate。

#### Scenario: Source boundary audit
- **WHEN** audit 掃描 dependency graph 與新增來源
- **THEN** 不存在 PExplorer 未核准衍生碼或 SuperExplorer path dependency，且所有第三方來源有版本/hash/license 紀錄
