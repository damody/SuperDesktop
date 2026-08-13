## ADDED Requirements

### Requirement: Capability spike 必須消費已凍結的 Windows/FFI substrate
系統 SHALL 在任何 spike 前驗證 bootstrap archive handoff 的 workspace contract hash，且直接 Windows binding 版本/features、offline provenance、全域 unsafe deny 與 `platform-win` 唯一 audited unsafe exception均未漂移。每個 unsafe block MUST 具體記錄 SAFETY invariant；其他 crate MUST NOT 直接依賴 Windows binding或降低 unsafe lint。

#### Scenario: Bootstrap substrate contract 漂移
- **WHEN** Windows binding、features、lock/vendor provenance、unsafe lint或唯一 owner與 bootstrap handoff 不同
- **THEN** capability change MUST stop，且 Platform owner不得自行修改 root contract後繼續

### Requirement: Reference profile 必須凍結
系統 SHALL 記錄 OS build 26200.8875、ExplorerPatcher 26100.8457.70.3、設定摘要、reference image SHA-256、GPUI source revision 與 spike binary hash。

#### Scenario: Profile 相符
- **WHEN** spike 在目前凍結 profile 執行
- **THEN** raw evidence 包含全部 identity/hash 並可與後續 baseline 比對

#### Scenario: Profile 漂移
- **WHEN** 任一 profile identity 不符
- **THEN** 舊 go disposition 不得直接重用

### Requirement: GPUI HWND、AppBar 與 Shell Hook 必須可逆
系統 SHALL 驗證 native HWND/message bridge、AppBar reserve/update/remove/work-area restore 與 Shell Hook register/event/unregister。

#### Scenario: AppBar spike
- **WHEN** spike 建立並移除測試 AppBar
- **THEN** work area 在結束後與開始 snapshot 相同

#### Scenario: Hook 解除註冊
- **WHEN** Shell Hook 已解除後產生 helper window event
- **THEN** adapter 不再收到 callback 且資源 snapshot 無持續成長

### Requirement: DPI、Topology 與 Start host 必須可探測
系統 SHALL 驗證 per-monitor DPI geometry、monitor identity/topology event，以及目前 ExplorerPatcher Start host probe/invocation。

#### Scenario: Start host 可用
- **WHEN** probe 與受控 invocation 在 reference profile 執行
- **THEN** 系統取得可判定成功結果且 Explorer 保持可操作

#### Scenario: Start host 不可用
- **WHEN** probe 回報 unavailable
- **THEN** disposition 為 stop，不得繼續依賴 Start 的 Shell takeover 實作

### Requirement: Guardian Lease 與 FFI Boundary 必須安全
系統 SHALL 驗證 inherited process handle/one-time channel、crash signal、callback `catch_unwind`、handle ownership 與 at-most-once cleanup。

#### Scenario: Callback panic
- **WHEN** spike callback 注入 Rust panic
- **THEN** unwind 不穿越 Win32 ABI，轉為 typed fatal result 並完整釋放資源

#### Scenario: Safe Mode 或不支援 session
- **WHEN** capability probe 判定 Safe Mode、非互動或不支援 session
- **THEN** 在任何 AppBar/Explorer mutation 前 fail closed

### Requirement: Go disposition 必須要求全部 required subcheck 通過
系統 SHALL 只有在所有 required capability 與 soak/resource check 通過時產生 go。

#### Scenario: 單一 subcheck 失敗
- **WHEN** 任一 required subcheck 失敗、未執行或證據 stale
- **THEN** 整體 disposition 為 stop 並阻擋下游 production change
