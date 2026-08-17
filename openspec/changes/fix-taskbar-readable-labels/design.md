## Context

The screenshot at the Windows 11 build 26200.9168 reference profile shows the SuperDesktop taskbar rendering isolated first characters across its upper row. `AccessibleTask` contains a name but no renderable icon, while `TaskbarView` treats `show_labels=false` as permission to use the first character as an icon substitute. The settings default and missing-field decoder both select that broken mode.

The approved source design is `docs/superpowers/specs/2026-08-17-taskbar-readable-labels-design.md`.

## Goals / Non-Goals

**Goals:** readable full-label fallback without real icons; truthful settings defaults; bounded single-line ellipsis; English/CJK/group regression coverage.

**Non-Goals:** icon extraction, new image caching, grouping/order changes, AppBar geometry changes, or Shell lifecycle changes.

## Decisions

1. Add a pure label policy that receives name, group size, label preference, and real-icon availability. It returns the full/grouped label whenever no icon exists. This fixes existing explicit `false` settings instead of only fixing new users.
2. Change both the struct default and missing JSON field fallback to `true`. Explicit persisted `false` remains round-trippable.
3. Place the label in a `flex_1 + min_w_0 + overflow_hidden + whitespace_nowrap + text_ellipsis` child. Badge/progress behavior remains separate and the parent task button keeps its identity/actions.
4. Blocking gate `G-TASKBAR-LABELS` requires policy/settings tests, source/render contract validation, and a headful capture with no isolated-character fallback.

## Risks / Trade-offs

- **[Stored false preference is not visually honored without icons]** → Truthful readable text takes priority; future real-icon support can honor it without a schema change.
- **[Long labels consume task width]** → Ellipsis is applied inside a shrinkable child and group/badge siblings retain space.
- **[CJK truncation splits poorly]** → Use GPUI's text layout/ellipsis instead of byte/character slicing.

## Migration Plan

Land settings and view changes together, run focused/workspace tests, then capture the reference taskbar. Rollback is a source revert; no persisted schema or system state migration is required.

## Open Questions

None.
