# Built-in screen snip clipboard completion tasks

## 1. Clipboard completion contract

### 1.1 Observe clipboard image publication without opening the clipboard

**目的：** Define a truthful, non-invasive completion fence for the built-in screenshot.
**輸入：** Clipboard sequence API, image format availability API, and current overlay lifecycle.
**產出：** Clipboard observation model, native adapter, and unit tests.
**依賴：** None.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-CLIPBOARD-COMPLETION`; `evidence/1.1/*`.
**完成門檻：** Changed materialized image, advertised-but-empty image, unchanged state, and unrelated non-image updates are classified deterministically without locking or reading image pixels.

- [x] 1.1.1 Add sequence, bitmap/DIB/DIBV5 availability, and non-empty data-handle observation through Windows APIs.
- [x] 1.1.2 Implement a pure completion classifier requiring changed sequence and a materialized image payload.
- [x] 1.1.3 Add unchanged, bitmap, DIB, DIBV5, advertised-empty, and unrelated non-image classifier tests.

### 1.2 Distinguish capture intent, cancellation, and timeout

**目的：** Avoid both premature success and a long delay for explicit cancellation.
**輸入：** Overlay visibility polling, left-button activity, Escape activity, and clipboard observations.
**產出：** Captured/cancelled/error terminals and transition tests.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / wave 1.
**Gate／Evidence：** `G-SNIP-TERMINAL`; `evidence/1.2/*`.
**完成門檻：** Capture waits up to ten seconds for image publication, Escape cancels after a short grace, and ambiguous dismissal never becomes false success.

- [x] 1.2.1 Track selection intent and Escape while the built-in overlay remains visible.
- [x] 1.2.2 Return distinct captured and cancelled outcomes and a bounded clipboard-publication error.
- [x] 1.2.3 Add capture, explicit-cancel, ambiguous-dismissal, unrelated-update, and timeout transition tests.

## 2. Broker lifecycle and app integration

### 2.1 Move temporary Explorer cleanup after clipboard completion

**目的：** Preserve the Windows protocol broker through Snipping Tool post-capture work.
**輸入：** Existing trusted Explorer observation/recovery/cleanup and the terminal classifier.
**產出：** Reordered cleanup with combined primary/cleanup errors and ownership tests.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-BROKER-LIFETIME`; `evidence/2.1/*`.
**完成門檻：** Temporary broker survives until terminal and is always removed afterward; pre-existing Explorer is untouched.

- [x] 2.1.1 Retain the temporary broker through captured, cancelled, and error terminal determination.
- [x] 2.1.2 Preserve cleanup on activation, overlay, clipboard, and timeout failure paths without masking the primary error.
- [x] 2.1.3 Add source and lifecycle tests for temporary versus pre-existing Explorer ownership.

### 2.2 Publish exact app trace and console outcomes

**目的：** Make admission, captured, cancelled, and failure states independently observable.
**輸入：** Screen-snip worker result, existing trace function, and console error routing.
**產出：** Outcome-specific traces and app tests.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / wave 2.
**Gate／Evidence：** `G-SNIP-OBSERVABILITY`; `evidence/2.2/*`.
**完成門檻：** Captured trace appears only after image publication, cancelled trace only after cancellation, and failures remain console-visible.

- [x] 2.2.1 Add captured and cancelled result handling without treating admission as capture completion.
- [x] 2.2.2 Keep clipboard-publication and broker-cleanup failures routed to the console.
- [x] 2.2.3 Add source tests forbidding custom capture, clipboard writes, and success-before-publication.

## 3. Physical verification and release

### 3.1 Upgrade Explorer-free physical screen-snipping evidence

**目的：** Prove a real built-in selection reaches the clipboard and cancellation remains clean.
**輸入：** Release candidate, physical pointer injection, clipboard metadata probe, package identity, watchdog, and Explorer suppression.
**產出：** Two capture reports, one cancellation report, and privacy scan.
**依賴：** 2.2.
**Owner／Wave：** Primary integrator / wave 3.
**Gate／Evidence：** `G-PHYSICAL-SNIP`, `G-PRIVACY`, `G-NO-CRASH`; `evidence/3.1/*`.
**完成門檻：** Two real selections publish non-zero images and one Escape run preserves sequence; all clean brokers and persist no pixels.

- [x] 3.1.1 Extend the focused script to drag a deterministic rectangle and inspect only clipboard metadata/dimensions.
- [x] 3.1.2 Run the Explorer-free capture twice and cancellation once against one candidate hash.
- [x] 3.1.3 Scan reports/logs/artifacts for errors, Explorer leakage, image persistence, user data, and hash drift.

### 3.2 Pass automated, packaging, and traceability gates

**目的：** Deliver the verified app/platform change through the normal installer.
**輸入：** Passing physical candidate, complete tests, OpenSpec artifacts, and parent installer.
**產出：** Automated reports, release/installer hashes, commits, and 18-record evidence index.
**依賴：** 3.1.
**Owner／Wave：** Primary integrator / wave 4.
**Gate／Evidence：** `G-AUTOMATED`, `G-OPENSPEC`, `G-PACKAGE`; `evidence/3.2/*`.
**完成門檻：** Format/tests/Clippy/release/strict validation pass, installer hashes match, both tracked worktrees are clean, and the change remains unarchived.

- [x] 3.2.1 Run formatting, focused tests, and locked/offline workspace tests plus Clippy warnings-as-errors.
- [x] 3.2.2 Run release build, installer without launch, and packaged app hash comparison.
- [x] 3.2.3 Write 18 unique evidence records, commit nested/parent integration, and rerun strict/status checks.
