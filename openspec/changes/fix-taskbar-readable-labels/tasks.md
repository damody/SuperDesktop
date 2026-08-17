## 1. Settings Defaults

- [ ] 1.1 Change new taskbar settings to default `show_labels=true`.
- [ ] 1.2 Change missing-field decoding to default `show_labels=true` while preserving explicit false.
- [ ] 1.3 Add default, partial-document, and explicit-false round-trip tests.

## 2. Readable Label Policy

- [ ] 2.1 Add a pure task display-label policy with group-count formatting and real-icon awareness.
- [ ] 2.2 Remove the first-character pseudo-icon fallback for tasks without real icons.
- [ ] 2.3 Add English, Traditional Chinese, grouped, empty-name, and legacy-label-disabled policy tests.

## 3. Bounded Rendering

- [ ] 3.1 Render the task label in a flex-one, minimum-width-zero child.
- [ ] 3.2 Apply single-line hidden-overflow ellipsis styling without changing task actions or accessibility names.
- [ ] 3.3 Add a source/render contract that rejects reintroduction of first-character fallback or missing ellipsis containment.

## 4. Verification

- [ ] 4.1 Run formatting and targeted settings-store/taskbar-ui tests.
- [ ] 4.2 Run workspace locked offline check and relevant release tests.
- [ ] 4.3 Capture and inspect a headful taskbar at the active reference DPI.
- [ ] 4.4 Record `G-TASKBAR-LABELS` evidence and pass strict OpenSpec validation.
