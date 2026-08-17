## ADDED Requirements

### Requirement: 工作列必須以 GPUI 繪製並保留 work area
系統 SHALL 為每個目標螢幕建立底部 GPUI 工作列；Shell 模式下必須以 AppBar 保留 work area，預覽模式不得修改 work area。

#### Scenario: Shell 模式雙螢幕工作列
- **WHEN** Shell 模式在雙螢幕 topology 完成接管
- **THEN** 每個目標螢幕均有正確 bounds 的工作列 AppBar，且一般 maximized window 不被工作列遮住

#### Scenario: 預覽模式工作列
- **WHEN** 以預覽模式顯示工作列
- **THEN** 工作列位於一般 GPUI 預覽視窗，系統 work area 與 Explorer 工作列保持不變

### Requirement: 工作列列數必須可設定且預設為雙列
系統 SHALL 提供一至三列配置，首次啟動預設為兩列高密度配置，並在 DPI 或螢幕變更後維持有效 hit target 與文字截斷。

#### Scenario: 首次啟動
- **WHEN** 沒有既有 taskbar row 設定
- **THEN** 中央工作區以兩列顯示圖示與省略標題，右側系統區保持可讀且可操作

#### Scenario: 無效列數設定
- **WHEN** 設定包含小於一或大於三的列數
- **THEN** 系統只對該欄位回復為兩列、記錄設定修正，且不阻止工作列啟動

### Requirement: 系統必須權威追蹤符合資格的頂層視窗
系統 SHALL 使用 Shell Hook 作增量更新並以 `EnumWindows` 週期性或 overflow 後對帳，且 SHALL 排除 invisible、tool、cloaked、owned transient 及明確排除視窗。

#### Scenario: 新增符合資格的視窗
- **WHEN** Shell Hook 回報新的符合資格頂層視窗
- **THEN** 工作列在一個 event-to-visible budget 內顯示對應 task，且後續對帳不產生重複項目

#### Scenario: Hook 事件遺失
- **WHEN** 視窗已存在但增量事件遺失或 queue overflow
- **THEN** 下一次權威對帳補入或移除 task，並記錄 reconciliation 原因

#### Scenario: 排除暫時視窗
- **WHEN** 視窗為 tool、cloaked、owned transient 或不可見
- **THEN** 系統不把它顯示為一般 task button

### Requirement: Task 順序與群組必須穩定
系統 SHALL 分離 WindowId 與 ApplicationId，依 application identity 分組，並禁止標題、圖示、foreground、attention 或 minimize 更新造成非使用者要求的重排。

#### Scenario: 同一應用程式建立第二個視窗
- **WHEN** 既有 application group 新增第二個符合資格視窗
- **THEN** 系統保留群組位置、更新群組子視窗狀態，且不把其他群組重排

#### Scenario: 視窗標題快速變更
- **WHEN** task 的標題或圖示在短時間內多次變更
- **THEN** 顯示內容更新但 task/group 順序保持不變

### Requirement: Task 按鈕必須執行 Windows 基本切換語意
系統 SHALL 在點擊 task 時啟用非前景視窗、還原已最小化視窗、最小化目前前景視窗，並在 pinned application 沒有視窗時啟動它。

#### Scenario: 點擊目前前景視窗
- **WHEN** 使用者點擊代表目前前景視窗的 task button
- **THEN** 系統提出一次最小化命令並在平台確認後更新狀態

#### Scenario: 點擊已最小化視窗
- **WHEN** 使用者點擊代表已最小化視窗的 task button
- **THEN** 系統還原並嘗試啟用該視窗，且失敗時保持可重新操作並顯示真實狀態

#### Scenario: 點擊沒有視窗的釘選應用程式
- **WHEN** pinned application 目前沒有符合資格視窗且使用者點擊它
- **THEN** 系統送出一次 application launch effect，並以 correlation ID 收斂為單一 terminal event

### Requirement: 工作列必須呈現基本執行與系統狀態
系統 SHALL 以藍色底線表示執行中 task/group、以作用中背景表示 foreground，並顯示開始入口、網路連線狀態、音量/靜音狀態、作用中輸入語言、可取得時的電源/電池狀態、通知數量、時間與日期。M0 不得把這些固定系統狀態宣稱為完整第三方通知區相容性。

#### Scenario: Foreground 變更
- **WHEN** Windows foreground window 從 task A 變成 task B
- **THEN** A 的 active state 被移除、B 的 active state 被設定，兩者的 running indicator 與順序保持正確

#### Scenario: 開始主機不可用於預覽或非參考平台
- **WHEN** 開始主機能力不存在但目前模式不要求 Shell 接管
- **THEN** 開始按鈕顯示為 accessibly unavailable，且不會執行偽造或無回應的命令

#### Scenario: 系統狀態提供者可用
- **WHEN** Windows 回報網路、音量、輸入語言、電源或通知數量變更
- **THEN** 對應狀態在工作列更新，並具有可讀的 accessible name/state

#### Scenario: 個別系統狀態提供者不可用
- **WHEN** 任一非必要系統狀態提供者回報 unavailable
- **THEN** 只有該狀態顯示 truthful unavailable，時鐘、工作按鈕與其他狀態仍可操作

### Requirement: 工作列必須固定提供 SuperExplorer 入口
系統 SHALL 在每個主要工作列固定顯示可由 pointer、keyboard 與 accessibility action 操作的「SuperExplorer」入口；其啟動語意為應用程式預設位置，不得依賴目前已有 SuperExplorer 視窗才顯示，也不得標示為保證導覽至「本機」。

#### Scenario: SuperExplorer 未執行
- **WHEN** 工作列顯示且目前沒有 SuperExplorer 視窗
- **THEN** 固定入口仍存在，啟動後透過 `explorer-bridge` 送出一次無 initial-path 的 SuperExplorer 請求

#### Scenario: SuperExplorer 啟動失敗
- **WHEN** 使用者操作固定入口但 executable resolver 或 spawn 失敗
- **THEN** 入口保持可操作，系統顯示規範的 GPUI 修復提示，且不靜默改用 Windows Explorer

### Requirement: Reference Start 必須分模式驗證
系統 SHALL 在凍結 ExplorerPatcher profile 分別驗證 preview 與 Shell mode 的 Start probe/invocation；Shell mode 結果是 takeover health 的必要輸入。

#### Scenario: Reference Preview 與 Shell mode Start
- **WHEN** 在凍結 ExplorerPatcher profile 分別以 preview 與 Shell mode 啟動 Start
- **THEN** 兩種模式都必須保存獨立 probe/invocation 結果；Shell mode probe 失敗 SHALL 阻止 takeover
