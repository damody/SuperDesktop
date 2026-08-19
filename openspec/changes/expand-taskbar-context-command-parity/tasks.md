## 1. Context model and view

- [x] 1.1 Expand the typed command order to Search, Task View, Show desktop, Task Manager, Lock, and Settings with keyboard-wrap tests.
- [x] 1.2 Render current Search text and checked Task View/Lock accessibility state in six 32 DIP rows and content-fit 220 DIP geometry.
- [x] 1.3 Add Traditional Chinese and English label/state tests without self-referential source assertions.

## 2. Runtime behavior

- [x] 2.1 Add a pure one-field settings mutation helper for Search cycling, Task View, and Lock with unrelated-field preservation tests.
- [x] 2.2 Persist context mutations atomically, route Show desktop to the existing owned session, and retain isolated Task Manager/Settings paths.

## 3. UTIT and integration

- [x] 3.1 Record ordered UIA menu names and row/popup physical and logical bounds in the Explorer-free resize case.
- [x] 3.2 Reject missing, reordered, clipped, falsely checked, out-of-monitor, or implausibly sized menu evidence while retaining the Lock action.
- [x] 3.3 Run focused and full shell-parity plus formatting, workspace, Clippy, release, architecture, source-boundary, and strict gates.
- [x] 3.4 Commit compact evidence and implementation, update the parent gitlink, and keep the change unarchived.
