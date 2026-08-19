# Align Taskbar Clock Format Design

## Goal

Align SuperDesktop's owned taskbar clock with the supplied Windows reference. A three-row taskbar shall display a localized long time with seconds, localized weekday, and short date on three independently centered lines. One- and two-row taskbars shall retain a bounded two-line presentation without clipping.

## Chosen approach

Make the clock presentation aware of configured taskbar rows and give every visible clock line the same fixed control width plus explicit full-width text centering. Extend the owned local-time value with seconds and derive weekday deterministically from the calendar date. This fixes both the missing Windows-style content and the unreliable intrinsic-width alignment.

Padding-only alignment was rejected because it would retain the wrong two-line minute-only format. An always-three-line layout was rejected because it would overflow the supported one-row taskbar. Delegating to the Explorer clock was rejected because SuperDesktop must remain usable without Explorer.

## Formatting contract

Traditional Chinese long time uses `上午/下午 hh:mm:ss`; weekday uses `星期一` through `星期日`; date uses unpadded `yyyy/M/d`, matching the reference. English uses `hh:mm:ss AM/PM`, full weekday names, and `M/d/yyyy`. Midnight and noon map to 12-hour values correctly. The status refresh loop observes the new seconds field, so visual updates occur once per second without a separate timer or synthetic value.

## Layout contract

The clock control reserves 112 logical pixels and remains a single Button hit target. At three rows it renders time, weekday, and date; at one or two rows it renders time and date. Every line is full-width, non-wrapping, and text-centered. The right-side reservation grows by the same width delta so task buttons never overlap the clock. Existing hover, pressed, focus, calendar action, taskbar row geometry, and high-contrast behavior remain unchanged.

## Accessibility and failure behavior

The UI Automation label contains time, weekday when visible, and date in visual order. Calendar activation remains identical for pointer, Enter, and Space. Local-time acquisition is query-only; an invalid calendar input in pure formatting tests fails to a bounded neutral weekday instead of panicking. No Explorer window, Settings URI, registry write, or locale mutation is introduced.

## Verification

Pure tests cover midnight, noon, AM/PM, seconds, leap dates, all seven weekdays, Traditional Chinese and English order, one/two/three-row visibility, 112-DIP width, centered source contract, and unchanged calendar activation. Geometry tests cover 96/144/168/216 DPI and one-to-three rows. Headful verification at 168 DPI captures the three-row Traditional Chinese clock in light, dark, and high contrast and verifies UIA text, containment, one-second advancement, and calendar activation. Full format, locked workspace check/test, warnings-as-errors Clippy, strict OpenSpec validation, and evidence privacy checks remain blocking.

## Scope limits

This change does not add user-selectable clock formats, seconds toggles, time-zone controls, alternate calendars, notification badges, or native Explorer clock delegation.
