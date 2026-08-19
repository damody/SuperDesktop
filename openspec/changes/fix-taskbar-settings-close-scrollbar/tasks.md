## 1. Settings chrome implementation

- [x] 1.1 Add deterministic scrollbar geometry helpers and boundary tests for overflow, fit, progress, minimum thumb, and collapsed-content clamping.
- [x] 1.2 Add view-owned `ScrollHandle` and drag state, track the content viewport, and map thumb drag to bounded content offsets.
- [x] 1.3 Add the fixed localized close button with shared dismissal, keyboard/UIA semantics, and theme interaction states.
- [x] 1.4 Render the fixed accessible vertical scrollbar, reserve content space, and keep wheel/drag/section changes synchronized.

## 2. Regression and headful evidence

- [x] 2.1 Extend focused source/model tests for fixed chrome, accessibility, close dismissal, scrollbar tracking, and unchanged settings persistence.
- [x] 2.2 Extend the settings headful validation to capture visible close/scrollbar chrome, invoke close through UIA, drag the thumb, and record changed scroll position/content.
- [x] 2.3 Run focused tests, locked compilation, formatting, and warnings-as-errors clippy; save hashed quality evidence.
- [x] 2.4 Run the headful close/scrollbar scenario, visually review captures, and save hashed reports/screenshots.

## 3. Final review

- [x] 3.1 Run strict OpenSpec validation and verify every scenario maps to current passing evidence.
- [x] 3.2 Confirm all tasks are complete with no failed, blocked, stale, P0, or P1 item.
