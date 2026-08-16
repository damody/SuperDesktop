# Add SuperDesktop Start and Search

## 1. Search contracts and providers

### 1.1 Extend search protocol and ranking

**目的：** Define generation-bound, cancellable search requests and deterministic batches.
**輸入：** Shared search DTOs and provider lifecycle contracts.
**產出：** Search request/batch/status messages, normalization and ranking functions.
**依賴：** `extend-superdesktop-shell-contracts`.
**Owner／Wave：** Primary integrator / Wave 4.
**Gate／Evidence：** `G-START-SEARCH`, `G-PERF`; protocol/ranking tests.
**完成門檻：** Stale generations are ignored and ranking is deterministic across locales.

- [x] 1.1.1 Add search query, batch, provider-state, cancellation, and activation contracts.
- [x] 1.1.2 Add normalized prefix/word/substring/recency ranking with stable tie-breaks.
- [x] 1.1.3 Add stale-generation, Unicode, empty-query, and result-limit tests.

### 1.2 Add bounded local providers

**目的：** Supply useful app, settings, and file results without relying on Explorer UI.
**輸入：** Admitted Start Menu/file roots, curated settings catalog, deadlines and limits.
**產出：** Application discovery, settings catalog, bounded file traversal and typed activation.
**依賴：** 1.1.
**Owner／Wave：** Primary integrator / Wave 4.
**Gate／Evidence：** `G-START-SEARCH`, `G-SAFETY`, `G-PERF`; fixture tests.
**完成門檻：** Providers enforce roots/depth/count/deadline and isolate individual item failures.

- [x] 1.2.1 Add Start Menu application discovery and stable app identity.
- [x] 1.2.2 Add capability-tagged Windows settings catalog.
- [x] 1.2.3 Add bounded file search with cancellation and deadline checks.
- [x] 1.2.4 Add provider fixture and performance-budget tests.

## 2. Owned Start surface

### 2.1 Implement Start model and interactions

**目的：** Replace shell-mode Start deferral with a GPUI-owned state model.
**輸入：** Search contracts/providers, taskbar Start action, settings store.
**產出：** Start open/close/focus state, pins/recent/all-apps, results and typed activation.
**依賴：** 1.2.
**Owner／Wave：** Primary integrator / Wave 4.
**Gate／Evidence：** `G-START-SEARCH`, `G-A11Y-I18N`; model tests.
**完成門檻：** Pointer/keyboard/accessibility routes match, stale results do not render, and pins persist.

- [x] 2.1.1 Add owned Start sections, open/close/focus, pins, recent, and power/settings actions.
- [x] 2.1.2 Add IME composition/commit, debounce, query cancellation, result merge and navigation.
- [x] 2.1.3 Route shell-mode Start to the owned model while retaining preview probe behavior.

### 2.2 Verify integration and budgets

**目的：** Prove functional, accessibility, and performance behavior.
**輸入：** 2.1 model, providers, deterministic clocks and fixtures.
**產出：** Contract/performance evidence and completed change.
**依賴：** 2.1.
**Owner／Wave：** Primary integrator / Wave 4 exit.
**Gate／Evidence：** `G-START-SEARCH`, `G-PERF`, `G-TRACE`; cargo/OpenSpec logs.
**完成門檻：** First-frame/app/all-provider budgets and all validation pass.

- [x] 2.2.1 Add end-to-end query/merge/cancel/activate/accessibility tests and timing evidence.
- [x] 2.2.2 Run fmt, offline check, clippy, tests, strict OpenSpec, and task validation.
