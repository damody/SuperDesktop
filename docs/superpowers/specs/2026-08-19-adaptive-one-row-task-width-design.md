# Adaptive One-Row Task Width Design

## Intent

Shrink labeled task buttons before clipping them when a one-row taskbar becomes crowded, while preserving the notification/status exclusion region and all visual state layers.

## Design

Compute logical available width from the live GPUI window: subtract Start, optional Search, optional Task View, the 160 DIP fixed SuperExplorer entry, dynamic notification-area reserve, and the 210 DIP status region. Divide by `ceil(task_count / rows)` and clamp labeled buttons to 44–160 DIP. Icon-only buttons remain 44 DIP. If every task cannot fit at 44 DIP, existing overflow clipping remains the terminal fallback.

The same adaptive width drives hit target, label ellipsis, progress fill, attention surface, and long running indicator. A pure helper covers zero tasks, 1–3 rows, 44/160 boundaries and crowded/narrow widths. UTIT records every task rectangle and asserts width bounds, non-overlap, order, adaptive shrink under crowding, and exclusion from right controls. No Explorer call or new persistence is introduced.
