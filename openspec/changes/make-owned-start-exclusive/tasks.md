# Exclusive Owned Start Implementation Tasks

## 1. Ownership Cutover

### 1.1 Add source ownership guards

**目的：** Make system Start delegation a test-detectable product violation.
**輸入：** Approved design, current `surface_runtime.rs`, historical platform Start adapter.
**產出：** Source guard and mode-contract tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-OWNED-START`; `evidence/evidence-index.json` automated record.
**完成門檻：** Tests reject product Start-host calls while allowing the isolated historical capability adapter.

- [ ] 1.1.1 Add a source guard that scans product composition for system Start-host invocation.
- [ ] 1.1.2 Add a mode-contract test proving preview, Shell and verification select one owned renderer.
- [ ] 1.1.3 Add a negative fixture proving the guard rejects a delegated Start callback.

### 1.2 Route every Start action to StartView

**目的：** Remove the product branch that delegates preview Start to Explorer/ExplorerPatcher.
**輸入：** 1.1 failing guards and existing owned Start composition.
**產出：** Unified Start callback and updated trace contract.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-OWNED-START`; focused test output and source diff.
**完成門檻：** No execution mode returns through `invoke_start_host_controlled`; all modes toggle one owned window slot.

- [ ] 1.2.1 Remove the Shell/environment renderer-selection branch from the Start callback.
- [ ] 1.2.2 Preserve exactly-once owned open/close traces and stale-window cleanup.
- [ ] 1.2.3 Remove now-unused product imports/closures without deleting the historical platform capability adapter.

## 2. Owned Start Behavior

### 2.1 Regress sections, input, activation and persistence

**目的：** Prove exclusive ownership preserves all current Start contracts.
**輸入：** Unified callback, `StartModel`, `StartView`, settings store and activation adapters.
**產出：** Focused automated tests across every owned Start mode.
**依賴：** 1.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-OWNED-START`, `G-START-INPUT`; automated test record.
**完成門檻：** Home, All apps, Search, activation, persistence, power, placement and dismissal tests pass in mode-independent fixtures.

- [ ] 2.1.1 Verify Home section bounds, native icons, All apps ordering and truthful unavailable search state.
- [ ] 2.1.2 Verify app/file/folder/Settings activation and pin/recent persistence remain exactly once.
- [ ] 2.1.3 Verify Power is collapsed by default and every destructive action retains explicit confirmation.
- [ ] 2.1.4 Verify Escape, arrows, Enter, repeated toggle, IME generation, UIA focus and 175% placement.

### 2.2 Capture owned Start headful evidence

**目的：** Demonstrate the rendered product is SuperDesktop Start rather than the system Start host.
**輸入：** Release binary, owned Start capture script and active Windows 11 reference host.
**產出：** Home/All apps screenshots, UIA/trace JSON and source-process proof.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-OWNED-START`, `G-START-INPUT`; `evidence/start-*.png`, JSON and logs.
**完成門檻：** Preview and Shell-safe owned routes show the same sections and no system Start-host invocation/process transition.

- [ ] 2.2.1 Update the Start capture harness so no environment variable is required to select owned Start.
- [ ] 2.2.2 Capture and inspect Start Home and All apps at host 175% DPI through pointer/UIA routes.
- [ ] 2.2.3 Record taskbar Start traces and process observations proving exclusive owned presentation.

## 3. Desktop Non-Regression

### 3.1 Reverify marquee and fixed-entry routing

**目的：** Ensure owned Start window composition does not break desktop pointer ownership.
**輸入：** Integrated taskbar/Start composition and existing desktop marquee harness.
**產出：** Geometry tests, input-route report and active marquee screenshot.
**依賴：** 2.2.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-DESKTOP-MARQUEE`; desktop JSON/log/screenshot.
**完成門檻：** Normal/reverse/Ctrl selection passes and pointer activation targets the reserved SuperExplorer cell.

- [ ] 3.1.1 Rerun normal, reverse, threshold, Ctrl-union and lost-button marquee tests.
- [ ] 3.1.2 Rerun desktop pointer/keyboard/UIA fixed-entry activation with overlapping-position reconciliation.
- [ ] 3.1.3 Capture and inspect an active reverse marquee with at least two selected UIA items after Start closes.

## 4. Verification and Packaging

### 4.1 Complete automated and traceability gates

**目的：** Produce reproducible proof for every requirement and task.
**輸入：** Integrated source, focused/headful outputs and OpenSpec artifacts.
**產出：** Full quality logs, evidence index and strict validation result.
**依賴：** 2.2 and 3.1.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** All change gates; `evidence/evidence-index.json`.
**完成門檻：** Complete workspace commands exit zero, all artifacts hash correctly, every leaf has a unique record and strict validation passes.

- [ ] 4.1.1 Run `cargo fmt --check` and complete locked/offline workspace check and tests.
- [ ] 4.1.2 Run complete locked/offline clippy with warnings denied.
- [ ] 4.1.3 Build release binaries and record product hashes plus headful artifact hashes.
- [ ] 4.1.4 Create the unique task-linked evidence index and pass strict OpenSpec validation.

### 4.2 Refresh installers

**目的：** Package the exclusive owned Start implementation without launching an installer.
**輸入：** Gate-passing release binaries and formal parent submodule admission.
**產出：** Standalone and combined NSIS installers with SHA-256 records.
**依賴：** 4.1.
**Owner／Wave：** Primary agent / wave 5.
**Gate／Evidence：** `G-OWNED-START`, `G-TRACE`; packaging record.
**完成門檻：** Both UTF-8 NSIS builds exit zero and package the admitted SuperDesktop revision.

- [ ] 4.2.1 Build and hash the standalone SuperDesktop installer without launching it.
- [ ] 4.2.2 Build and hash the combined SuperExplorer installer with exact submodule admission.
