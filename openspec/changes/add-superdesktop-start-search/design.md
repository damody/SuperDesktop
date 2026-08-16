## Context

M0 delegates Start to the existing Windows host in preview mode and truthfully reports shell-mode Start as deferred. The completion shell must own its Start surface. The provider protocol already has search result DTOs and process isolation.

## Goals / Non-Goals

**Goals:** Add an owned Start model, app/settings/file providers, streaming ranked results, cancellation, activation, persisted pins/recent items, keyboard/IME/accessibility behavior, and explicit provider states.

**Non-Goals:** Replace Windows Search indexing, provide web search, collect cloud activity, or execute untrusted provider code in GPUI.

## Decisions

1. Replace the shell-mode deferred fixture with a GPUI-owned `StartModel`; preview mode retains the controlled host probe for compatibility evidence.
2. Add versioned search request/batch DTOs. Provider generations make old batches harmless after query replacement.
3. Discover applications from admitted Start Menu roots and settings from a curated URI catalog. File search is bounded to configured roots, item count, depth, and deadline.
4. Rank normalized prefix, word-prefix, substring, and recency signals deterministically; category and stable ID break ties.
5. IME composition updates text without dispatch; committed text starts a 50 ms debounce. Empty query shows pins/recent/all-apps.

## Risks / Trade-offs

- [Filesystem search can be slow] → Bound roots, depth, results, deadline, and isolate it in the provider host.
- [App shortcuts can be malformed] → Return owned metadata and fail individual items without failing Start.
- [Stale results flash] → Accept batches only for the active query generation.
- [Settings URIs vary] → Curate capability-tagged entries and hide unsupported entries.

## Migration Plan

Keep preview-host behavior, add owned shell-mode Start, then route the taskbar Start action according to execution mode. Settings persistence is additive and can be removed without affecting files.

## Open Questions

None.
