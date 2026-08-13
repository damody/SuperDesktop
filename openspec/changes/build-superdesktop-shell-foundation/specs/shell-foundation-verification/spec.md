## ADDED Requirements

### Requirement: 架構與依賴 gate 必須阻斷不合規建置
系統 SHALL 驗證 Windows-only target、crate 依賴方向、固定 Rust/GPUI-CE revision、lockfile、產品 identity、禁止 UI 依賴 Win32 handle，以及 PExplorer/SuperExplorer 授權與來源邊界。

#### Scenario: UI crate 直接引入 Win32 handle
- **WHEN** architecture check 發現 `desktop-ui` 或 `taskbar-ui` 公開或依賴 Win32 handle/COM interface
- **THEN** `G-ARCH` 失敗，相關實作與發行不得標為完成

#### Scenario: PExplorer 程式碼被機械式移植
- **WHEN** license/source review 發現未經核准的 PExplorer 衍生程式碼
- **THEN** `G-ARCH` 失敗，該程式碼必須移除或先取得獨立授權決策與歸屬核准

### Requirement: Production 實作前必須通過 Windows/GPUI Capability Spike
系統 SHALL 在相依 production work package 開始前，以固定候選 GPUI-CE revision 驗證最小 GPUI native HWND/message bridge、AppBar reserve/restore、Shell Hook unregister、per-monitor DPI/topology、Windows 10 Start host probe/invocation 及 guardian inherited process-handle lease，並保存 source revision、binary hash、OS build、raw result 與 resource snapshot。

#### Scenario: 所有必要 spike subcheck 通過
- **WHEN** 每個必要 capability 在 Windows 10 reference machine 達成規範終態且資源 snapshot 無持續增長
- **THEN** 相依 production work package 可開始，且 evidence index 記錄 go disposition

#### Scenario: 任一必要 spike subcheck 失敗
- **WHEN** 任一必要 capability 無法達成或資源無法完整釋放
- **THEN** 所有相依 production work package 保持 blocked，並依 B 級修正或 C 級使用者核准流程處置，不得直接繞過 spike

### Requirement: 品質 gate 必須可重現
系統 SHALL 通過 `cargo fmt --check`、workspace check、clippy warnings-as-errors、workspace tests、架構檢查及依賴/授權稽核，並保存命令、exit status、hash 與時間戳。

#### Scenario: 任一品質命令失敗
- **WHEN** 任一必要品質命令 exit status 非零
- **THEN** 對應 gate 保持失敗，且不得以其他成功命令取代該失敗結果

### Requirement: Windows 參考與相容平台必須分開驗證
系統 SHALL 以 Windows 10 22H2 x64 驗證 Shell 行為與視覺參考，並以 Windows 11 驗證可啟動、可操作與可復原相容性，不得以 Windows 11 結果取代 Windows 10 reference gate。

#### Scenario: 只有 Windows 11 結果
- **WHEN** Windows 11 相容測試通過但 Windows 10 reference evidence 缺失
- **THEN** reference capability 保持未完成，change 不得宣稱 M0 發行 gate 通過

#### Scenario: Windows 11 相容性完整驗證
- **WHEN** 執行 Windows 11 compatibility gate
- **THEN** 系統必須完成啟動、桌面/工作列互動、正常退出復原及 forced-crash guardian 復原，並保存各自可判定結果

### Requirement: DPI 與多螢幕矩陣必須完整
系統 SHALL 驗證 100%、125%、150%、175%、200% DPI，以及至少一個混合 DPI 雙螢幕 topology 的桌面、工作列、hit target、文字截斷、hot-plug 與 work area。

#### Scenario: 任一必要 DPI 未執行
- **WHEN** evidence index 缺少任一必要 DPI 或混合 DPI 雙螢幕結果
- **THEN** `G-DPI-MONITOR` 保持未完成，而不得以 not-applicable 結案

### Requirement: 協助工具與在地化 gate 必須驗證真實互動
系統 SHALL 驗證 keyboard-only 操作、穩定 accessibility identity/role/name/state/action、可見焦點、高對比、繁體中文、英文、簡體中文字形/截斷/fallback、RTL/bidi 版面，以及 IME 不破壞工作列與桌面操作。M0 不要求完整簡體中文翻譯，但不得因簡中或 bidi 內容造成版面或操作失效。

#### Scenario: 可操作控制缺少 accessible name
- **WHEN** UI Automation 或 AccessKit 檢查發現可操作控制沒有穩定 name 或 action
- **THEN** `G-A11Y-I18N` 失敗，相關 surface 不得標為完成

