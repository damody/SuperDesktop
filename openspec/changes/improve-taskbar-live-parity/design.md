## Context

The approved design is `docs/superpowers/specs/2026-08-17-taskbar-live-parity-design.md`. The active host exposes `TaskbarGlomLevel=2` (never combine). The production taskbar instead defaults to grouping, uses row-major flex wrapping despite a column-major layout contract, and constructs status from a fixed `TestClock`.

## Goals / Non-Goals

**Goals:** host-aligned defaults, occupied configured rows, current local time, deterministic tests and headful evidence.

**Non-Goals:** icons, Task View registry mirroring, complete tray parity, or broad undocumented ExplorerPatcher configuration ingestion.

## Decisions

1. Default and missing `combine_groups` to false; preserve explicit true.
2. Use column-direction flex wrapping in the full-height task region. Fixed entry and tasks keep 40-logical-pixel height and fill rows before columns.
3. Add an owned `LocalDateTime` platform value from Win32 `GetLocalTime`; map it to the existing formatting type in app composition.
4. Refresh status inside the existing reconciliation loop and notify only on value change.
5. Blocking gate `G-TASKBAR-LIVE-PARITY` requires settings/layout/platform/update tests and 175% DPI headful confirmation.

## Risks / Trade-offs

- **[Column widths differ between fixed and task buttons]** → Preserve current widths in this change and test row occupancy; width unification is separate visual tuning.
- **[50 ms loop reads time frequently]** → `GetLocalTime` is local and cheap; GPUI updates occur only when formatted status changes.
- **[Never-combine increases buttons]** → Existing overflow/capacity behavior remains authoritative and explicit grouping stays supported.

## Migration Plan

Land defaults, layout direction, platform adapter, and refresh atomically; run tests/capture; revert sources to roll back. No persisted migration is required.

## Open Questions

None.
