## Context

The owned 640×720 `StartView` already supports Search, Pinned, Recommended, All apps, Account, Settings and confirmed Power. Its hardcoded English labels and flat controls create a large visual mismatch on the Traditional Chinese Windows 11 host. The approved source design is `docs/superpowers/specs/2026-08-18-windows11-owned-start-visuals-design.md`.

## Goals / Non-Goals

**Goals:**

- Localize visible and accessible Start presentation labels to Traditional Chinese or English.
- Align panel, search, grids, footer, Power flyout and interaction states to Windows 11.
- Preserve the existing model, effects, placement, provider failure and exclusive ownership contracts.

**Non-Goals:**

- Do not change app discovery, ranking, search, persistence or destructive-action confirmation.
- Do not invoke Explorer, Windows Start hosts or system-owned Start UI.

## Decisions

`StartStrings` is a bounded value table selected from `SUPERDESKTOP_LOCALE` when set, otherwise `platform_win::taskbar_status::user_locale_name()`. Only `zh-TW` and English are presentation variants; unsupported locales fall back to English.

`StartVisualTokens` centralizes light and high-contrast colors, borders, radius and state fills. Existing fixed geometry remains the compatibility boundary, while footer actions become compact icon-led controls. All click, key and UIA callbacks continue to use the current `StartActions` routes.

The source guard expands to reject Explorer and system Start presentation protocols in production composition. Windows Settings remains an allowed typed activation because it is neither Explorer nor a delegated Start surface. Provider failures remain localized owned messages.

Blocking gates are `G-START-LOCALE`, `G-START-VISUAL`, `G-START-A11Y`, `G-SHELL-NONINTERFERENCE`, `G-TRACE` and `G-PACKAGE`. A-level task refinements may adjust commands; B-level in-scope geometry corrections reopen affected tasks and evidence; C-level scope or gate changes require user approval.

## Risks / Trade-offs

- **[Long Traditional Chinese labels clip]** → Keep bounded ellipsis containers and capture 175% UIA bounds.
- **[Localization changes model identity]** → Localize presentation only; stable IDs and typed effects remain unchanged.
- **[High contrast relies on color]** → Use visible borders and focus geometry in addition to palette changes.

## Migration Plan

Add strings/tokens, restyle each surface, run model/source/headful gates, then rebuild standalone and combined installers. Rollback is a source revert; no settings migration is required.

## Open Questions

None.
