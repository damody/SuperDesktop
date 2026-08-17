# SuperDesktop Taskbar Live Parity Design

## Context

The active Windows 11 + ExplorerPatcher reference profile uses `TaskbarGlomLevel=2`, meaning taskbar buttons are never combined and visible windows retain labels. SuperDesktop currently defaults `combine_groups=true`.

The taskbar model calculates multi-row slots column-major (`row = index % rows`), but `TaskbarView` ignores those slots and uses row-direction flex wrapping. On a wide display, all buttons remain in the first row while lower configured rows are blank.

Production status also calls a fixture-only `status()` function containing a fixed 2026-08-14 11:22 clock, so an installed taskbar never displays current local time.

## Goals

- Default new and partially specified settings to never combine task windows, matching the current host setting.
- Preserve explicit `combine_groups=true` for users who choose grouping.
- Fill configured taskbar rows vertically before creating the next column.
- Read local date/time through a thin Windows platform adapter and update visible taskbars when the minute changes.
- Preserve task identity, actions, labels, grouping support, AppBar height, and notification provider behavior.

## Non-Goals

- Real task icon extraction and caching.
- Automatically mirroring every Windows taskbar registry preference.
- Hiding Task View from the Windows `ShowTaskViewButton` setting.
- Replacing the notification-area provider or implementing undocumented tray compatibility.

## Design

Change `TaskbarSettings::default()` and missing-field decoding so `combine_groups=false`. Explicit persisted values remain round-trippable.

Change the task region from row-direction wrapping to column-direction wrapping while retaining full-height containment and fixed 40-logical-pixel buttons. This causes the fixed SuperExplorer entry and subsequent tasks to fill rows top-to-bottom, then wrap into the next column, consistent with `TaskbarLayout`.

Add `platform_win::common::taskbar_status::local_date_time()`, returning an owned date/time record from `GetLocalTime`. `superdesktop-app` maps it to the existing `TestClock` value type so deterministic formatter tests remain reusable. The 50 ms reconciliation loop computes the current status and updates the GPUI entity only when the minute/date changes.

Verification covers never-combine defaults, explicit grouping compatibility, one/two/three-row packing policy, live-clock platform bounds, minute-change UI updates, workspace tests, and a 175% DPI headful capture showing both rows populated and a current clock.

## Alternatives

- Merely change labels: rejected because blank lower rows and the stale clock remain visible.
- Mirror every Explorer/ExplorerPatcher registry value: deferred because it creates a broad compatibility subsystem and couples production behavior to undocumented settings.
- Implement icons in the same change: deferred because icon extraction, ownership, DPI invalidation, and caching have independent failure modes.

## Rollback

Revert the defaults, flex direction, platform clock adapter, and refresh assignment. No settings schema or external system state migration is required.
