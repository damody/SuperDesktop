# Owned-shell system interactions tasks

## 1. Keyboard and exact window state

### 1.1 Make physical Windows-key chord admission deterministic

**目的：** Recognize Win+D, Win+Shift+S, and Win+Space independently of asynchronous modifier sampling.
**輸入：** Existing low-level hook/reducer and shortcut action queue.
**產出：** Tracked modifier state, repeat fences, and reducer tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-KEYBOARD`; `evidence/1.1/*.json`.
**完成門檻：** Initial, repeat, release, standalone Win, unsupported modifiers, and left/right variants pass.

- [ ] 1.1.1 Add explicit left/right Windows and Shift state to the production hook boundary.
- [ ] 1.1.2 Route chord admission and standalone Start cancellation through the tracked state.
- [ ] 1.1.3 Add reducer/source tests for Win+D, Win+Shift+S, Win+Space, repeats, releases, and recovery sampling.

### 1.2 Restore hidden exact windows on the second Win+D

**目的：** Complete a reversible Show Desktop cycle without exposing minimized windows on the desktop.
**輸入：** `ShowDesktopSession`, live task snapshot, and `MinimizedWindowShelf`.
**產出：** Shelf-merged restore planning and identity-safety tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-SHOW-DESKTOP`; `evidence/1.2/*.json`.
**完成門檻：** First-cycle success set restores on the second cycle while stale/new windows remain unchanged.

- [ ] 1.2.1 Merge minimized-shelf observations into the runtime snapshot only for truthful task modeling.
- [ ] 1.2.2 Preserve exact HWND/process/stable-identity validation through minimize and restore completion.
- [ ] 1.2.3 Add hidden-restore, partial-failure, stale-HWND, new-window, and repeated-cycle tests.

### 1.3 Revalidate the fixed built-in screen-snipping route

**目的：** Ensure the corrected hook still activates only Windows' built-in overlay and cleans its broker.
**輸入：** Existing `OpenScreenSnip` action, fixed URI, broker lifecycle, and focused UTIT.
**產出：** Regression tests and physical admission/cleanup evidence.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SNIPPING`; `evidence/1.3/*`.
**完成門檻：** Overlay opens once, Escape dismisses it, Explorer remains absent afterward, and failure has no fallback.

- [ ] 1.3.1 Verify source boundaries retain the fixed URI, verified broker, and forbidden fallback list.
- [ ] 1.3.2 Extend physical Win+Shift+S evidence to assert tracked-chord admission and broker cleanup.
- [ ] 1.3.3 Run the focused snipping UTIT twice against one candidate hash.

## 2. Smooth system status commands

### 2.1 Implement latest-value-wins volume coordination

**目的：** Keep slider motion local and smooth while bounding native command pressure.
**輸入：** System flyout pointer/keyboard callbacks and status command client.
**產出：** Optimistic value, one-in-flight coalescer, final commit, and tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-VOLUME-COALESCE`; `evidence/2.1/*.json`.
**完成門檻：** Every local pointer value renders, only latest native values dispatch, and release commits exactly once.

- [ ] 2.1.1 Separate displayed/desired volume from the last authoritative provider snapshot.
- [ ] 2.1.2 Add a one-in-flight latest-value coordinator with final-release commit and bounded refresh.
- [ ] 2.1.3 Add drag-storm, keyboard, clamp, mute, failure, and final-value unit/integration tests.

### 2.2 Recover exact input-profile activation

**目的：** Switch profiles by pointer and Win+Space despite provider restarts and delayed TSF/HKL observation.
**輸入：** Status reconciler, exact profile identities, platform activation, and host deadline.
**產出：** One-refresh replay, bounded fresh observation, and tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-INPUT-PROFILE`; `evidence/2.2/*.json`.
**完成門檻：** Fresh exact profiles activate; stale/disappeared identities never redirect; timeout remains non-fatal and truthful.

- [ ] 2.2.1 Trace the stale-generation and observation-timeout paths with structured provider identities.
- [ ] 2.2.2 Implement one generation resynchronization and bounded exact-profile observation recovery.
- [ ] 2.2.3 Add host-restart, delayed-observation, disappeared-profile, pointer, Win+Space, and deadline tests.

## 3. Native notification interactions

### 3.1 Deliver version-correct notification-area callbacks

**目的：** Make visible and overflow tray icons respond to pointer, keyboard, and context interaction like Explorer.
**輸入：** Icon registry identity/version data, callback delivery adapter, and UI action paths.
**產出：** Exact callback mapping, foreground context handling, stale cleanup, and tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-TRAY-CALLBACK`; `evidence/3.1/*.json`.
**完成門檻：** Version-4/legacy left, keyboard, and right-click fixtures receive exact payloads once; stale owners receive none.

- [ ] 3.1.1 Audit and correct modern/legacy callback message, `wParam`/`lParam`, coordinates, and foreground sequencing.
- [ ] 3.1.2 Reconcile stale generation or owner identity before dispatch and remove invalid registrations.
- [ ] 3.1.3 Add fixture-window tests for visible/overflow pointer, Enter/Space, right-click menu, and reused HWND.

### 3.2 Recover Windows notification event and mutation flow

**目的：** Keep notification add/remove/clear state authoritative without coupling WinRT callbacks to GPUI borrows.
**輸入：** Long-lived WinRT source, notification host cadence, client reconciler, and center actions.
**產出：** Data-only dirty callbacks, coalesced refresh, confirmed mutation, and tests.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-WINDOWS-NOTIFICATIONS`; `evidence/3.2/*.json`.
**完成門檻：** Added/removed/storm/recovery events reconcile; exact dismiss/clear confirm; access failure preserves prior state and tray function.

- [ ] 3.2.1 Verify WinRT apartment/listener/subscription lifetime and make callback state panic-contained and data-only.
- [ ] 3.2.2 Coalesce dirty refreshes and require authoritative absence after exact dismiss and clear.
- [ ] 3.2.3 Add added/removed/storm/malformed/access-denied/dismiss/clear/recovery host and app tests.

## 4. Physical GUI integration

### 4.1 Add focused interaction UTIT cases

**目的：** Prove production input and native callbacks rather than only reducer behavior.
**輸入：** Release candidate, controlled windows/icons/profiles/toasts, UTIT runner, and watchdog.
**產出：** Focused scripts, catalog entries, privacy-safe JSON/JUnit/Markdown reports.
**依賴：** 1.2, 1.3, 2.1, 2.2, 3.1, 3.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PHYSICAL-UTIT`; `evidence/4.1/*`.
**完成門檻：** Six requested interaction paths pass against the same candidate hash with Explorer absent where required.

- [ ] 4.1.1 Add/extend Win+D two-cycle and continuous volume-drag physical cases.
- [ ] 4.1.2 Add/extend tray left/right and pointer/Win+Space input-profile physical cases.
- [ ] 4.1.3 Add/extend Win+Shift+S and controlled Windows notification add/remove/clear cases with privacy redaction.

### 4.2 Run focused and regression GUI evidence

**目的：** Detect lifecycle, focus, geometry, crash, and Explorer-revival regressions.
**輸入：** Completed focused cases and one release candidate hash.
**產出：** Two clean focused runs plus shell-parity regression reports.
**依賴：** 4.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-PHYSICAL-UTIT`, `G-NO-CRASH`, `G-EXPLORER-ABSENT`; `evidence/4.2/*`.
**完成門檻：** Reports contain no panic/borrow/stale-generation signature, Explorer absence is restored, and all expected native observations pass.

- [ ] 4.2.1 Execute each focused case twice from clean state and bind reports to the candidate hash.
- [ ] 4.2.2 Execute the existing shell-parity regression set and inspect screenshots/UIA where applicable.
- [ ] 4.2.3 Scan stdout/stderr/process evidence for panics, provider leakage, Explorer revival, and privacy-sensitive data.

## 5. Release admission and traceability

### 5.1 Pass automated quality gates

**目的：** Admit source only after every affected crate and workspace contract passes.
**輸入：** Integrated implementation and focused tests.
**產出：** Format, tests, Clippy, release, architecture, and strict OpenSpec reports.
**依賴：** 4.2.
**Owner／Wave：** Primary integrator / wave 5.
**Gate／Evidence：** `G-AUTOMATED`, `G-OPENSPEC`; `evidence/5.1/*`.
**完成門檻：** All required commands exit zero without warnings-as-errors or placeholder/traceability failures.

- [ ] 5.1.1 Run format checks plus focused and locked/offline workspace tests.
- [ ] 5.1.2 Run Clippy warnings-as-errors, architecture/source-boundary checks, and release build.
- [ ] 5.1.3 Run strict OpenSpec, detailed task validation, placeholder scan, and proposal-to-evidence traceability review.

### 5.2 Package, hash, and integrate the candidate

**目的：** Produce a reproducible installed result and integrate nested/parent revisions.
**輸入：** Passing candidate and clean tracked worktrees.
**產出：** Nested commit, parent submodule commit, installer, installed hashes, and evidence index.
**依賴：** 5.1.
**Owner／Wave：** Primary integrator / wave 6.
**Gate／Evidence：** `G-PACKAGE`, `G-TRACE`; `evidence/5.2/*`.
**完成門檻：** Installer succeeds, installed binaries match admitted package artifacts, all 33 leaves have unique evidence, and the change remains unarchived.

- [ ] 5.2.1 Commit the nested implementation and update the parent submodule pointer without touching unrelated untracked files.
- [ ] 5.2.2 Build/install the package without launch races and compare packaged/installed executable hashes.
- [ ] 5.2.3 Write the 33-leaf evidence index, commit final evidence, and re-run clean status plus strict validation.
