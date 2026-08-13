## Why

桌面、工作列、SuperExplorer 與 lifecycle 需要共用且不依賴 GPUI/Win32 的權威狀態、非同步結果與設定合約，否則平行實作會產生互相衝突的狀態來源。

## What Changes

- 實作 stable identity、command/event/effect、immutable snapshot 與 reducer。
- 實作 generation/cancellation、bounded queue、coalescing、overflow reconciliation 與 exactly-once terminal。
- 實作版本化設定、execution-mode safety、原子寫入、migration 與 quarantine。

## Capabilities

### New Capabilities

- `shell-state-and-reconciliation`：核心 state machine、identity、非同步 generation、queue 與權威對帳。
- `shell-settings-store`：版本化設定、原子保存、局部 fallback、損毀隔離與執行模式安全。

### Modified Capabilities

無。

## Impact

依賴 workspace 與 capability go disposition；只修改 core/settings/test-support，不建立 platform handle 或產品 UI。
