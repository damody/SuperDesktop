# Taskbar Jump List Presentation Design

## Intent

Replace the generic bullet list with Windows-style grouped Jump List presentation while preserving typed commands and the source-anchored geometry already implemented.

## Design

Each non-empty Recent, Frequent, Tasks and Local group renders a 24 DIP UIA Heading followed by 32 DIP MenuItems. The Local heading is presented as Actions. A fixed 16 DIP semantic glyph column replaces bullet characters: history, frequent, task, pin and close glyphs are selected from the typed group/command/risk without loading Explorer resources. Group transitions use spacing and heading text instead of applying a border to an arbitrary first item.

Geometry includes 24 DIP per visible group in the content-height calculation and retains the 480 DIP cap. Keyboard focus continues to index commands only, so headings are not focus targets. Pointer, Enter and Escape paths remain exactly-once.

UTIT requires Recent/Frequent/Tasks/Actions headings when their groups are populated, at least two MenuItems, no generic bullet glyph in the rendered source, 24/32 DIP source contracts, screenshot/hash evidence and no Explorer/system Jump List delegation. Rollback is a source revert; keep the OpenSpec change unarchived.
