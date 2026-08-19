## 1. Authoritative local-time model

- [ ] 1.1 Add seconds to the owned Windows local-time snapshot and all fixtures.
- [ ] 1.2 Add pure Traditional Chinese and English long-time, weekday, and short-date formatting.
- [ ] 1.3 Cover midnight, noon, leap date, all weekdays, seconds, and locale order with unit tests.
- [ ] 1.4 Compose runtime clock locale from the configured/user locale instead of hard-coding Traditional Chinese.

## 2. Row-aware clock presentation

- [ ] 2.1 Add one shared 112-DIP clock-width constant and update the right-side task reservation.
- [ ] 2.2 Render time/weekday/date at three rows and time/date at one or two rows.
- [ ] 2.3 Give every visible line full width, non-wrapping content, and explicit centered text.
- [ ] 2.4 Build the UIA name from the exact visible fields while preserving pointer/Enter/Space calendar activation.
- [ ] 2.5 Add source/model tests for row visibility, width/reservation, centering, UIA order, and unchanged activation.

## 3. Integrated verification

- [ ] 3.1 Run focused status/view tests plus format and locked all-target compilation.
- [ ] 3.2 Run full locked workspace tests and warnings-as-errors Clippy.
- [ ] 3.3 Build release and capture the 168-DPI three-row clock in light, dark, and high contrast.
- [ ] 3.4 Verify one-second advancement, UIA content, containment, and calendar activation in the headful run.
- [ ] 3.5 Save redacted evidence, run strict OpenSpec validation, and commit without unrelated worktree files.
