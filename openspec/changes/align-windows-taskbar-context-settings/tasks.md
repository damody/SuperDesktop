# Align Windows Taskbar Context Settings

## 1. Contracts and Settings

### 1.1 Extend bounded taskbar settings

**目的：** Persist Search, Task View and alignment without changing valid existing preferences.
**輸入：** Approved design, `settings-store` schema/version behavior.
**產出：** Enums, fields, validation, encode/decode and migration tests.
**依賴：** None.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-PERSISTENCE`; protocol test record.
**完成門檻：** Missing, valid, invalid and round-trip cases preserve every independent field deterministically.

- [ ] 1.1.1 Add bounded Search visibility and taskbar alignment enums.
- [ ] 1.1.2 Add Search, Task View and alignment fields with stable defaults.
- [ ] 1.1.3 Decode and encode additive fields while isolating invalid values.
- [ ] 1.1.4 Add migration, invalid-field and round-trip tests.

### 1.2 Add pure context and settings effects

**目的：** Define UI behavior without process, filesystem or registry authority.
**輸入：** 1.1 settings contracts and existing command descriptors.
**產出：** Pure menu/settings models, effects and deterministic tests.
**依賴：** 1.1.
**Owner／Wave：** Primary agent / wave 1.
**Gate／Evidence：** `G-TASKBAR-CONTEXT`, `G-TASKBAR-SETTINGS`; model tests.
**完成門檻：** Pointer, keyboard, disabled, stale and dismissal scenarios emit one typed effect or no effect.

- [ ] 1.2.1 Add empty-taskbar context command and selection model.
- [ ] 1.2.2 Add grouped settings rows, expansion state and candidate effects.
- [ ] 1.2.3 Model unsupported rows as disabled with bounded explanations.
- [ ] 1.2.4 Add navigation, activation, dismissal and unavailable tests.

## 2. Windows 11 UI Surfaces

### 2.1 Render the empty-taskbar context menu

**目的：** Provide the owned compact Windows 11 taskbar menu.
**輸入：** 1.2 context model and current theme/accessibility conventions.
**產出：** `TaskbarContextView`, geometry helpers and render tests.
**依賴：** 1.2.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-CONTEXT`, `G-TASKBAR-A11Y`; source/render record.
**完成門檻：** Task Manager/settings rows, focus, theme, DPI and dismissal match the normative scenarios.

- [ ] 2.1.1 Implement 220px Windows 11 menu geometry, palette and shadow surface.
- [ ] 2.1.2 Implement pointer, keyboard, UIA and focus styling for both commands.
- [ ] 2.1.3 Add Escape, successful-command and focus-loss dismissal behavior.
- [ ] 2.1.4 Add light, dark, high-contrast and 100–500% geometry tests.

### 2.2 Restyle application Jump Lists

**目的：** Align application right-click commands without changing command authority.
**輸入：** Existing `JumpListModel`, validated provider/local commands and 2.1 tokens.
**產出：** Windows 11 Jump List rendering and command-state tests.
**依賴：** 2.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-CONTEXT`, `G-TASKBAR-A11Y`; Jump List render record.
**完成門檻：** Provider, pin and close commands retain behavior while geometry and states use shared menu tokens.

- [ ] 2.2.1 Extract shared command-surface visual tokens.
- [ ] 2.2.2 Apply row spacing, icons, separators and hover/focus states to Jump Lists.
- [ ] 2.2.3 Render disabled and destructive commands without changing typed effects.
- [ ] 2.2.4 Add provider-failure, pin, close and source-contract tests.

### 2.3 Render the owned taskbar settings window

**目的：** Mirror the Windows 11 Personalization > Taskbar information hierarchy.
**輸入：** 1.2 settings model and shared Windows 11 visual tokens.
**產出：** `TaskbarSettingsView`, reusable rows/cards and accessibility tests.
**依賴：** 1.2, 2.1.
**Owner／Wave：** Primary agent / wave 2.
**Gate／Evidence：** `G-TASKBAR-SETTINGS`, `G-TASKBAR-A11Y`; settings render record.
**完成門檻：** Every supported and unsupported row is truthful, keyboard accessible and layout-safe.

- [ ] 2.3.1 Implement breadcrumb, information banner and expandable card hierarchy.
- [ ] 2.3.2 Implement switch, dropdown, supporting-text and related-settings rows.
- [ ] 2.3.3 Bind supported rows and disabled explanations to the pure model.
- [ ] 2.3.4 Add long-label, keyboard, UIA, theme and scale tests.

## 3. Product Composition and Live Behavior

### 3.1 Own context/settings window lifecycle

**目的：** Open, position and dismiss singleton surfaces without Explorer.
**輸入：** Wave 2 views, taskbar monitor geometry and existing popup patterns.
**產出：** Composition callbacks, window slots, focus restoration and lifecycle tests.
**依賴：** 2.1, 2.3.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-TASKBAR-CONTEXT`, `G-SHELL-NONINTERFERENCE`; composition record.
**完成門檻：** Exactly one relevant surface exists, stays monitor-clamped and tears down in Preview and Shell modes.

