## Context

依賴 workspace bootstrap 與 platform spike go disposition。本 change 是 desktop/taskbar/bridge/lifecycle 的唯一 shared contract owner。

## Goals / Non-Goals

**Goals:** stable identity、immutable snapshot、typed command/event/effect、reducer、generation/cancellation、bounded queue/reconciliation、settings v1 與 evidence-ready telemetry。

**Non-Goals:** 不依賴 GPUI/Win32，不建立 UI 或 platform adapter。

## Decisions

- 所有 user-visible state 只由 reducer 改變。
- 非同步結果帶 RequestId/Generation；stale/late 結果只記診斷。
- Queue overflow 是顯式事件，要求 authoritative reconciliation。
- Settings atomic replace；execution preference 不能繞過明確 `--shell` opt-in。
- 下游 contract 變更先由本 change owner 修正並使相依 evidence stale。

## Risks / Trade-offs

- **[過度抽象]** → 只定義 M0 已有 scenario 所需的 DTO/effect。
- **[Queue capacity 不合理]** → 容量可設定但 overflow semantics 固定，不以放大容量取代 recovery。

## Migration Plan

Identity → snapshot/command/event/effect → reducer → async registry/queue → settings → contract/property tests → publish contract hash。

## Open Questions

無。
