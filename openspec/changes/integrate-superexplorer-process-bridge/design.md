## Context

SuperExplorer 現有 main 只接受 plugin CLI；初始資料夾由 child environment `EXPLORER_INITIAL_PATH` 提供。本 change 不修改 SuperExplorer dirty worktree。

## Goals / Non-Goals

**Goals:** resolver、folder/default launch、deadline/cancel/exactly-once、handle cleanup、repair prompt、redacted diagnostics。

**Non-Goals:** 不導覽既有程序、不承諾「本機」synthetic root、不 fallback Windows Explorer。

## Decisions

- Resolver：user absolute setting → development release artifact → adjacent executable。
- Directory 必須 existing absolute dir；default launch 不設定 env，UI 稱「SuperExplorer」。
- Admission deadline 5 秒；第一 terminal 權威，成功後取消不強制殺外部程序但關閉本端 handles。
- 所有 source/worktree hash 前後比對，證明未修改 SuperExplorer。

## Risks / Trade-offs

- **[現有合約不足]** → truthful default launch；既有程序/This PC 導覽另開 IPC change。
- **[Spawn race]** → correlation、deadline、late suppression、handle census。

## Migration Plan

Resolver → validation/env builder → launcher → terminal/cancel → prompt → fake/real process tests。

## Open Questions

無。