- [ ] 3.1.1 Add background context callback and stop task-context propagation.
- [ ] 3.1.2 Add monitor-clamped context and settings window options.
- [ ] 3.1.3 Enforce singleton replacement, Escape/focus dismissal and focus return.
- [ ] 3.1.4 Add monitor retirement, repeated-open, Preview and Shell lifecycle tests.

### 3.2 Execute validated commands and atomic settings saves

**目的：** Connect typed effects to safe external behavior and saved revisions.
**輸入：** 1.1 contracts, 1.2 effects, settings store and trusted Windows paths.
**產出：** Task Manager route, atomic save/reconcile path and failure tests.
**依賴：** 3.1.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-TASKBAR-PERSISTENCE`, `G-SHELL-NONINTERFERENCE`; mutation record.
**完成門檻：** Only validated actions execute; failed/stale saves preserve all authoritative state.

- [ ] 3.2.1 Resolve and launch Task Manager without shell expansion or substitution.
- [ ] 3.2.2 Validate complete taskbar candidates before persistence.
- [ ] 3.2.3 Save atomically and distribute only returned saved revisions.
- [ ] 3.2.4 Add launch rejection, save failure, stale revision and independence tests.

### 3.3 Apply Search, Task View and alignment live

**目的：** Make saved settings visibly and interactively authoritative.
**輸入：** Saved taskbar settings and existing taskbar layout/start/task-view callbacks.
**產出：** Search control modes, Task View visibility, alignment layout and integration tests.
**依賴：** 3.2.
**Owner／Wave：** Primary agent / wave 3.
**Gate／Evidence：** `G-TASKBAR-SETTINGS`, `G-TASKBAR-A11Y`; live behavior record.
**完成門檻：** All monitors update without overlap, stale hit targets or status-region movement.

- [ ] 3.3.1 Render hidden/icon/box Search modes and route visible modes to owned search.
- [ ] 3.3.2 Remove disabled Task View from render, focus, hit testing and UIA.
- [ ] 3.3.3 Left- or center-align the bounded task cluster across rows and overflow.
- [ ] 3.3.4 Reconcile labels, grouping, previews, monitors and rows through the same save path.
- [ ] 3.3.5 Add multi-row, constrained-width, multi-monitor and live-update tests.

## 4. Verification and Packaging

### 4.1 Run Windows 11 parity and safety gates

**目的：** Prove the owned surfaces are accessible, truthful and Explorer-independent.
**輸入：** Integrated release product and committed settings/context fixtures.
**產出：** Screenshots, UIA traces, command/persistence reports and evidence index.
**依賴：** 3.3.
**Owner／Wave：** Primary agent / wave 4.
**Gate／Evidence：** All change gates; `evidence/evidence-index.json`.
**完成門檻：** Every leaf has task-linked passing evidence and strict validation passes on Windows 11 build 26200.

- [ ] 4.1.1 Run fmt, locked/offline workspace tests and clippy warnings-as-errors.
- [ ] 4.1.2 Capture empty-taskbar and application context menus in light and dark themes.
- [ ] 4.1.3 Capture settings cards, supported controls and disabled explanations.
- [ ] 4.1.4 Capture high-contrast, reduced-motion, Traditional Chinese and UIA evidence.
- [ ] 4.1.5 Verify Task Manager admission, save failure and stale revision traces.
- [ ] 4.1.6 Verify Preview non-interference and controlled Explorer-free behavior.
- [ ] 4.1.7 Create unique task-linked evidence and pass strict OpenSpec validation.

### 4.2 Build standalone and combined installers

**目的：** Ship the modified product without launch or residue.
**輸入：** Gate-passing release revision and existing NSIS manifests.
**產出：** Hashed standalone/combined installers and cleanup record.
**依賴：** 4.1.
**Owner／Wave：** Primary agent / wave 5.
**Gate／Evidence：** `G-PACKAGE`; `evidence/packaging-record.json`.
**完成門檻：** Both installers contain updated binaries, build without launch, and uninstall declares no feature-specific residue.

- [ ] 4.2.1 Build the release workspace and hash modified binaries.
- [ ] 4.2.2 Build and hash the standalone SuperDesktop installer without launching it.
- [ ] 4.2.3 Build and hash the combined SuperExplorer installer without launching it.
- [ ] 4.2.4 Verify install/uninstall manifests and record final package evidence.


