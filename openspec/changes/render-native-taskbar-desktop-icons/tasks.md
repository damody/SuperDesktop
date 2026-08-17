## 1. Native Icon Boundary

### 1.1 Win32 extraction and ownership

**?桃?嚗?* Owned validated RGBA icon data with deterministic Win32 lifetime behavior.  
**頛詨嚗?* Approved design, Windows API ownership contracts, existing `IconData`.  
**?Ｗ嚗?* `platform-win` icon module and tests.  
**靘陷嚗?* None.  
**Owner嚗ave嚗?* Primary agent / wave 1.  
**Gate嚗vidence嚗?* `G-NATIVE-ICON-PARITY`; `evidence/evidence-index.json`.  
**摰??瑼鳴?** Existing Shell items and windows resolve without leaked owned handles.

- [x] 1.1.1 Implement Shell path icon acquisition with owned-HICON destruction.
- [x] 1.1.2 Implement bounded window/class/executable fallback without destroying borrowed icons.
- [x] 1.1.3 Implement top-down DIB BGRA-to-RGBA conversion and legacy-alpha repair.
- [x] 1.1.4 Add pixel validation and native extraction/resource-lifetime tests.

## 2. Taskbar Application Icons

### 2.1 Task model and reconciliation cache

**?桃?嚗?* Every live task carries cached owned icon pixels.  
**頛詨嚗?* Platform icon API and live task snapshots.  
**?Ｗ嚗?* Task model field, resolver/cache composition, pruning tests.  
**靘陷嚗?* 1.1.  
**Owner嚗ave嚗?* Primary agent / wave 2.  
**Gate嚗vidence嚗?* `G-NATIVE-ICON-PARITY`; targeted test output in evidence index.  
**摰??瑼鳴?** Unchanged tasks reuse icons and stale entries are bounded/pruned.

- [x] 2.1.1 Add optional owned icon data to accessible task models and fixtures.
- [x] 2.1.2 Resolve live-window icons during task composition and reuse a bounded identity cache.
- [x] 2.1.3 Prune stale task icon entries and test unchanged/removed identity behavior.

### 2.2 Taskbar rendering

**?桃?嚗?* Task buttons show the native icon before the readable label.  
**頛詨嚗?* Icon-bearing accessible tasks and existing GPUI image conversion.  
**?Ｗ嚗?* Taskbar view implementation and source/render tests.  
**靘陷嚗?* 2.1.  
**Owner嚗ave嚗?* Primary agent / wave 2.  
**Gate嚗vidence嚗?* `G-NATIVE-ICON-PARITY`; taskbar screenshot.  
**摰??瑼鳴?** 24-logical-pixel icons render without shrinking labels or actions.

- [x] 2.2.1 Generalize validated RGBA-to-GPUI conversion for task icons.
- [x] 2.2.2 Render task icons with fixed sizing, aspect preservation, and generic fallback.
- [x] 2.2.3 Add tests preserving label, hit-target, grouping, and task-action contracts.

## 3. Desktop Shell Icons

### 3.1 Desktop model and reconciliation cache

**?桃?嚗?* Every accessible desktop node carries cached Shell icon pixels when available.  
**頛詨嚗?* Canonical desktop paths, fixed SuperExplorer entry, platform icon API.  
**?Ｗ嚗?* Desktop model field, path resolver/cache, pruning tests.  
**靘陷嚗?* 1.1.  
**Owner嚗ave嚗?* Primary agent / wave 3.  
**Gate嚗vidence嚗?* `G-NATIVE-ICON-PARITY`; targeted test output in evidence index.  
**摰??瑼鳴?** Files, folders, shortcuts, and fixed entry resolve or degrade safely.

- [x] 3.1.1 Add optional owned icon data to accessible desktop nodes and fixtures.
- [x] 3.1.2 Resolve canonical-path and fixed-entry icons during namespace reconciliation.
- [x] 3.1.3 Reuse and prune the bounded desktop icon cache with changed-item tests.

### 3.2 Desktop rendering

**?桃?嚗?* Desktop tiles show native Shell icons above labels.  
**頛詨嚗?* Icon-bearing accessible nodes and GPUI conversion helper.  
**?Ｗ嚗?* Desktop view implementation and interaction regression tests.  
**靘陷嚗?* 3.1.  
**Owner嚗ave嚗?* Primary agent / wave 3.  
**Gate嚗vidence嚗?* `G-NATIVE-ICON-PARITY`; desktop screenshot.  
**摰??瑼鳴?** 48-logical-pixel icons preserve selection, labels, drag, and operations.

- [x] 3.2.1 Add validated desktop RGBA-to-GPUI image conversion.
- [x] 3.2.2 Replace the placeholder glyph with native image and stable fallback rendering.
- [x] 3.2.3 Add tests preserving desktop layout, selection, labels, and operations.

### 3.3 Color-correct BC7 rendering

**Outcome:** Correct visible colors with direct compressed GPU icon uploads where supported.  
**Inputs:** RGBA icon contract, GPUI Windows BGRA contract, approved BC7 expansion.  
**Outputs:** Updated GPUI fork, BC7 encoder/cache, BGRA fallback, color and payload tests.  
**Dependencies:** 2.2 and 3.2.  
**Owner/Wave:** Primary agent / wave 3.  
**Gate/Evidence:** `G-NATIVE-ICON-PARITY`; production GPU trace and screenshots.  
**Completion threshold:** Channel tests pass, direct BC7 upload is observed, and known icon colors are visually correct.

- [x] 3.3.1 Correct RGBA-to-GPUI BGRA conversion and add byte-level channel tests.
- [x] 3.3.2 Upgrade to the validated BC7-capable GPUI fork and vendor its exact sources.
- [x] 3.3.3 Add bounded 4x4 BC7 encoding and cached hardware-gated render images.
- [x] 3.3.4 Add BC7 size/layout/render tests and production upload telemetry.

## 4. Production Verification and Packaging

### 4.1 Automated and headful release gate

**?桃?嚗?* Auditable proof that native icons work without resource regressions.  
**頛詨嚗?* Integrated platform/app/UI implementation.  
**?Ｗ嚗?* Test logs, screenshots, evidence index, validation report.  
**靘陷嚗?* 2.2 and 3.2.  
**Owner嚗ave嚗?* Primary agent / wave 4.  
**Gate嚗vidence嚗?* `G-NATIVE-ICON-PARITY`; `evidence/evidence-index.json`.  
**摰??瑼鳴?** Every required command exits zero and headful evidence has no placeholder-only items.

- [ ] 4.1.1 Run formatting, focused clippy/tests, and locked offline workspace check.
- [ ] 4.1.2 Run repeated native extraction and record stable GDI-object evidence.
- [ ] 4.1.3 Capture and inspect production taskbar and desktop icons at host DPI.
- [ ] 4.1.4 Record task-linked evidence and pass strict OpenSpec validation.

### 4.2 Installer integration

**?桃?嚗?* Standalone and combined installers contain the verified icon implementation.  
**頛詨嚗?* Gate-passing release sources.  
**?Ｗ嚗?* Rebuilt installers with hashes.  
**靘陷嚗?* 4.1.  
**Owner嚗ave嚗?* Primary agent / wave 5.  
**Gate嚗vidence嚗?* Installer hashes in evidence index.  
**摰??瑼鳴?** Both expected installer artifacts build successfully and are hashed.

- [ ] 4.2.1 Build the SuperDesktop standalone test installer.
- [ ] 4.2.2 Build the combined SuperExplorer installer and record artifact hashes.
