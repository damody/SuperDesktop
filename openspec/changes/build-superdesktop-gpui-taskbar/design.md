## Context

UI reference 是目前 ExplorerPatcher 雙列工作列。此 change 依賴 spike/core，擁有 `taskbar-ui` 與 taskbar-specific window/AppBar/Start/status adapters。

## Goals / Non-Goals

**Goals:** per-monitor AppBar、1–3 rows、window tracking/group/order/click semantics、固定 SuperExplorer 入口、Start/status/time/a11y。

**Non-Goals:** 完整 tray protocol、custom Start menu/search、jump list、live thumbnail、takeover coordinator。

## Decisions

- Shell Hook 是增量來源，`EnumWindows` 是權威來源。
- WindowId 與 ApplicationId 分離；非 membership/pin 變更不得重排。
- 固定入口名稱「SuperExplorer」，發送無 initial-path bridge command。
- ExplorerPatcher profile/hash 是 visual baseline；變更後舊 baseline stale。
- Preview 不改 work area；production AppBar mutation 由 lifecycle change 最終協調。

## Risks / Trade-offs

- **[窗口分類差異]** → helper matrix + reconciliation。
- **[AppBar 多螢幕差異]** → spike go、virtual topology、自動 restore。

## Migration Plan

Window tracker → task model → AppBar/layout → fixed entry/Start/status → visual/a11y tests。

## Open Questions

無。
