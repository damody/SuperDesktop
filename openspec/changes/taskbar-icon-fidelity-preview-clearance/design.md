## Context

Task buttons render their icon at 24 DIP, but task enumeration always requests 32 physical pixels, `window_icon` asks for small icons before large icons, and `taskbar-ui` prefers BC7 whenever the renderer supports it. On a 144–192 DPI monitor that combines upscaling with lossy block compression. Task preview geometry currently anchors to `MonitorRecord.work_area.bottom`; in Explorer-compatible preview mode that is only the native taskbar top and does not reserve the SuperDesktop taskbar drawn immediately above it.

The approved source design is `docs/superpowers/specs/2026-08-21-taskbar-icon-fidelity-preview-clearance-design.md`.

## Goals / Non-Goals

**Goals:**

- acquire enough physical icon detail for the highest active monitor DPI;
- preserve small-icon pixels and alpha without lossy texture compression;
- keep previews above the visible SuperDesktop taskbar for all supported row counts and shell modes;
- make both properties deterministic and testable without changing the visual size of task icons.

**Non-Goals:**

- redesign task buttons, Jump Lists, preview card content, animation, or grouping;
- replace Windows icon-resource selection with a new icon theme;
- move or resize the native Explorer taskbar;
- add settings or migrate persisted configuration.

## Decisions

### 1. Size the source for the highest active monitor DPI

The application computes `ceil(24 * max(dpi_x, dpi_y) / 96)` and clamps it to 32–64 px. One shared task list can be rendered on multiple monitors, so using the maximum active DPI prevents an icon cached for a lower-DPI monitor from becoming the source of later upscaling. Invalid DPI falls back to 96.

Per-window/per-monitor caches were rejected because they add invalidation complexity while the 64 px upper bound keeps memory small.

### 2. Prefer size-matched resources and large borrowed icons

When an executable path fits the Windows resource API contract, `PrivateExtractIconsW` requests the selected edge. The owned result is converted and destroyed. Window messages and class-icon fallbacks query large before small, and borrowed handles are never destroyed. Existing shell and executable extraction remains the recovery chain.

Keeping the old small-first order was rejected because a successful 16 px handle prevents every higher-quality fallback.

### 3. Keep small task icons lossless

`taskbar-ui` uploads icons whose width and height are at most 64 px as uncompressed BGRA. BC7 stays available for larger raster inputs. This avoids block artifacts around thin strokes and transparent edges while adding at most 16 KiB per 64 px icon.

Always disabling BC7 was rejected because this change does not own large-image memory policy.

### 4. Derive preview placement from the same taskbar mode and rows

Preview geometry receives the effective owned-shell boolean and configured row count. Its taskbar bottom is monitor bounds bottom for an owned shell, or Windows work-area bottom for Explorer-compatible mode. Subtracting the DPI-aware SuperDesktop taskbar height yields the taskbar top; the preview outer bottom is clamped above that top with the popup gap.

Delayed hover callbacks capture these values rather than recomputing an unrelated default. Negative origins remain valid because calculations use the selected monitor's logical coordinates.

Using only the Windows work area was rejected because Windows reserves the native taskbar but has no knowledge of the preview-mode SuperDesktop taskbar.

### 5. Gate with deterministic and headful evidence

Unit tests cover source-edge sizing, lossless upload, and geometry matrices. A focused headful UTIT run records the real preview and taskbar rectangles while Explorer exists and asserts no intersection. The focused case must pass twice, followed by formatting, workspace tests, Clippy with warnings denied, release build, and installer production.

Evidence records use the change evidence index with immutable task IDs and content hashes. Large raw captures may live under the parent build log directory, with hashed pointers retained in the change.

## Failure handling and observability

Windows API failures return to the existing icon fallback chain; no task disappears and no `unwrap`/panic is introduced. Bad DPI becomes 96, bad rows become the supported one-row minimum, and geometry remains clamped to the monitor. Runtime failures are printed through existing console diagnostics. Test-visible traces include selected source edge and popup/taskbar rectangles.

## Risks / Trade-offs

- [Some executables expose no resource at the requested size] → fall back through large window/class and shell icons without stretching a known small icon first.
- [Maximum-monitor sizing increases icon memory] → cap at 64 px and keep the 24 DIP display size.
- [Private icon extraction has a fixed path buffer contract] → use it only for representable paths and preserve existing long-path fallbacks.
- [Window or monitor state changes during delayed hover] → validate the target task at open time and use the scheduled taskbar layout snapshot, matching the visible surface that initiated the hover.
- [Headful geometry can be affected by stale installed binaries] → record binary hash/version and run focused validation twice from the release candidate.

## Migration Plan

Land the platform, UI, runtime, tests, and evidence changes atomically. Build the release candidate and installer only after all source gates pass. There is no data migration. Rollback is a code revert and reinstall of the previous package.

## Plan correction policy

- **A — task refinement:** leaf split/order/command/evidence-path updates that preserve requirements and gates.
- **B — design/spec correction:** Windows API or GPUI constraints discovered inside this scope require pausing affected work, updating design/spec/tasks, marking dependent evidence stale, and revalidating.
- **C — material change:** weaker gates, different public behavior, permissions, dependencies, destructive operations, or scope expansion require user approval.

## Open Questions

None. Windows API availability and headful geometry are blocking implementation evidence rather than deferred design questions.
