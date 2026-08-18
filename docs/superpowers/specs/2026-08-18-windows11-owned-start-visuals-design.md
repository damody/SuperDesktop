# Windows 11 owned Start visual alignment

## Outcome

SuperDesktop's owned Start remains the only Start surface in preview and Shell modes, but its presentation aligns with Windows 11 build 26200 at the host's 175% DPI. Traditional Chinese systems display localized shell labels rather than an English-only test surface.

## Scope

The change owns Start presentation only: localization strings, panel tokens, search field, section headings, pinned/recommended cells, All apps navigation, footer controls, Power flyout, hover/focus/pressed states, high contrast and capture evidence. It does not change search ranking, app discovery, activation, persistence, confirmation, placement ownership or Explorer-free routing.

## Architecture

`StartModel` and `StartEffect` remain pure and unchanged. `StartView` derives a bounded `StartStrings` table from `SUPERDESKTOP_LOCALE` for deterministic evidence, otherwise from the Windows user locale. Supported presentation locales are Traditional Chinese and English; every unsupported locale falls back to English without malformed glyphs.

Visual tokens are local to the Start presentation layer: panel/background/border, search surface, neutral hover/pressed fills, accent focus border, typography, 12px outer radius, cell radius, shadow and high-contrast alternatives. All pointer, keyboard and UIA routes continue to emit the same typed effects.

## Windows 11 layout

The logical window remains 640×720 and scales once through GPUI. The search field uses a leading search glyph and Windows-like inset. Pinned keeps a six-column grid, Recommended keeps two columns, and All apps preserves bounded alphabetical ordering. The footer displays the account identity on the left and compact Settings/Power actions on the right, retaining Settings accessibility while reducing the oversized text-button appearance.

## Explorer independence

No Start path may call `explorer.exe`, `StartMenuExperienceHost`, `SearchHost`, `ShellExperienceHost` or the historical Start-host probe. Windows Settings remains an allowed typed platform activation because it is not Explorer or a delegated Start surface; unavailable providers render truthful owned states.

## Verification

Automated gates cover locale selection/fallback, long labels, section bounds, six/two-column density, hover/focus/pressed/high-contrast source contracts, typed action preservation and forbidden delegation. Headful gates capture Home, All apps and Power at 175% with Traditional Chinese labels and stable UIA bounds. Full workspace tests, clippy, release builds, strict/detailed OpenSpec validation and both installers must pass. The change remains unarchived.
