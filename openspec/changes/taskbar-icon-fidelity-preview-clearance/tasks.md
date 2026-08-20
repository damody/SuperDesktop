## 1. Baseline and contracts

### 1.1 Freeze implementation baseline

**目的：** Record the source and behavior baseline before implementation.
**輸入：** Approved design, proposal, capability specs, current repository.
**產出：** Baseline inventory and evidence-index schema/location.
**依賴：** None.
**Owner／Wave：** Primary agent / Wave 1.
**Gate／Evidence：** BASE-01; `evidence/index.jsonl` task IDs 1.1.1–1.1.3.
**完成門檻：** Source paths, existing tests, and immutable evidence record format are recorded without modifying unrelated work.

- [ ] 1.1.1 Record the current nested and parent revisions plus relevant dirty-path exclusions.
- [ ] 1.1.2 Record icon acquisition/upload and preview geometry call paths in the baseline evidence.
- [ ] 1.1.3 Create and validate the append-only evidence index record format used by every resolved leaf.

## 2. DPI-aware icon fidelity

### 2.1 Select and acquire a high-quality icon source

**目的：** Supply taskbar rendering with enough physical detail for active monitor DPI.
**輸入：** `taskbar-icon-fidelity` spec and current `platform-win`/runtime icon paths.
**產出：** DPI source-edge helper and ordered Windows icon extraction/fallback implementation.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** ICON-01, ICON-02; task IDs 2.1.1–2.1.4.
**完成門檻：** 96–192 DPI and failure paths pass focused tests with no leaked owned icon handles or panic paths.

- [ ] 2.1.1 Implement the 24 DIP maximum-monitor source-edge calculation with 32–64 px clamping.
- [ ] 2.1.2 Thread the selected source edge through initial and refreshed task enumeration.
- [ ] 2.1.3 Implement size-matched owned executable-resource extraction with safe path bounds and handle cleanup.
- [ ] 2.1.4 Reorder borrowed window/class icon fallback to prefer large sources while preserving recoverable shell fallbacks.

### 2.2 Preserve small-icon pixels through rendering

**目的：** Remove lossy compression from small task icons without changing their 24 DIP layout.
**輸入：** Validated `IconData` and current taskbar render-image cache.
**產出：** Lossless small-icon upload policy and focused renderer tests.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** ICON-03; task IDs 2.2.1–2.2.3.
**完成門檻：** Icons at most 64 px use byte-preserving BGRA images, invalid payloads remain recoverable, and larger-image policy is unchanged.

- [ ] 2.2.1 Route task icons at most 64 px per dimension to uncompressed BGRA render images.
- [ ] 2.2.2 Preserve the existing BC7 path for valid larger raster inputs.
- [ ] 2.2.3 Add focused tests for byte preservation, dimensions, alpha detail, invalid payloads, and the 24 DIP display contract.

## 3. Preview/taskbar clearance

### 3.1 Derive preview geometry from the visible taskbar

**目的：** Make preview placement reserve the exact visible SuperDesktop taskbar region.
**輸入：** `task-preview-taskbar-clearance` spec and taskbar Windows metrics.
**產出：** Mode/row-aware geometry helpers with monitor clamping.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** GEO-01, GEO-02; task IDs 3.1.1–3.1.4.
**完成門檻：** Geometry matrices prove no intersection in owned-shell and Explorer-compatible modes for 1–3 rows, mixed DPI, and negative origins.

- [ ] 3.1.1 Add effective shell mode and supported taskbar row count to preview geometry inputs.
- [ ] 3.1.2 Calculate the DPI-aware visible taskbar top for owned-shell and Explorer-compatible modes.
- [ ] 3.1.3 Clamp preview outer bounds above the taskbar top and within the selected monitor.
- [ ] 3.1.4 Add focused geometry tests for row, DPI, narrow-monitor, fallback, and negative-origin boundaries.

### 3.2 Keep immediate and delayed preview paths consistent

