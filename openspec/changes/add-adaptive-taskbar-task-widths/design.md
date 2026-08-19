## Context

Source design: `docs/superpowers/specs/2026-08-19-adaptive-one-row-task-width-design.md`.

## Decisions

- Subtract all live left/right fixed regions from logical window width.
- Divide remaining width by columns `ceil(tasks/rows)` and clamp 44–160 DIP.
- Icon-only tasks stay 44 DIP; under-minimum capacity uses existing overflow fallback.
- All state layers consume the same width.
- UTIT asserts widths, ordering, non-overlap and right exclusion.

## Rollback

Source revert only; no migration. Geometry corrections reopen evidence; delegation or gate weakening requires approval.