#### Scenario: 繁體中文截斷破壞操作
- **WHEN** 繁體中文在必要 DPI 下造成按鈕重疊或無法操作
- **THEN** visual/localization gate 失敗，且必須保留截圖與 geometry evidence

#### Scenario: 簡中或 bidi 內容破壞版面
- **WHEN** 簡體中文字形/fallback 或 RTL/bidi 測試內容造成重疊、錯序、裁切或不可操作控制
- **THEN** `G-A11Y-I18N` 失敗，並保存 locale、字型、DPI、截圖與 geometry evidence

### Requirement: 效能預算必須以原始樣本驗證
系統 SHALL 在 reference machine 驗證冷啟動不超過 2 秒、穩定後 idle CPU median 低於 0.5%、Shell event-to-visible p95 低於 100 ms、M0 working set 低於 150 MiB，並保存原始樣本與環境 metadata。

#### Scenario: 任一效能 threshold 未達成
- **WHEN** 任一測量超過或等於其不允許的界線
- **THEN** `G-PERF` 失敗，且不得只用摘要或排除失敗樣本降低結果

#### Scenario: 缺少原始樣本
- **WHEN** 只有摘要數字而沒有原始樣本、版本與機器 metadata
- **THEN** 對應效能 leaf 保持未完成

### Requirement: 每個任務結果必須可追溯
系統 SHALL 讓每個完成的 atomic task 對應唯一 evidence `task_id`，或 immutable shared record 加唯一 `subcheck`，並記錄 artifact/command/manual procedure、expected、actual、exit status 或 reviewer、hash、gate、adjustment ID 與 timestamp。

#### Scenario: 任務勾選但缺少證據
- **WHEN** tasks.md leaf 被標為完成但 evidence index 找不到有效 task_id/subcheck
- **THEN** `G-TRACE` 失敗，該 leaf 必須重新開啟

#### Scenario: 證據因 B 級修正過期
- **WHEN** B 級設計/規格修正影響已完成 leaf
- **THEN** 系統保留舊 evidence lineage、將相依證據標為 stale，並在重跑前重新開啟 task

### Requirement: M0 必做任務不得以不適用結案
系統 MUST 將本 change 的所有現有 task leaf 視為 mandatory，且 MUST NOT 以 `not-applicable` 結案。日後只有在 task 建立時已明確標記為 conditional、定義客觀 eligibility、replacement coverage 與 gate disposition 的 leaf，才可使用具證據的 `not-applicable`。

#### Scenario: Mandatory leaf 嘗試使用 not-applicable
- **WHEN** evidence index 對現有 M0 leaf 提交 `not-applicable`
- **THEN** evidence validator 拒絕該紀錄，task 保持未完成；若要移除承諾必須走 C 級使用者核准

#### Scenario: Superseded replacement 無效
- **WHEN** mandatory leaf 的 replacement 不存在、形成循環、不是 mandatory、未 trace 至相同 requirement/scenario/gate，或尚未具有有效 passed evidence
- **THEN** validator 拒絕 superseded completion，原 leaf 保持未完成

### Requirement: 固定依賴必須可在隔離環境重現建置
系統 SHALL 以 isolated `CARGO_HOME` 驗證所有 dependency source 與 hash，並在停用網路後使用已驗證的 vendored/mirrored source 或完整預取 cache 執行 `cargo check --locked --offline`。

#### Scenario: Offline source 不完整
- **WHEN** network-disabled isolated build 缺少任一固定 dependency source 或 hash 不符
- **THEN** `G-ARCH` 失敗，且不得只以可連網 `--locked` 建置取代

### Requirement: Safe Mode 與不支援工作階段必須拒絕接管
系統 SHALL 在任何 AppBar 或 Explorer mutation 前探測 Windows Safe Mode、非互動 session 及不支援的 session 類型，並在命中時 fail closed。

#### Scenario: Windows Safe Mode
- **WHEN** 系統探測目前在 Windows Safe Mode
- **THEN** Shell 模式拒絕接管，Explorer 與 work area 保持不變，並保存 capability/evidence disposition

### Requirement: Blocking gate 不得被靜默降低
系統 MUST NOT 在未取得使用者核准時降低或移除 blocking gate、threshold、必要平台、必要證據或安全邊界。

#### Scenario: 實作發現 gate 難以通過
- **WHEN** 實作者認為既定 gate、threshold 或必要證據不合理
- **THEN** 受影響工作停止並依 B 或 C 級流程更新工件；在核准前原 gate 仍具約束力