**目的：** Ensure every preview-opening path uses the initiating taskbar layout.
**輸入：** New geometry contract and taskbar callbacks.
**產出：** Updated immediate click and delayed hover call chains.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / Wave 2.
**Gate／Evidence：** GEO-03; task IDs 3.2.1–3.2.3.
**完成門檻：** All production call sites pass explicit mode/rows, stale targets recover safely, and no default path can reintroduce overlap.

- [ ] 3.2.1 Thread effective shell mode and rows through immediate preview opening.
- [ ] 3.2.2 Capture effective shell mode and rows in delayed hover scheduling.
- [ ] 3.2.3 Audit all production preview geometry call sites for explicit layout inputs and recoverable errors.

## 4. Automated behavior evidence

### 4.1 Run deterministic source-level gates

**目的：** Prove icon and geometry contracts before headful execution.
**輸入：** Completed implementation and focused tests.
**產出：** Formatted source and passing focused/workspace test reports.
**依賴：** 2.2, 3.2.
**Owner／Wave：** Primary agent / Wave 3.
**Gate／Evidence：** SRC-01; task IDs 4.1.1–4.1.3.
**完成門檻：** Format check, focused tests, workspace tests, and Clippy with denied warnings all return exit 0.

- [ ] 4.1.1 Run Rust formatting check and focused icon/geometry tests.
- [ ] 4.1.2 Run the complete workspace test suite.
- [ ] 4.1.3 Run workspace Clippy for all targets with warnings denied.

### 4.2 Prove real Explorer-compatible geometry

**目的：** Verify the installed/release GUI does not place a preview over the SuperDesktop taskbar.
**輸入：** Focused UTIT case, Explorer-compatible desktop session, candidate binary.
**產出：** Two consecutive reports with window rectangles, screenshots, logs, and hashes.
**依賴：** 4.1.
**Owner／Wave：** Primary agent / Wave 4.
**Gate／Evidence：** GUI-01; task IDs 4.2.1–4.2.3.
**完成門檻：** Both runs assert `preview_bottom <= superdesktop_taskbar_top`, open the intended preview, and record no panic/error signature.

- [ ] 4.2.1 Extend the focused hover-preview UTIT report with preview/taskbar rectangle and binary identity fields.
- [ ] 4.2.2 Run the Explorer-compatible focused hover-preview case once and index its evidence.
- [ ] 4.2.3 Repeat the same focused case from a clean launch and index the second passing result.

## 5. Release integration and review

### 5.1 Build and package the verified candidate

**目的：** Produce installable output from the exact verified source.
**輸入：** Passing source/headful gates and clean intended diff.
**產出：** Release binaries, installer, hashes, and packaging report.
**依賴：** 4.2.
**Owner／Wave：** Primary agent / Wave 5.
**Gate／Evidence：** REL-01; task IDs 5.1.1–5.1.3.
**完成門檻：** Release and installer commands return exit 0 and hashes trace to committed nested/parent revisions.

- [ ] 5.1.1 Build the SuperDesktop Windows release candidate and record its hash.
- [ ] 5.1.2 Build the SuperExplorer installer containing the verified candidate and record its hash.
- [ ] 5.1.3 Verify package provenance, expected launch payload, and absence of unrelated staged files.

### 5.2 Close traceability and repository integration

**目的：** Deliver an auditable change with every requirement and task resolved.
**輸入：** All gate evidence and final intended diff.
**產出：** Complete evidence index, checked tasks, strict validation, nested commit, and parent gitlink commit.
**依賴：** 5.1.
**Owner／Wave：** Primary agent / Wave 6.
**Gate／Evidence：** REVIEW-01; task IDs 5.2.1–5.2.4.
**完成門檻：** Every leaf has unique passing evidence, OpenSpec validates strictly, no P0/P1 review issue remains, and only intended tracked paths are committed.

- [ ] 5.2.1 Map proposal, design, requirements, scenarios, gates, and all task IDs in the final evidence index.
- [ ] 5.2.2 Review the implementation for handle ownership, panic paths, geometry consistency, regressions, and unrelated changes.
- [ ] 5.2.3 Run strict OpenSpec/task validation and mark every evidence-backed leaf complete.
- [ ] 5.2.4 Commit the nested change and synchronize only the SuperDesktop gitlink in the parent repository.
