## Context

The taskbar already owns AppBar geometry, tasks, native icons, long/short indicators, progress, attention, notifications, system status, IME and clock. Presentation values are scattered through `TaskbarView`, producing inconsistent spacing and no shared hover/focus palette. The approved source design is `docs/superpowers/specs/2026-08-18-windows11-taskbar-chrome-design.md`.

## Goals / Non-Goals

**Goals:**

- Centralize light/high-contrast taskbar chrome.
- Align control density, labeled task width, indicators and system-region typography.
- Localize Search and compact IME presentation without changing provider data.
- Preserve all existing behavior and Explorer-free routes.

**Non-Goals:**

- Do not remove multi-row or labeled modes to imitate Windows defaults.
- Do not change AppBar reservation, tracking, status providers or settings persistence.

## Decisions

`TaskbarChromeTokens` is a presentation-only Copy value selected by the existing high-contrast signal. Every stateful control uses the same panel, border, hover, pressed and focus tokens.

Rows remain 40 logical pixels. Labeled tasks become 160px wide with a 144px indicator; icon-only tasks retain their current compact width and short indicator. The fixed SuperExplorer entry follows the same density. The system region becomes 210px: two 36px icon cells, 44px IME and 82px clock, with explicit 12/11px typography.

`compact_input_language` maps `zh-TW`/`zh-CN` to `中`, English locales to `ENG`, and otherwise returns a bounded uppercase language prefix. Search presentation uses the same `zh-TW`/English locale choice as owned Start.

Blocking gates are `G-TASKBAR-CHROME`, `G-TASKBAR-A11Y`, `G-TASKBAR-LOCALE`, `G-SHELL-NONINTERFERENCE`, `G-TRACE` and `G-PACKAGE`. A-level mechanics may change; B-level in-scope geometry corrections reopen evidence; C-level scope/gate changes require approval.

## Risks / Trade-offs

- **[Reduced label width truncates titles]** → Preserve ellipsis and full accessible names.
- **[Multi-row differs from stock Windows 11]** → Align each row's chrome while retaining the user-requested extension.
- **[IME mapping hides detail]** → Keep full locale in UIA name/provider snapshot; compact only visible text.

## Migration Plan

Add tokens/helpers, restyle controls, run deterministic/headful gates, rebuild both installers. Rollback is a source revert with no settings migration.

## Open Questions

None.
