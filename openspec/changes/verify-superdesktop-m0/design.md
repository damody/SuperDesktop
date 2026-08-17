## Context

核准的 C-level adjustment `C-W11-REFERENCE-001` 將 release platform 固定為 exact Windows 11＋ExplorerPatcher reference profile，並將 Windows 10 compatibility 標為 not-claimed。

本 change 是 SuperDesktop M0 program 的最後一道發行驗證，不新增產品功能。所有 production child changes 必須先完成，才能在凍結的 Windows 11＋ExplorerPatcher reference environment 與螢幕拓撲矩陣上執行可重現的 gate；封存仍由使用者另行決定。

主要 UI／互動與 release lifecycle 環境為 Windows 11 build 26200.8875、ExplorerPatcher 26100.8457.70.3，以及 SHA-256 `48B5F990B9E155C5C2719D8F8B41D88ED4420A46C3B6018278511F9C349B387E` 的工作列截圖。Windows 10 compatibility 不在本 release claim 內。

目前機器只有一個實體顯示器，因此自動化 mixed-DPI topology 以虛擬顯示器執行；真實 mixed-DPI 雙螢幕保留為 release-candidate confirmation。外部環境缺失只能標記 `blocked`，不得改成 `not-applicable` 或假造通過。

## Goals / Non-Goals

**Goals:**

- 凍結並驗證 ExplorerPatcher reference profile 的 OS、版本、設定與影像雜湊。
- 對所有 blocking gates 建立可重現、可追溯、可獨立複核的證據。
- 驗證 exact reference profile 的啟動、操作、正常回復、強制崩潰回復與 installer reboot/rollback。
- 驗證 100%、125%、150%、175%、200% DPI 與虛擬 mixed-DPI topology。
- 分離可自動完成的虛擬 topology gate 與需實體設備的 release-candidate confirmation。
- 驗證鍵盤、UIA/AccessKit、高對比、繁中、英文、簡中字形 fallback、RTL/bidi 與 IME。
- 驗證離線重建、壓力、效能、安全、授權、來源邊界與完整追溯。

**Non-Goals:**

- 不在此 change 修補 production 功能；失敗回到原 owner 的 child change 修正。
- 不將 Windows 11 原生工作列當成 UI 基準；基準是已凍結的 ExplorerPatcher profile。
- 不因缺少實體雙螢幕、mutation-bearing reboot run 或 independent reviewer 而降低門檻。
- 不承諾完整簡中字串翻譯；M0 僅要求字形、fallback、截斷與版面安全。

## Decisions

### 1. 參考環境採不可變 profile

測試前把 OS build、ExplorerPatcher binary/version、影響 UI 的設定匯出、參考影像與其 SHA-256 寫入 evidence。任一欄位不符即停止視覺比較並建立 B/C correction disposition，不能更新 baseline 來掩蓋差異。

### 2. 視覺比較以幾何與狀態契約為主

比較工作列高度、列數、按鈕排列、active/attention 狀態、通知區、時鐘、Start 入口、SuperExplorer 固定入口及桌面圖示網格。比較前凍結 baseline contract：100% DPI 幾何 anchor、taskbar/row height 與 hit target 容許 ±2 physical px，其他 DPI 以 scale 四捨五入；動態時間、日期、通知數與 fixture window title 區域使用固定矩形遮罩；遮罩外全圖 SSIM 必須 ≥0.95，且控制 identity/state assertions 必須精確相符。互動與 accessibility assertions 不可被影像容差取代。`B-W6-REFERENCE-MASK-001` 依 immutable reference 的實際像素邊界，將誤記為 x=150..2650 的動態 task fixture 修正為 x=96..3400；reference image、SSIM threshold、幾何容差與 exact-state assertions 均不變，舊 contract record 保留為 stale 並由 replacement record 接續。

### 3. Exact reference profile 是 lifecycle 與 installer gate

Windows 11 build 26200.8875＋ExplorerPatcher 26100.8457.70.3 必須驗證 preview 啟動、Shell mode 明確 opt-in、核心互動、正常退出回復、forced-crash guardian 回復與 installer reboot/rollback。任一 profile/hash/candidate 漂移或必要 mutation phase 未執行時，相關 leaf 保持 `blocked`，M0 不得宣告完成。

### 4. 虛擬與實體 mixed-DPI 分開判定

虛擬顯示器覆蓋自動化 monitor add/remove、primary change、DPI change、work-area 與 hot-plug，是 mandatory gate。實體 mixed-DPI 雙螢幕覆蓋驅動、實際座標與肉眼確認，是獨立 mandatory release-candidate confirmation；缺硬體不阻塞較早的 production implementation，但會阻止最終發行結案。

### 5. 效能門檻固定且保留原始樣本

參考環境的冷啟動不超過 2 秒、idle CPU median 小於 0.5%、shell event-to-visible p95 小於 100 ms、M0 working set 小於 150 MiB。每項保存工具版本、暖機方式、樣本數、原始 timestamps/counters 與統計結果，不接受只有彙總圖。

### 6. 證據與 reviewer 職責分離

每個 mandatory leaf 在所屬 change 的 append-only index 對應全域唯一 `<change-name>/<L3-id>` `task_id` 與 immutable evidence record。Independent reviewer 只驗證 coverage、結果與 P0/P1 disposition；修正與重跑由 Primary integrator 或原 gate owner 負責。`superseded` 只有 replacement 已存在、無循環、mandatory、覆蓋相同 requirement/scenario/gate 且已有有效 passed evidence 時才成立。

## Risks / Trade-offs

- **ExplorerPatcher 更新造成基準漂移**：先比對凍結 profile；差異走 B/C correction，不直接更新 baseline。
- **虛擬顯示器與實體驅動行為不同**：保留實體 mixed-DPI release confirmation，兩者不可互相替代。
- **Reference profile 漂移或 mutation phase 未執行**：允許 leaf 保持 blocked，但不允許降低或略過 gate。
- **效能量測受背景程序干擾**：記錄環境、暖機與原始樣本，異常 run 必須有可稽核 disposition。
- **跨 child change 修正破壞已封存證據**：任何修正都建立新 lineage、重跑受影響 gate，舊證據標 stale 而不覆寫。

## Migration Plan

1. 驗證所有 production child changes 已完成、封存且 contract hashes 一致。
2. 凍結 ExplorerPatcher reference profile 與測試工具鏈。
3. 執行 build/offline、reference、DPI、虛擬 topology、a11y/i18n、stress、performance、安全與 traceability gates。
4. 在 exact Windows 11＋ExplorerPatcher profile 執行 lifecycle 與 installer reboot/rollback 矩陣。
5. 在真實 mixed-DPI 雙螢幕執行 release-candidate confirmation。
6. 由 independent reviewer 複核；P0/P1 交還原 owner 修正並重跑受影響 gate。
7. 所有 blocking gates 通過後封存本 change 與 program change。

Rollback 不變更使用者 Shell 設定；驗證期間若 Shell mode 失敗，沿用 guardian 回復 Explorer 與 work area，並保留失敗證據。

## Open Questions

無。缺少 mutation-bearing reboot/rollback、實體雙螢幕或 independent reviewer 是已知執行條件，不是規格待決事項。
