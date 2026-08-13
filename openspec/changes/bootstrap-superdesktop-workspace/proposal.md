## Why

所有後續 SuperDesktop 功能都需要一個可重現、Windows-only、具架構邊界與證據治理的 Rust workspace；若先跳到 UI 或 Shell 實作，依賴、授權與 gate 會在後期才失敗。

## What Changes

- 建立九個設計 crate、Windows 產品 identity、固定 toolchain/GPUI-CE 與 lockfile。
- 建立 architecture checker，禁止 core/UI/platform 邊界倒置。
- 建立 mandatory task evidence schema、lineage 與 adjustment ledger。
- 建立 isolated/offline dependency source 與建置 gate。

## Capabilities

### New Capabilities

- `superdesktop-workspace-foundation`：Windows-only workspace、依賴/授權來源、產品 identity、架構與離線重現。
- `superdesktop-evidence-governance`：task evidence schema、mandatory/superseded 規則、adjustment lineage 與 validator。

### Modified Capabilities

無。

## Impact

新增 Cargo workspace、`.cargo`/toolchain 設定、crate skeleton、架構與證據腳本；不建立產品 UI、不接管 Shell、不修改 SuperExplorer/PExplorer。
