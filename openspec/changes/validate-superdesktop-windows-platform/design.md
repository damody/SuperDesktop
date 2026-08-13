## Context

此 change 依賴 `bootstrap-superdesktop-workspace`。開發與 UI reference profile 為 Windows 11 build 26200.8875、ExplorerPatcher 26100.8457.70.3；參考圖 hash 固定。Spike 不建立完整產品 surface，也不切換 Explorer。

## Goals / Non-Goals

**Goals:** 最小化驗證 GPUI HWND/message、AppBar、Shell Hook、Start host、DPI/topology、guardian lease、FFI no-unwind、Safe Mode fail-closed 與資源清理。

**Non-Goals:** 不實作 production desktop/taskbar/core/takeover。

## Decisions

- 每個 capability 是獨立 mandatory subcheck，不能由其他成功項取代。
- 本 change 只在 bootstrap archive 發布可逐 input 驗證的 Wave 1 contract hash 後開始；該 contract 必須固定直接 Windows binding 版本/features、offline source provenance，並保留全域 unsafe deny 與 `platform-win` 唯一 crate-local audited unsafe exception。任何 root dependency/lint drift 先回 Primary，不由 Platform owner 越界修改。
- Reference profile 必須保存 OS/ExplorerPatcher/config/image/source/binary hashes。
- Spike 只在受控測試 HWND/AppBar 上執行，不隱藏 Explorer。
- 任一 required capability stop disposition 會阻擋下游，走 B/C correction。

## Risks / Trade-offs

- **[ExplorerPatcher 更新造成漂移]** → profile mismatch 直接使 baseline stale。
- **[Spike 意外改變 work area]** → 前後 snapshot 與 finally-style restore，失敗由原 Explorer 保持可用。

## Migration Plan

依序驗證 HWND → AppBar → Hook → DPI/topology → Start → guardian lease → no-unwind/soak，產生單一 go/stop report。

## Open Questions

無；失敗項依 adjustment policy 處理。
