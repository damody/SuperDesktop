## Context

依賴 platform spike 與 shell-core contract。本 change 擁有 `desktop-ui` 與 desktop-specific `platform-win` adapter；不修改 taskbar/bridge/lifecycle owned paths。

## Goals / Non-Goals

**Goals:** per-monitor GPUI desktop、wallpaper、User/Public Desktop Shell items、selection/focus/position、watcher recovery、Windows association。

**Non-Goals:** rename、context menu、file transfer drag/drop、Recycle Bin mutation、Shell takeover。

## Decisions

- Item identity 使用 owned Shell identity，不使用 display name。
- Pointer drag 在 M0 只重新定位圖示，不啟動資料傳輸。
- 一般檔案由 Windows association adapter；資料夾只發 typed bridge command。
- 無 initial-path 的固定應用程式入口稱「SuperExplorer」，不稱「本機」。
- 虛擬 monitor adapter 完成自動 topology gate；真實雙螢幕留給 final confirmation。

## Risks / Trade-offs

- **[Shell enumeration/thumbnail 卡住]** → apartment owner、bounded request/deadline/cache。
- **[Watcher overflow]** → full refresh + stable identity restore。

## Migration Plan

Monitor host → wallpaper → namespace/icon/watcher → interaction/persistence → association → a11y/headful tests。

## Open Questions

無。
