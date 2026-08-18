## Context

The existing owned page was introduced before the final high-DPI window contract. `surface_runtime` converts logical dimensions back to physical values before constructing `WindowOptions`, while `TaskbarSettingsView` renders a fixed `900×760` root and `836` content width. At 175% this produces double-scaled outer bounds but an undersized fixed page. The setting model itself is authoritative and already tested, so this change is intentionally render/geometry only.

## Goals / Non-Goals

**Goals:** Single logical DPI conversion; full-window responsive root; centered Windows 11 content column; accurate cards, rows, switches, focus and colors; complete vertical reachability; retained localization/UIA/atomic behavior; no Shell delegation.

**Non-Goals:** Add settings, enable unavailable capabilities, invoke inbox Settings, change persistence, implement the complete Settings navigation shell, or claim Windows 10 compatibility.

## Decisions

1. `taskbar_settings_placement` returns logical `f32` origin/size. `WindowOptions` consumes those values directly. Physical geometry is derived only in tests.
2. Add pure `TaskbarSettingsLayout` metrics derived from logical window width/height: 16 or 32 outer padding, content width capped at 1000, and centered content left offset.
3. The GPUI root uses `w_full().h_full()` and owns the only vertical scroll. The content column uses `w_full().max_w(...)`, horizontal auto margins, and bottom padding so the last card clears the viewport edge.
4. Retain existing Windows tokens but centralize them into a testable `TaskbarSettingsTokens`. High contrast uses explicit borders and focus rather than opacity alone.
5. Card headers use 64px, rows at least 56px, eight-pixel radii, one-pixel separators, 44×24 toggles, and 18px thumbs. Focused rows gain a visible two-pixel accent outline without changing layout.
6. Existing model activation, save and dismiss callbacks remain untouched. No layout branch gains filesystem, process, registry, Explorer, or Settings authority.
7. A-level refinements can split tests or tune non-contract code order. Geometry/token/authority/gate changes are B/C-level and require artifact correction or user approval as applicable; gates cannot be weakened.

## Risks / Trade-offs

- [GPUI max-width support differs from CSS] → Use explicit pure metrics and a full-width centered wrapper verified headfully.
- [Scroll focus does not reveal offscreen rows] → Keep one scroll owner and add keyboard/UIA bottom-control reachability evidence.
- [Small windows clip descriptions] → Use minimum heights, wrapping text, bounded padding, and test the 640×480 logical floor.
- [DPI regression returns] → Assert WindowOptions uses logical sizes and capture 175% outer/client geometry.
- [Visual refactor mutates behavior] → Re-run model/save/auto-hide UIA tests and reject any new delegated route.

## Migration Plan

No data migration. Rollback restores the previous render and window geometry without touching settings. The change is admitted only after visual/UIA evidence and both installers pass.

## Open Questions

None.
