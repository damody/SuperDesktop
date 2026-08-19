# Taskbar Jump List Geometry Design

## Intent

Make the owned task Jump List source-anchored, content-sized and taskbar-aware. The current 360×480 DIP centered window is visibly wrong for the common two-command list and can overlap or float away from the source task.

## Design

Add a pure geometry helper receiving monitor, preview/shell mode, rows, physical pointer X, entry count and non-empty group count. Width remains 360 DIP and clamps to the monitor. Height is 8 DIP outer padding plus 32 DIP per entry, 2 DIP inter-item gaps and one DIP per group separator, capped at 480 DIP and available height. Horizontal center follows the pointer and clamps; missing pointer falls back to monitor center. Bottom follows the matching preview/shell taskbar top minus 8 DIP.

The composition derives entry/group counts before moving the model into the view and captures the physical cursor on right-click admission. Existing typed actions, keyboard/UIA Menu/MenuItem semantics and Explorer-free provider boundaries remain unchanged.

UTIT extends the production taskbar case to right-click a controlled task, measure actual source/popup/taskbar rectangles, verify containment, source anchoring, bounded content height and Menu/MenuItem semantics, and retain screenshots/hashes. Pure tests cover 96–216 DPI, both modes, 1–3 rows, edges, negative origins, empty/minimum/maximum lists and cursor fallback.

No Explorer, system Jump List, privilege, persistence or undocumented API is added. Rollback is a source revert; keep the OpenSpec change unarchived.
