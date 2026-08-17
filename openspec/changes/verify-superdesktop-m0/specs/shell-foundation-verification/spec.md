## ADDED Requirements

### Requirement: 凍結 ExplorerPatcher 參考環境
驗證流程 SHALL 在任何視覺比較或 lifecycle effect 前記錄並驗證 Windows build/UBR、ExplorerPatcher 版本與 binary hash、影響工作列的設定匯出、參考影像及其 SHA-256；M0 UI／互動與 release lifecycle 基準 SHALL 為 Windows 11 build 26200.8875 與 ExplorerPatcher 26100.8457.70.3。Windows 10 compatibility SHALL 標為 not-claimed。

#### Scenario: Reference profile 完整且相符
- **WHEN** 驗證者載入凍結的 reference profile
- **THEN** 所有 identity、設定與影像雜湊皆須相符，才可開始視覺 gate

#### Scenario: Reference profile 漂移
- **WHEN** 任一 OS、ExplorerPatcher、設定或影像 identity 不符
- **THEN** 視覺 gate MUST 停止並建立 B/C correction disposition，不得靜默更新 baseline

### Requirement: 視覺容差必須在候選結果前凍結
驗證流程 SHALL 在擷取候選畫面前雜湊 immutable baseline contract：100% DPI 幾何、row/taskbar height 與 hit target 容許 ±2 physical px，其他 DPI 依 scale 四捨五入；只遮罩預先列出的時間、日期、通知數與 fixture window title 矩形；`B-W6-REFERENCE-MASK-001` 所校正的 reference task fixture 矩形為 x=96、y=0、width=3304、height=140，status 動態矩形為 x=3400、y=0、width=437、height=140；遮罩外 SSIM SHALL ≥0.95，control identity/state SHALL 精確相符。

#### Scenario: 事後修改容差或遮罩
- **WHEN** 候選 capture 產生後 tolerance、mask 或 comparison algorithm 發生變更
- **THEN** 既有 visual evidence MUST 標 stale 並在新 contract hash 下全部重跑

### Requirement: 建置與離線可重現性
驗證流程 SHALL 執行 format、workspace check、clippy warnings-as-errors、workspace tests、release build 與網路停用的 isolated `CARGO_HOME` `--locked --offline` build，並保存 dependency source hash、命令、exit status 與 binary hash。

#### Scenario: 離線來源不完整
- **WHEN** isolated offline build 缺少 dependency source 或 hash 不一致
- **THEN** `G-ARCH` MUST 失敗，即使一般 `--locked` build 成功

### Requirement: DPI 與虛擬顯示器矩陣
驗證流程 SHALL 覆蓋 100%、125%、150%、175%、200% DPI，以及虛擬 mixed-DPI 的 monitor add/remove、primary change、DPI change、hot-plug、taskbar ownership、work area 與 desktop layout reconciliation。

#### Scenario: 任一 DPI 或 topology 子檢查缺失
- **WHEN** evidence index 缺少任一指定 DPI 或虛擬 topology 子檢查
- **THEN** `G-DPI-MONITOR` MUST 失敗，且不得標記 not-applicable

### Requirement: 實體 mixed-DPI release confirmation
發行候選 SHALL 在兩個實體顯示器與不同 DPI scale 上確認 taskbar、work area、desktop layout、pointer/keyboard interaction、primary change 與 hot-plug；此確認為 mandatory release gate，不能由虛擬顯示器結果取代。

#### Scenario: 實體顯示器環境尚未提供
- **WHEN** 無法取得符合條件的實體 mixed-DPI 雙螢幕
- **THEN** confirmation leaf SHALL 保持 blocked，較早 production work 可繼續，但 M0 MUST NOT 宣告可發行

### Requirement: Windows 11 ExplorerPatcher reference-profile lifecycle
驗證流程 SHALL 在 exact Windows 11 build 26200.8875＋ExplorerPatcher 26100.8457.70.3 profile 驗證 preview 啟動、Shell mode 明確 opt-in、桌面與工作列操作、SuperExplorer 啟動、正常退出回復、forced-crash guardian 回復 Explorer/work area，以及 installer reboot/rollback。

#### Scenario: Reference profile 漂移或 mutation sequence 尚未完成
- **WHEN** exact profile admission 失敗或 installer reboot/rollback sequence 未完整執行
- **THEN** reference-profile leaf SHALL 保持 blocked 且 M0 MUST NOT 宣告可發行

