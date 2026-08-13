## ADDED Requirements

### Requirement: Capability spike 必須消費已凍結的 Windows/FFI substrate
系統 SHALL 在任何 spike 前驗證 bootstrap archive handoff 的 workspace contract hash，且直接 Windows binding 版本/features、offline provenance、全域 unsafe deny 與 `platform-win` 唯一 audited unsafe exception均未漂移。每個 unsafe block MUST 具體記錄 SAFETY invariant；其他 crate MUST NOT 直接依賴 Windows binding或降低 unsafe lint。

封存造成的唯一路徑變更 SHALL 由綁定 immutable archive revision 的 relocation verifier 處理：舊 `openspec/changes/bootstrap-superdesktop-workspace/` 前綴只能映射到該封存根目錄，所有宣告 SHA-256 仍須逐 input 相符；不得修改封存 manifest 來製造通過結果。

#### Scenario: Bootstrap substrate contract 漂移
- **WHEN** Windows binding、features、lock/vendor provenance、unsafe lint或唯一 owner與 bootstrap handoff 不同
- **THEN** capability change MUST stop，且 Platform owner不得自行修改 root contract後繼續

#### Scenario: Bootstrap archive relocation
- **WHEN** manifest input 只因已核准封存而從 active change 前綴移到固定 archive revision
- **THEN** verifier MUST 套用唯一前綴映射、驗證 archive tree與每個 input hash，且任何其他 path或bytes drift MUST stop

### Requirement: Reference profile 必須凍結
系統 SHALL 記錄 OS build 26200.8875、ExplorerPatcher 26100.8457.70.3、設定摘要、reference image SHA-256、GPUI source revision 與 1.1 read-only profile/admission probe binary hash。該 binary MUST NOT 建立 HWND、AppBar、Shell Hook或修改 Explorer；native-window spike binary 屬後續 1.2。

#### Scenario: Profile 相符
- **WHEN** spike 在目前凍結 profile 執行
- **THEN** raw evidence 包含全部 identity/hash 並可與後續 baseline 比對

#### Scenario: Profile 漂移
- **WHEN** 任一 profile identity 不符
- **THEN** 舊 go disposition 不得直接重用

### Requirement: GPUI HWND、AppBar 與 Shell Hook 必須可逆
系統 SHALL 驗證 native HWND/message bridge、AppBar reserve/update/remove/work-area restore 與 Shell Hook register/event/unregister。

#### Scenario: Native GPUI HWND message bridge
- **WHEN** 1.2 在 `App::open_window` callback 從 live `gpui::Window` 借用 Win32 HWND 並安裝 SuperDesktop-owned subclass
- **THEN** trace MUST 證明該 HWND 的 PID、thread、session、GPUI WindowId 與 generation，DPI/display/activation raw message 轉成 owned event，且不得建立第二個替代 HWND或由 bridge 銷毀 GPUI-owned HWND

#### Scenario: Native GPUI HWND terminal
- **WHEN** GPUI window 關閉或 subclass teardown
- **THEN** attach 與 closing 先發生，`WM_NCDESTROY` 與 GPUI `on_window_closed` 作為順序不保證的兩個獨立 terminal signal，finalized 必須晚於兩者且 raw callback reference 已釋放；late callback 被 generation fence 拒絕，callback/state outstanding 為零，且 handle/USER/GDI 資源在明列 deadline 與 threshold 內回到 baseline

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

1.2 MUST 先凍結 SuperDesktop-owned subclass 的 no-unwind wrapper，但不得據此宣稱 pinned `gpui_windows` 主 WndProc 已安全。3.2 MUST 對 pinned backend 的真實 public GPUI callback 路徑注入 panic，只有在取得 typed fatal、backend HWND terminal、GPUI window-closed terminal 與資源清理證據後，才可解除 preview-only 限制或供 production/Shell 使用。

#### Scenario: Callback panic
- **WHEN** spike callback 注入 Rust panic
- **THEN** unwind 不穿越任何 SuperDesktop-owned 或 pinned GPUI backend Win32 ABI，轉為 typed fatal result，停止 late user callback，並完整釋放 HWND、userdata、callback state 與資源

#### Scenario: Safe Mode 或不支援 session
- **WHEN** capability probe 判定 Safe Mode、非互動或不支援 session
- **THEN** 在任何 AppBar/Explorer mutation 前 fail closed

### Requirement: Go disposition 必須要求全部 required subcheck 通過
系統 SHALL 只有在所有 required capability 與 soak/resource check 通過時產生 go。

#### Scenario: 單一 subcheck 失敗
- **WHEN** 任一 required subcheck 失敗、未執行或證據 stale
- **THEN** 整體 disposition 為 stop 並阻擋下游 production change
