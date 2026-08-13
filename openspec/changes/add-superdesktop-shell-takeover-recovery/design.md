## Context

此 change 等 desktop/taskbar/bridge 完成後才執行。凍結 ExplorerPatcher profile 是 takeover/recovery reference；M0 不修改登入 Shell registry。

## Goals / Non-Goals

**Goals:** preview composition、session single owner、六階段 transaction、health/failpoints、guardian anti-spoof、安全 Explorer restore、FFI fatal routing、10-run recovery。

**Non-Goals:** installer/autostart/registry Shell replacement、完整 tray/Start menu。

## Decisions

- `superdesktop-app` 是唯一 composition root；先在 zero-mutation preview 將 core/settings/desktop/taskbar/bridge 的真實 adapters 接通，再允許 takeover transaction 使用同一組 typed routes。
- Preview 永不改 Explorer/work area/registry。
- Owner lease 在任何 AppBar/Explorer mutation 前取得，以 PID/creation/session/file identity/nonce fencing。
- Guardian authority 來自 inherited process handle + one-time channel，不來自 journal。
- Explorer 使用 verified system absolute path、explicit application、interactive token、restricted handles/env。
- T0 為 process handle signaled；10 runs 每次 ≤10 秒恢復 input-ready Explorer/work area。

## Risks / Trade-offs

- **[ExplorerPatcher lifecycle 不穩定]** → capability probe、fail-closed、guardian、每階段 failpoint。
- **[雙 owner 競態]** → atomic session lease + non-owner rejection。

## Migration Plan

Composition preview → lease/guardian → takeover state machine → Explorer switch/restore → failpoints → 10-run reference evidence。

## Open Questions

無。