#### Scenario: Reference-profile 強制崩潰回復
- **WHEN** SuperDesktop 在 Shell mode 被強制終止
- **THEN** guardian SHALL 在 lifecycle contract 的 deadline 內恢復可操作 Explorer 與正確 work area

### Requirement: 協助工具與國際化矩陣
驗證流程 SHALL 覆蓋 keyboard-only、focus order、UIA/AccessKit identity/role/name/state/action、高對比、繁中、英文、簡中字形與 fallback、RTL/bidi layout、文字截斷，以及 IME 組字期間的輸入與焦點穩定性。

#### Scenario: 可操作控制缺少 accessible contract
- **WHEN** UIA/AccessKit 掃描發現可操作控制缺少 name、role、state 或 action
- **THEN** `G-A11Y-I18N` MUST 失敗

#### Scenario: 簡中字形或 bidi 破版
- **WHEN** zh-CN fallback 或 RTL/bidi fixture 出現缺字、重疊、不可讀截斷或互動順序錯誤
- **THEN** `G-A11Y-I18N` MUST 失敗並保存 locale、字串、DPI 與 geometry evidence

### Requirement: 壓力與資源穩定性
驗證流程 SHALL 分別驗證 watcher overflow、window-event storm、monitor hot-plug、bridge cancellation/timeout、guardian crash loop，以及長時間 soak 後的 working set、thread、handle、GDI object、USER object 與 cache bounds。

#### Scenario: 資源在 soak 後無界成長
- **WHEN** 任一指定資源超過既定 bound 或未回到穩定區間
- **THEN** 對應 safety/performance gate MUST 失敗，且每種資源保留獨立 subcheck

### Requirement: 效能門檻
在凍結參考環境上，SuperDesktop SHALL 達到冷啟動不超過 2 秒、idle CPU median 小於 0.5%、shell event-to-visible p95 小於 100 ms，以及 M0 working set 小於 150 MiB；每項 SHALL 保存工具版本、暖機方式、樣本數、原始 timestamps/counters 與統計結果。

#### Scenario: 任一效能指標超過門檻
- **WHEN** 任一有效樣本集的計算結果超過指定門檻
- **THEN** `G-PERF` MUST 失敗，不得只以彙總截圖覆蓋原始結果

### Requirement: 安全、授權與來源邊界
驗證流程 SHALL 分別稽核 Shell mode opt-in、Safe Mode fail-closed、受保護檔案 mutation、路徑與參數注入、credential/clipboard/log redaction、dependency/license inventory，以及 PExplorer 僅供閱讀參考而未複製受限制來源。

#### Scenario: 發現來源或授權邊界違反
- **WHEN** audit 發現未揭露 dependency、授權不相容或 PExplorer 來源片段進入 production
- **THEN** `G-ARCH` 與 `G-SAFETY` MUST 失敗，直到由相應 owner 修正並重新稽核

### Requirement: 完整追溯與不可逃逸的 mandatory leaf
每個 mandatory task leaf SHALL 在所屬 change 的 append-only evidence index 中對應全域唯一 `<change-name>/<L3-id>` `task_id` 與 requirement/scenario/gate；mandatory leaf MUST NOT 使用 `not-applicable`。`superseded` 只有 replacement 存在、無循環、為 mandatory、覆蓋相同 requirement/scenario/gate 且具有有效 passed evidence 時才可算 covered。

#### Scenario: Replacement 懸空、循環或未完成
- **WHEN** validator 發現 replacement 不存在、形成循環、非 mandatory、coverage 不同或尚無 passed evidence
- **THEN** 原 leaf 與 replacement SHALL 均不得視為完成，`G-TRACE` MUST 失敗

### Requirement: 獨立發行複核
Independent reviewer SHALL 複核所有 blocking gate、evidence lineage 與 P0/P1 disposition，但 SHALL NOT 擔任 production remediation owner；修正與重跑由 Primary integrator 或原 gate owner 執行。

#### Scenario: 複核仍有未解 P0 或 P1
- **WHEN** independent review 發現未修正或證據不足的 P0/P1
- **THEN** M0 release MUST 被阻擋，修正後須重跑受影響 gate 並再次複核
