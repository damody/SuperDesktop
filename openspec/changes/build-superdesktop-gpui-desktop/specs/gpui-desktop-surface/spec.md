## ADDED Requirements

### Requirement: 每個 Monitor 必須有 DPI-aware GPUI Desktop
系統 SHALL 為每個 active monitor 建立 bottommost GPUI surface，並在 topology/DPI 改變時更新或移除。

#### Scenario: 虛擬 mixed-DPI topology
- **WHEN** 可控 virtual display 提供兩個不同 DPI monitor
- **THEN** 每個 monitor 有正確 logical bounds 的 desktop，且不重疊 taskbar work area

#### Scenario: Hot plug
- **WHEN** monitor 新增、移除或重排
- **THEN** surface 與 item positions 被建立、移除或安全 remap/clamp

### Requirement: 桌布模式必須完整且有界
系統 SHALL 支援 fill、fit、stretch、center、tile、span，並以 bounded async decode/cache 呈現。

#### Scenario: 六種模式
- **WHEN** 對固定 image/topology 依次選擇六種模式
- **THEN** geometry 符合各模式且來源檔不被改寫

#### Scenario: 來源不可用
- **WHEN** wallpaper 無法讀取/解碼
- **THEN** 使用 semantic fallback background，桌面仍可操作

### Requirement: User/Public Desktop 必須以 Stable Shell Identity 合併
系統 SHALL 保留同名不同 identity 項目，並解析 display name、icon descriptor 與 capability。

#### Scenario: 同名不同來源
- **WHEN** User/Public Desktop 有同名項目
- **THEN** 顯示兩個獨立項目且位置/選取不互相覆寫

#### Scenario: Identity 名稱變更
- **WHEN** 同 identity display name 外部變更
- **THEN** 名稱更新但位置/選取保留

### Requirement: 桌面必須提供完整 M0 互動
系統 SHALL 支援 single、Ctrl/Shift multi-select、rubber-band、keyboard focus/navigation、Enter/double-click activation 與 pointer position drag。

#### Scenario: Rubber-band
- **WHEN** selection rectangle 與多個 item 相交
- **THEN** visual selected state 與 accessibility tree 一致

#### Scenario: Pointer reposition
- **WHEN** 使用者拖曳 item 到新位置
- **THEN** 只更新 layout position，不啟動 file transfer

### Requirement: Item Position 必須跨重啟/DPI 保存
系統 SHALL 以 monitor/item identity、logical coordinate 與 layout revision 保存位置。

#### Scenario: DPI 改變
- **WHEN** 保存後 monitor DPI 改變
- **THEN** 位置依 logical coordinate 投影並 clamp 到 visible bounds

### Requirement: Watcher 必須從 Overflow/Stale 結果復原
系統 SHALL coalesce event，overflow 後 full refresh，並拒絕 stale generation。

#### Scenario: Overflow
- **WHEN** watcher queue overflow
- **THEN** full namespace refresh 恢復 identity/selection/position 並產生 recovery event

### Requirement: 一般檔案必須經 Windows Association 啟動
系統 SHALL 對非資料夾 item 使用 owned Shell identity 啟動 Windows association，具 5 秒 admission deadline、cancel 與 exactly-once terminal。

#### Scenario: Association 成功
- **WHEN** 有效關聯檔案被啟動
- **THEN** Windows 接受啟動且 terminal 為 launched

#### Scenario: Failure/timeout
- **WHEN** 無關聯、adapter failure 或 5 秒內無 terminal
- **THEN** GPUI 顯示 keyboard/UIA 可操作錯誤，late callback 不反轉結果

### Requirement: 資料夾必須路由到 SuperExplorer Bridge
系統 SHALL 對具 folder capability 的桌面項目發送單一 typed bridge command，並以 request/generation 接收 exactly-once terminal；desktop child 使用 fake bridge 驗證 contract，真實程序由整合 change 驗證。

#### Scenario: Enter 或 Double-click 資料夾
- **WHEN** 使用者以 Enter 或 double-click 啟動資料夾項目
- **THEN** desktop SHALL 發送恰好一個含 owned folder identity 的 bridge command

#### Scenario: Bridge 回傳失敗或逾時
- **WHEN** fake bridge 回傳 validation/spawn failure、cancel 或 timeout
- **THEN** desktop SHALL 顯示可操作 repair state，且 late terminal 不得反轉結果

### Requirement: 延後的 Desktop Actions 不得提早暴露
系統 SHALL 對 rename、native context menu、delete/recycle、explicit refresh command 與 file-transfer drag/drop 保留 typed unavailable 邊界，但 M0 MUST NOT 顯示可操作控制或執行對應 mutation。

#### Scenario: 使用者觸發延後操作
- **WHEN** 使用者按 F2、Delete、F5、context-menu gesture 或嘗試 file-transfer drag
- **THEN** 系統 SHALL 不執行檔案 mutation、不顯示虛假成功，且保持桌面可操作

### Requirement: 桌面必須固定顯示 SuperExplorer 入口
系統 SHALL 在每個 desktop surface 顯示 stable、truthful、keyboard/UIA 可操作的「SuperExplorer」固定入口；不得將無 initial path 的啟動標示為「本機」。

#### Scenario: 啟動 Desktop 固定入口
- **WHEN** 使用者以 pointer、Enter 或 UIA invoke 固定入口
- **THEN** desktop SHALL 發送一次 default bridge command，並依 terminal 顯示 launched 或 repair state
