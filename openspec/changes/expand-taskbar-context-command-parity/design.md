## Context

The existing owned context menu has a stable typed-command model but only three rows. Settings already contain Search mode, Task View visibility, and Lock state; Show desktop already has an owned session-based minimize/restore implementation. The change connects those existing capabilities without introducing Explorer or a new schema.

## Goals / Non-Goals

**Goals:**

- Render six ordered Windows-style context commands in a bounded 220 DIP popup.
- Expose Search mode, Task View, and Lock with truthful current state and atomic persistence.
- Route Show desktop through the existing owned session.
- Measure the full menu and accessibility order in Explorer-free UTIT.

**Non-Goals:**

- Nested Search submenus, toolbar extension menus, window arrangement, or new settings fields.
- Invoking Explorer-owned taskbar, Start, settings, or Show desktop UI.

## Decisions

`TaskbarContextModel::COMMANDS` becomes the single six-item order: CycleSearchMode, ToggleTaskView, ShowDesktop, OpenTaskManager, ToggleLockTaskbar, and OpenTaskbarSettings. Search cycles Hidden -> Icon -> Box -> Hidden. A nested submenu was rejected for this iteration because it requires a second popup navigation state; static rows were rejected because they would expose nonfunctional UI.

`TaskbarContextView` receives current Search, Task View, and Lock values. It derives labels and checked accessibility state from those values and continues emitting only typed commands. Each row remains 32 DIP; popup height is derived from the six rows, two-pixel gaps, and four-DIP padding rather than a magic size unrelated to content.

`surface_runtime` uses a pure helper for the three settings mutations. It clones the complete settings document, mutates one taskbar field, and publishes the saved snapshot only after the revisioned store succeeds. ShowDesktop receives a separately cloned `ShowDesktopSession`; Task Manager and owned settings retain their existing isolated branches.

The Explorer-free resize test enumerates UIA MenuItem descendants, records names and bounds in order, and verifies exact command count and semantic names. It converts the popup rectangle using `GetDpiForWindow`, requires 200-240 DIP width and content-fit height, then activates the existing Lock row so persistence and resize behavior remain exercised.

## Risks / Trade-offs

- **Risk: Search cycling is less exact than Windows' nested submenu.** -> The label exposes the current mode and every mode remains reachable in a bounded deterministic sequence.
- **Risk: Save failure could leave the menu label stale.** -> The popup dismisses after activation and live settings change only on successful save.
- **Risk: Locale text changes affect literal UTIT names.** -> The headful case fixes `en-US`; unit tests cover Traditional Chinese labels separately.

## Migration Plan

No migration is required. Existing settings values are consumed as-is. Rollback removes the three command variants and handlers; persisted documents remain valid.

## Open Questions

None.
