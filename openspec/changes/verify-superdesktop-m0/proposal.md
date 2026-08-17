## Why

Production changes 完成不等於 M0 可發行；必須將凍結 Windows 11＋ExplorerPatcher reference lifecycle、虛擬/真實多螢幕、效能、安全、協助工具與追溯分成可判定 gate。

## What Changes

- 驗證 ExplorerPatcher reference profile 與 reference image hash，防止 baseline 漂移。
- 完成 DPI、虛擬多螢幕、exact reference-profile lifecycle/installer 與真實 mixed-DPI release confirmation。
- 完成 a11y/i18n/bidi、效能、stress、offline、安全、授權、traceability 與獨立 review。

## Capabilities

### New Capabilities

- `shell-foundation-verification`：M0 跨平台/硬體、視覺、協助工具、效能、安全、證據與最終發行 gate。

### Modified Capabilities

無。

## Impact

依賴所有 production child changes；目前機器可完成 reference 與虛擬 topology，大部分 gate 可直接執行，mutation-bearing reboot/rollback、真實雙螢幕 confirmation 與 independent review 仍不得假完成。Windows 10 compatibility 不在本 release claim 內。
