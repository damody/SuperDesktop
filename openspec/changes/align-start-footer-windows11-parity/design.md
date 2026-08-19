## Context

The Start footer currently owns account, Settings, and Power controls. Windows 11 uses account and Power only. Settings activation is already available through the Start catalog and owned taskbar settings, so the footer gear is redundant rather than a required recovery path.

## Goals / Non-Goals

**Goals:** remove the footer gear, retain one 40 DIP Power button and account control, preserve the 52 DIP footer and owned power popup, and make live UTIT reject regression.

**Non-Goals:** changing Start window geometry, pins, search, account activation, power commands, settings discovery, or Explorer lifecycle.

## Decisions

Delete only the `start-settings` render subtree and its render-local callback clones. Keep the footer action group as the accessible container for one Power child. This avoids changing StartActions or provider protocols. Moving Settings elsewhere was rejected because search/pins already provide the Windows-consistent location; hiding the gear visually while leaving it accessible was rejected as dishonest accessibility.

The Start capture enumerates Button descendants under the footer group, requires one child named Power, rejects a Settings button, converts its rectangle using the Start HWND DPI, requires 38-42 DIP width and height, and requires an 8-24 DIP right inset. Existing power expansion and three MenuItem checks remain unchanged.

## Risks / Trade-offs

- **Risk: a fixture lacks a Settings search result.** -> Settings remains a catalog/provider concern outside this footer change; taskbar settings remains directly accessible.
- **Risk: DWM scaling rounds the Power target.** -> UTIT uses a 38-42 DIP tolerance around the authored 40 DIP size.

## Migration Plan

No data migration. Rollback restores the render subtree. No persisted state changes.

## Open Questions

None.
