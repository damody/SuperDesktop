## Context

Taskbar models contain live HWND and executable identities but no icon pixels; desktop models contain canonical paths but the view always renders `▣`. The approved design is `docs/superpowers/specs/2026-08-17-windows-shell-icon-rendering-design.md`. The change spans Win32 handle ownership, app reconciliation/cache behavior, and two GPUI views.

## Goals / Non-Goals

**Goals:** Render recognizable native icons for live applications and desktop Shell items, keep refresh bounded and leak-free, preserve existing labels/actions/layout, and prove behavior on the active Windows 11 host.

**Non-Goals:** Reimplement the Windows image list, add thumbnail previews, change task grouping, replace Shell overlays, or persist icon pixels in settings.

## Decisions

1. `platform-win::common::icon` is the only Win32 icon boundary and returns owned `IconData`; UI crates never own native handles. This centralizes safety and reuses the existing protocol type instead of adding a second pixel contract.
2. Taskbar resolution uses bounded `WM_GETICON` small/big requests, then class icon, then `SHGetFileInfoW` on the executable. Borrowed window/class handles are not destroyed; Shell-returned handles are destroyed after conversion.
3. Desktop resolution uses `SHGetFileInfoW` on the canonical path so file associations, folders, shortcuts, and overlays remain Windows-owned behavior.
4. Conversion draws to a zero-initialized top-down 32-bit DIB, converts BGRA to RGBA, repairs the legacy all-zero-alpha case, restores the selected object, and releases GDI resources on every path.
5. App composition owns bounded caches. Taskbar keys use window identity plus executable fallback identity; desktop keys use canonical paths. Reconciliation prunes stale entries and never caches malformed pixel buffers.
6. Taskbar and desktop views validate RGBA buffers, convert to GPUI's documented Windows BGRA upload contract, and render 24- and 48-logical-pixel images respectively.
7. The approved user expansion upgrades GPUI to the existing BC7-capable fork revision and encodes each distinct icon once into 4x4/16-byte BC7 blocks. A bounded strong render cache prevents per-frame re-encoding. Hardware capability gates direct upload; unsupported adapters retain the corrected BGRA path.
8. The current polychrome shader preserves display-encoded values and targets a UNORM swap chain, so the BC7 resource uses `DXGI_FORMAT_BC7_UNORM`. Using `_SRGB` here would perform an unmatched sample-time linearization and darken icons; `_SRGB` remains a future option only with a paired output transfer-function change.
9. Gate `G-NATIVE-ICON-PARITY` requires unit/integration tests, stable GDI-object counts across repeated extraction, a real BC7 GPU upload trace, headful color inspection, strict validation, and rebuilt installers.

Implementation evidence may refine task splits or commands without changing scope (A). A correction to APIs, caching, or rendering inside this scope requires updating design/spec/tasks and reopening dependent evidence (B). Changes to supported platform, gate thresholds, permissions, registry behavior, dependencies, or required evidence are material and require user approval (C).

## Risks / Trade-offs

- **[Hung application window]** → Use `SendMessageTimeoutW` and continue to class/file fallback.
- **[GDI handle leak]** → Encode ownership in narrow helpers and block on repeated-extraction resource evidence.
- **[High-frequency Shell work]** → Cache owned pixels and prune by the current live identity set.
- **[Legacy alpha masks]** → Repair only an all-zero-alpha buffer with visible RGB; otherwise preserve source alpha.
- **[Icon unavailable]** → Keep labels, hit targets, and a generic fallback; never suppress the item.

## Migration Plan

Land platform extraction, model propagation, caches, and both views atomically. Run tests and headful capture, then rebuild standalone and combined installers. Roll back by reverting source/model changes; no persisted migration or registry cleanup is required.

## Open Questions

None.
