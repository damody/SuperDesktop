## ADDED Requirements

### Requirement: 每個使用中螢幕必須有 GPUI 桌面表面
系統 SHALL 為每個使用中螢幕建立 bottommost GPUI 桌面表面，並依螢幕 topology、work area 與 per-monitor DPI 更新其 bounds。

#### Scenario: 雙螢幕混合 DPI 啟動
- **WHEN** 系統在兩個 DPI 不同的使用中螢幕上啟動
- **THEN** 每個螢幕各有一個不重疊工作列的桌面表面，且內容使用該螢幕的邏輯比例繪製

#### Scenario: 顯示器熱插拔
- **WHEN** 使用者新增、移除或重新排列螢幕
- **THEN** 系統建立、移除或更新相應桌面表面，並將不可見項目位置安全地 clamp 或重新映射至可見螢幕

### Requirement: 桌布必須支援 Windows 配置模式
系統 SHALL 支援 fill、fit、stretch、center、tile 及 topology 允許時的 span 模式，且不得改變原始桌布檔案。

#### Scenario: 切換桌布配置
- **WHEN** 使用者選擇任一受支援桌布模式
- **THEN** 每個桌面表面依該模式與目前 DPI 重繪，並持久化模式而不改寫來源影像

#### Scenario: 桌布來源不可用
- **WHEN** 已設定的桌布無法讀取或解碼
- **THEN** 系統使用安全的語意背景色、保持桌面可操作，並記錄不含敏感完整路徑的診斷

### Requirement: 桌面項目必須使用穩定 Shell identity
系統 SHALL 合併 User Desktop 與 Public Desktop known folders，並以穩定 Shell identity 而非顯示名稱識別項目。

#### Scenario: 同名項目來自不同來源
- **WHEN** User Desktop 與 Public Desktop 含有相同顯示名稱但不同 identity 的項目
- **THEN** 系統保留兩個可分辨項目、各自圖示與位置，且不互相覆寫

#### Scenario: 顯示名稱變更
- **WHEN** 同一 Shell identity 的顯示名稱因外部變更而更新
- **THEN** 系統更新顯示名稱但保留選取與已保存位置

### Requirement: M0 桌面必須提供選取與啟動
系統 SHALL 支援單選、Ctrl/Shift 複選、rubber-band 選取、焦點、方向鍵導覽、Enter 與雙擊啟動，並提供對應協助工具狀態與 action。

#### Scenario: Rubber-band 複選
- **WHEN** 使用者在空白處拖曳選取框跨越多個項目
- **THEN** 所有相交且符合選取規則的項目進入 selected 狀態，視覺與協助工具樹一致

#### Scenario: 鍵盤啟動資料夾
- **WHEN** 焦點位於檔案系統資料夾且使用者按 Enter
- **THEN** 系統送出一次帶穩定 identity 的資料夾啟動命令給 `explorer-bridge`

#### Scenario: 啟動一般檔案
- **WHEN** 使用者雙擊具 Windows 關聯的一般檔案
- **THEN** 系統要求 Windows 依其關聯啟動該檔案，並將失敗呈現為可復原錯誤

#### Scenario: 一般檔案關聯啟動逾時或取消
- **WHEN** Windows association request 在 dispatcher 接受後 5 秒內未得到 admission terminal，或因關閉而取消
- **THEN** 系統只接受 `timed-out` 或 `cancelled` 的第一個 terminal，保持桌面可操作，並將較晚 callback 記為 late diagnostic

#### Scenario: 一般檔案關聯啟動失敗
- **WHEN** Windows 沒有可用關聯或 association adapter 回報錯誤
- **THEN** 系統顯示可用鍵盤與協助工具操作的 GPUI 錯誤/重試提示，且不把失敗標成已啟動

### Requirement: 桌面位置必須跨 DPI 與重啟持久化
系統 SHALL 以螢幕 identity、Shell item identity、DPI-aware 邏輯座標與 layout revision 保存桌面項目位置。

#### Scenario: 以指標重新定位桌面圖示
- **WHEN** 使用者拖曳已選取的桌面項目至同一桌面的新位置
- **THEN** 系統只更新該項目的桌面版面位置，不啟動檔案資料傳輸，並保存新的邏輯座標

#### Scenario: 重啟後恢復位置
- **WHEN** 使用者移動項目、正常關閉並在相同 topology 重新啟動
- **THEN** 項目回到保存的邏輯位置，且不超出可見桌面範圍

#### Scenario: DPI 改變後恢復位置
- **WHEN** 項目位置已保存後該螢幕 DPI 改變
- **THEN** 系統按邏輯座標重新投影並 clamp，而不是直接重用舊 physical pixel

### Requirement: 桌面監看必須可從 overflow 復原
系統 SHALL coalesce 重複檔案事件，拒絕 stale generation，並在 watcher overflow 時執行權威完整重新整理。

#### Scenario: Watcher overflow
- **WHEN** 桌面變更 queue 回報 overflow
- **THEN** 系統安排完整 namespace refresh、依穩定 identity 恢復仍存在項目的選取與位置，並產生可觀測 recovery event

#### Scenario: 過期刷新結果到達
- **WHEN** 舊 generation 的完整刷新在較新刷新完成後才到達
- **THEN** 系統拒絕舊結果，且不還原已刪除項目或破壞目前選取
