## Context

此 change 依賴 `bootstrap-superdesktop-workspace`。開發與 UI reference profile 為 Windows 11 build 26200.8875、ExplorerPatcher 26100.8457.70.3；參考圖 hash 固定。Spike 不建立完整產品 surface，也不切換 Explorer。

## Goals / Non-Goals

**Goals:** 最小化驗證 GPUI HWND/message、AppBar、Shell Hook、Start host、DPI/topology、guardian lease、FFI no-unwind、Safe Mode fail-closed 與資源清理。

**Non-Goals:** 不實作 production desktop/taskbar/core/takeover。

## Decisions

- 每個 capability 是獨立 mandatory subcheck，不能由其他成功項取代。
- 本 change 只在 bootstrap archive 發布可逐 input 驗證的 Wave 1 contract hash 後開始；archive relocation 不改寫封存 manifest，而由固定 archive revision 的 relocation verifier 將舊 active-change 前綴唯一映射到封存根目錄並逐 input 驗 hash。該 contract 必須固定直接 Windows binding 版本/features、offline source provenance，並保留全域 unsafe deny 與 `platform-win` 唯一 crate-local audited unsafe exception。任何 bytes、root dependency/lint 或非 relocation path drift 先回 Primary，不由 Platform owner 越界修改。
- Reference profile 必須保存 OS/ExplorerPatcher/config/image/source與 1.1 read-only profile/admission probe binary hashes；1.2 才建立 native-window spike binary。
- Spike 只在受控測試 HWND/AppBar 上執行，不隱藏 Explorer。
- GPUI native-window spike 由 `desktop-ui` example composition 擁有，並以 dev-dependency 呼叫 `platform-win/common` 的 HWND/message bridge；`platform-win` 不得反向依賴 GPUI，該 dev-only architecture successor 不得進入 product public API。
- Headful `Application` factory 使用同一 GPUI-CE pinned repository/revision的 `gpui_windows` package且停用 default features，僅為 `desktop-ui` dev-dependency；由 `Application::with_platform` 注入 Windows backend，其 lock/vendor/license/provenance與離線建置必須在1.2 evidence中閉合。
- 任一 required capability stop disposition 會阻擋下游，走 B/C correction。

## Risks / Trade-offs

- **[ExplorerPatcher 更新造成漂移]** → profile mismatch 直接使 baseline stale。
- **[Spike 意外改變 work area]** → 前後 snapshot 與 finally-style restore，失敗由原 Explorer 保持可用。

## Migration Plan

依序驗證 HWND → AppBar → Hook → DPI/topology → Start → guardian lease → no-unwind/soak，產生單一 go/stop report。

## Open Questions

無；失敗項依 adjustment policy 處理。
