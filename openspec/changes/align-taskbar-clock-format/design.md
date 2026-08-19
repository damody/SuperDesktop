## Context

`platform-win::taskbar_status` publishes local year/month/day/hour/minute. `taskbar-ui::StatusRegion` formats one `HH:mm` string and one zero-padded date, while `TaskbarView` always renders those two intrinsic-width children inside an 82-DIP clock button. The taskbar already supports one to three rows and rerenders from a 50 ms status reconciliation loop.

## Goals / Non-Goals

**Goals:**

- Match the supplied Traditional Chinese Windows three-row clock content and centering.
- Preserve bounded layouts at one and two rows.
- Refresh visible seconds from the authoritative local system time.
- Preserve the existing owned Button, calendar callback, keyboard/UIA, themes, and Explorer-free behavior.

**Non-Goals:**

- User-configurable formats, seconds toggles, time zones, alternate calendars, or clock mutation.

## Decisions

### Pure locale formatting in the UI model

Add `second` to `LocalDateTime` and `TestClock`. `StatusRegion` derives `time`, `weekday`, and `date` using pure deterministic functions. Traditional Chinese uses `上午/下午 hh:mm:ss`, `星期X`, and `yyyy/M/d`; English uses `hh:mm:ss AM/PM`, full weekday, and `M/d/yyyy`. The existing status polling naturally advances seconds.

Alternative: call Windows formatting APIs for each render. Rejected because the current locale abstraction is intentionally deterministic and pure tests need stable output.

### Row-aware presentation with one shared width

Add a 112-DIP clock-width constant and derive the right-side reservation from the same value. All clock lines use `w_full`, `text_center`, and `whitespace_nowrap`. Three rows render time/weekday/date; one or two render time/date. The UIA label is constructed from the exact visible sequence.

Alternative: intrinsic child widths with padding. Rejected because unequal strings can appear offset and the reservation cannot prove containment.

## Risks / Trade-offs

- **[Long translated weekday or AM/PM string]** → Fixed 112-DIP width, 11–12 px typography, no wrapping, and locale fixtures gate clipping.
- **[Second-level rerenders increase work]** → Existing polling already runs faster than once per second; equality checks limit notifications to actual status changes.
- **[Gregorian weekday edge error]** → Pure tests cover all seven days, leap day, midnight, and noon.

## Migration Plan

Land the additive in-process status field, update all constructors/fixtures, then switch the clock rendering and reservation together. Rollback restores the prior two strings; no persisted data or protocol migration exists.

## Open Questions

None.
