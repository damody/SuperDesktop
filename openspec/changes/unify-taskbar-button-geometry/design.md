## Context

The taskbar renderer owns a separate fixed SuperExplorer entry followed by tracked window tasks. The previous adaptive-width change excluded the fixed entry by reserving 160 DIP before dividing available space among ordinary tasks. This prevents overlap but leaves unequal button and indicator geometry. The product must remain functional without Explorer and all measurements must use one logical/physical DPI conversion.

## Goals / Non-Goals

**Goals:**

- Give the fixed entry and ordinary labeled task buttons one adaptive 44-160 DIP width under the default labeled layout.
- Preserve an 8 DIP inset for the fixed entry's long running indicator.
- Make UTIT record and reject fixed/task width mismatch, unstable order, invalid bounds, and right-control overlap.
- Remove self-referential source-contract coverage.

**Non-Goals:**

- Changing fixed-entry activation, task grouping, pinning, Jump List behavior, system controls, Start, IME, or installer behavior.
- Calling, launching, or delegating product UI to Explorer.

## Decisions

The allocator counts the fixed entry as a task slot and removes its former 160 DIP reservation from the left fixed-control region. This keeps one source of truth for crowded geometry while leaving command routing separate. Converting the fixed entry into a normal task was rejected because it would incorrectly inherit window lifecycle and grouping semantics. Hiding its label was rejected because it causes visual jumps and loses Windows-style labeled-button parity.

The fixed entry consumes `adaptive_task_width` for the outer width and derives its indicator as `max(12, width - 16)` with an 8 DIP left inset. Ordinary task state layers continue to consume the same value.

The live capture stores the fixed rectangle independently, then stores ordinary task records in UI Automation order. It converts all widths using `GetDpiForWindow`, requires the fixed width to match every labeled task within one physical pixel in the forced labeled profile, and retains the existing right-control boundary assertion.

Source tests check named allocation variables and assert that obsolete `.w(px(160.))` fixed-entry geometry and the prior literal source assertion are absent. Assertions shall not succeed by matching their own required string.

## Risks / Trade-offs

- **Risk: Mixed icon-only layouts can use conservative space.** -> The change preserves the existing safe allocator and changes only fixed-entry participation; a later capability can model heterogeneous slot widths.
- **Risk: Fractional DPI rounding creates a one-pixel difference.** -> Live parity permits at most one physical pixel while logical clamp checks retain tolerance.
- **Risk: Narrow bars reach minimum width before every label is readable.** -> Width remains clamped at 44 DIP and labels use ellipsis; right-side system controls remain reserved.

## Migration Plan

No persisted data or public API changes. Deploy through the existing release build. Rollback is the two renderer/script edits; settings and evidence formats remain backward-readable because new report fields are additive.

## Open Questions

None. Scope and thresholds are fixed by the approved design.
