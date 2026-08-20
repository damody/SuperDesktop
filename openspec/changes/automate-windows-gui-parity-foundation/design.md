## Context

The authoritative design is `docs/superpowers/specs/2026-08-20-windows-gui-parity-automation-design.md`. Existing UTIT cases have independent JSON shapes and embed geometry thresholds in scripts. Production has multiple popup formulas in `surface_runtime.rs`, while recovery and normal Explorer references are distinguishable only by manual inspection.

## Goals / Non-Goals

**Goals:** complete first-wave surface inventory; normalized physical/DIP reports; manifest-to-catalog closure; Windows metric reuse; automatic Explorer-free source/runtime gates; first-wave shell chrome corrections.

**Non-Goals:** claiming every desktop/SuperExplorer view is pixel-perfect in this wave; fabricating mixed-DPI/reboot/review evidence; prohibiting explicit recovery or Return to default Explorer; screenshot equality across user content.

## Decisions

### Compiled manifest is authoritative

`superdesktop-utit` owns typed `GuiSurfaceSpec`, `GuiVariant`, `GeometryRule`, and `ExplorerPolicy` values. A compiled manifest avoids runtime config drift and allows catalog unit tests to prove bidirectional coverage. Alternatives—script-local constants or screenshots only—cannot prove closure and are rejected.

### Normalized reports use physical bounds plus one DIP conversion

Scripts record physical rectangles and window DPI. Rust validation performs `physical * 96 / dpi` once, evaluates absolute/range/ratio/containment rules, and reports exact deltas. Preview and committed-shell anchor modes are separate variants. Text scaling can expand height but cannot shrink hit targets or cause overlap.

### Shared production metrics

First-wave shell chrome uses `WindowsGuiMetrics` constants for row heights, target sizes, widths, padding, radii, and gaps. Content-dependent heights remain bounded formulas. Composition owns monitor clamping and popup anchors; views own internal layout.

The taskbar has no implicit SuperExplorer entry. SuperExplorer participates only through the ordinary tracked-task path when running, or through an explicit persisted user pin; Win+E and the desktop entry remain separate launch surfaces.

### Explorer policy is path-aware

A source auditor classifies explicit allowed modules/actions: guardian recovery, installer rollback, test watchdog, and Return to default Explorer. Any Explorer token in normal product composition, UI callbacks, providers, or SuperExplorer launch paths fails. Headful cases also prove Explorer absence during measurement.

## Failure handling and evidence

Unknown surface IDs, missing variants, duplicate rules, invalid rectangles/DPI, missing controls, ratio drift, stale UIA, absent artifacts, Explorer presence, or recovery failure fail the case. Conditional hardware gates use blocked/not-applicable evidence. Each task writes hashed evidence under this change.

## Risks / Trade-offs

- [Reference values differ by Windows build] -> version manifest entries by Windows reference family and keep tolerances explicit.
- [Screenshot noise] -> gate geometry/UIA first and retain screenshots as evidence, not sole authority.
- [Large migration] -> first accept legacy reports through adapters while requiring normalized reports for new/converted cases.
- [Recovery token false positives] -> path-aware allowlist plus tests that inject forbidden normal-path references.

## Migration Plan

Add manifest/model/validator tests, convert first-wave scripts, centralize metrics, run `gui-parity`, fix failures, then make closure mandatory. Rollback removes the new catalog gate and shared tokens together; no persisted user schema or production IPC changes.

## Adjustment policy

A-level changes may split tasks or tune polling. B-level corrections within first-wave scope update design/spec/tasks and stale dependent evidence. C-level changes to scope, reference family, thresholds, Explorer allowances, public contracts, or mandatory gates require user approval and cannot be silently weakened.

## Open Questions

None for wave 1. Physical mixed-DPI and independent visual review retain explicit external dispositions.
