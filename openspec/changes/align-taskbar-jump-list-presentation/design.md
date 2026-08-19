## Context

Source design: `docs/superpowers/specs/2026-08-19-taskbar-jump-list-presentation-design.md`.

## Decisions

- Recent/Frequent/Tasks/Local map to Recent/Frequent/Tasks/Actions headings.
- Headings are 24 DIP UIA Heading and not keyboard focus targets.
- MenuItems remain 32 DIP; 16 DIP glyphs derive from typed group/command/risk.
- Geometry adds 24 DIP per non-empty group and remains capped at 480 DIP.
- UTIT/source gates reject generic bullet presentation and require headings/MenuItems.
- Rollback is source revert; no migration. Presentation corrections reopen artifacts; delegation or behavior changes require approval.

## Open Questions

None.
