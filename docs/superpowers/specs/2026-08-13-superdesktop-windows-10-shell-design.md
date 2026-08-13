# SuperDesktop Windows 10 桌面 Shell 設計

日期：2026-08-13

狀態：設計已核准，尚未開始實作

## 1. 目的

SuperDesktop 是使用 Rust 實作、僅支援 Windows 的桌面環境。它會重現 Windows 10 風格的使用者可見桌面 Shell，並以 `D:\SuperExplorer` 作為檔案總管。M0 的 UI/互動基準凍結為目前 Windows 11 build 26200.8875 加 ExplorerPatcher 26100.8457.70.3；工作列參考圖 SHA-256 為 `48B5F990B9E155C5C2719D8F8B41D88ED4420A46C3B6018278511F9C349B387E`。所有屬於產品的可見介面皆使用 GPUI 繪製。由於桌面註冊、AppBar、Shell Hook、COM/OLE、通知區相容性、螢幕拓撲及 DPI 事件無法脫離 Windows API 而可靠實作，因此這些功能會封裝於狹窄且不可見的 Windows 平台介面層。

專案根目錄為 `D:\SuperDesktop\SuperDesktop`，並擁有獨立的 Cargo workspace 與 Git 儲存庫。`D:\SuperExplorer` 維持獨立建置，SuperDesktop 透過程序邊界與它整合。

## 2. 範圍

### 2.1 長期產品範圍

完整產品以重現下列 Windows 10 Shell 行為為目標：

- 桌面表面、桌布、Shell 項目、選取、版面配置、重新命名、右鍵選單、拖放、重新整理及多螢幕行為。
- 固定於底部的工作列；預設高密度配置依照使用者提供的雙列參考圖，並可設定為一至三列。
- 視窗追蹤、啟用、最小化與還原、群組、釘選、排序、進度與注意狀態、縮圖、跳躍清單、工作檢視及顯示桌面。
- 開始按鈕、開始功能表、應用程式索引、搜尋、執行、電源、工作階段及常用 Shell 命令。
- 通知區、時鐘、行事曆、輸入法、網路、音量、電源狀態、通知徽章及相容的第三方通知區圖示。
- 鍵盤快速鍵、協助工具、高對比、每螢幕 DPI、多螢幕、虛擬桌面整合、自動啟動、可控的 Shell 安裝與復原。
- 由 SuperExplorer 處理檔案系統資料夾導覽。

「完整 Windows 10 行為」指的是可觀察的桌面 Shell 能力與互動集合。每項能力都必須列入相容性矩陣，並附上自動化或人工驗證證據。若畫面或行為由 Windows API 或第三方提供者控制，則不要求逐像素複製其未公開的實作細節。

### 2.2 不在範圍內

SuperDesktop 不會重新實作 Windows 核心、合成器、登入或鎖定畫面、安全邊界、裝置驅動程式、控制台內部、設定應用程式內部、內建 UWP 應用程式、網路堆疊、音訊堆疊或檔案系統。這些能力由既有 Windows 服務提供，SuperDesktop 只負責整合。

M0 不會一次完成全部長期範圍。M0 的責任是建立正式產品架構，並交付本文件所定義的第一個可用桌面與工作列切片。

## 3. 原始碼與授權邊界

`D:\SuperDesktop\PExplorer` 是源自 ReactOS Explorer 工作的 LGPL-2.1-or-later C++/Win32 參考專案。它可用於理解行為、訊息流程、Shell Hook、AppBar 用法及失敗案例。SuperDesktop 不得複製或機械式翻譯其原始碼。若日後確實要採用衍生程式碼，必須先另行作成授權決策，並在進入產品前加入明確歸屬說明。

`D:\SuperExplorer` 視為獨立建置的產品。SuperDesktop 不得依賴其未提交的工作樹、內部 crate 或 `vendor/gpui-ce` 路徑，以免兩個專案產生建置耦合，並保護使用者現有的 SuperExplorer 變更。

SuperDesktop 會固定自己的 GPUI-CE revision。M0 候選固定為 `https://github.com/damody/gpui-ce-explorer.git` commit `8945e2981b9fd00ca887e042d8adb9acc241b168` 的乾淨來源；不得依賴 `D:\SuperExplorer\vendor\gpui-ce` 目前的未提交 patch。兩個儲存庫分別管理 dependency lock 與升級時程。

## 4. 架構方案

### 4.1 可見介面與平台層

所有由 SuperDesktop 擁有的桌面、工作列、開始、搜尋、通知、設定、提示與復原介面都是 GPUI view。由 Windows 或第三方擁有、但由 SuperDesktop 呼叫的表面，例如 M0 的 Windows 開始體驗或原生 Shell 右鍵選單，仍由外部系統繪製。平台層可以建立不可見或 message-only HWND 並呼叫 Windows API，但不得實作 SuperDesktop 的產品介面。

主要事件流程如下：

```text
Windows 事件 → platform-win → shell-core 狀態轉移 → 不可變快照 → GPUI 繪製
GPUI 操作 → shell-core 命令 → platform-win 或 explorer-bridge 副作用 → 結果事件
```

平台 callback 不得直接修改 GPUI 狀態。它只能送出帶有穩定視窗、螢幕、桌面項目及請求識別碼的型別化事件。`shell-core` 是所有使用者可見狀態轉移的唯一權威。

### 4.2 Workspace 單元

- `superdesktop-app`：組合根、命令列模式、診斷、程序生命週期及有序關閉。
- `shell-core`：與平台無關的狀態機，管理螢幕、桌面項目、選取、工作按鈕、群組、釘選、焦點、命令及復原階段。
- `platform-win`：不可見的 Windows 整合層，負責桌面與 Shell 註冊、Shell Hook、視窗列舉、AppBar、螢幕與 DPI 事件、COM/OLE、已知資料夾、Shell 識別、圖示及右鍵選單工作階段。
- `desktop-ui`：GPUI 桌面表面、桌布、項目配置、互動及協助工具語意。
- `taskbar-ui`：GPUI 工作列、工作按鈕、開始入口、通知區、時鐘、溢出區及協助工具語意。
- `explorer-bridge`：SuperExplorer 的尋找、驗證、啟動、錯誤回報及未來的版本化 IPC。
- `settings-store`：版本化設定、原子寫入、遷移、損毀隔離及版面配置持久化。
- `superdesktop-guardian`：精簡的 Rust 復原程序，監看 Shell 租約，並在 SuperDesktop 異常結束後恢復可用的 Windows 工作階段。
- `superdesktop-test-support`：假的平台介面、確定性時鐘、受控測試根目錄、視窗輔助程式、視覺 fixture 及失敗注入。

每個單元都公開型別化合約。`desktop-ui` 與 `taskbar-ui` 依賴 `shell-core` 的值與命令，而不依賴 Win32 handle。Windows handle 與 COM interface 不得離開 `platform-win`。

## 5. 執行模式與 Shell 所有權

### 5.1 預覽模式

開發期間預設使用預覽模式。桌面與工作列會在一般 GPUI 視窗內繪製，不隱藏 Explorer，也不改變系統工作區。此模式不得修改任何 Shell 相關登錄值，適合介面迭代與有畫面的自動化測試。

### 5.2 Shell 模式

Shell 模式必須明確指定，並採用交易式接管：

1. 啟動 `superdesktop-guardian` 並建立租約。
2. 初始化診斷、GPUI、COM apartment、型別化事件 channel 及設定。
3. 建立所有螢幕的桌面表面與工作列 AppBar。
4. 註冊 Shell Hook、全域快速鍵、通知、螢幕與 DPI listener 及復原 callback。
5. 完成健康檢查，確認桌面與工作列皆能接受輸入。
6. 僅在前述步驟全部成功後，才隱藏或取代目前工作階段中由 Explorer 擁有的 Shell 表面。

若在第 6 步前失敗，只撤銷 SuperDesktop 自己建立的資源。若在第 6 步後失敗，guardian 必須移除 AppBar、恢復工作區，並顯示既有 Explorer Shell；若不存在可用的 Explorer Shell，則啟動 `explorer.exe`。

M0 提供執行期間的 Shell 模式，但不變更 Windows 登入時所設定的 Shell。登錄安裝、自動啟動、解除安裝與復原介面屬於後續的可控安裝里程碑。

## 6. M0 功能設計

### 6.1 桌面

M0 會為每個使用中的螢幕建立一個位於最底層的 GPUI 桌面表面。支援 Windows 桌布配置模式：填滿、適合、延展、置中、並排，以及在螢幕拓撲允許時跨螢幕延展。

項目來源會合併目前使用者的 Desktop 已知資料夾與 Public Desktop 已知資料夾。項目以穩定 Shell identity 作為識別，而不是以顯示名稱作為識別。M0 支援：

- 解析真實 Shell 圖示與顯示名稱。
- 單選、Ctrl/Shift 複選、框選、焦點、鍵盤導覽及啟動。
- 雙擊與 Enter 啟動。
- 依穩定項目識別、螢幕識別及 DPI-aware 邏輯座標保存圖示位置。
- 監看檔案系統變更、合併重複事件，並在 watcher overflow 後以完整重新整理復原。

開啟檔案時使用一般 Windows 關聯。開啟檔案系統資料夾時則透過 `explorer-bridge`。由於目前 SuperExplorer 的啟動合約只接受以 `EXPLORER_INITIAL_PATH` 傳入的絕對資料夾路徑，M0 不得把未設定該變數的啟動宣稱為「本機」導覽；桌面與工作列只顯示「SuperExplorer」入口，讓 SuperExplorer 自行選擇預設位置。真正的「本機」synthetic root 導覽必須等版本化 IPC 或明確啟動合約完成後才能啟用。

重新命名、原生右鍵選單、拖放、自動排列、貼齊格線、排序、重新整理命令及資源回收筒異動會在 M0 後的桌面相容性增量中完成。M0 必須預留型別化命令與狀態邊界，但不得提早顯示可操作卻尚未實作的控制項。

### 6.2 工作列

工作列固定於底部，並在 Shell 模式註冊為 Windows AppBar。預設為兩列緊湊配置，資訊密度依照使用者提供的參考圖；使用者可設定一、二或三列。架構從一開始就支援每螢幕工作列；初始驗收矩陣要求雙螢幕環境中的主要與次要工作列皆能運作。

左側為開始按鈕。M0 會呼叫目前 ExplorerPatcher/Windows 提供的開始體驗，而不繪製自訂開始功能表。在凍結參考環境上，僅當開始主機能力探測成功時才允許 Shell 接管；若之後呼叫失敗，應回報為可復原的降級狀態。在其他相容環境上，若開始主機不存在，按鈕可以顯示為具備正確協助工具語意的不可用狀態。中央區域顯示含應用程式圖示與省略標題的工作按鈕；藍色底線代表執行中工作或群組，作用中背景代表目前前景工作。右側包含溢出入口、核心系統狀態、時間、日期及通知數量。

M0 視窗行為包含：

- 透過 Shell Hook 事件尋找符合資格的頂層視窗，並定期以 `EnumWindows` 對帳。
- 過濾 tool window、cloaked window、擁有者暫時視窗、不可見視窗及明確排除的視窗。
- 在標題、圖示、注意、最小化及前景狀態變更時維持穩定順序。
- 點擊時啟用視窗、最小化目前前景視窗，或還原已最小化視窗。
- 釘選應用程式沒有視窗時可直接啟動。
- 依解析後的應用程式 identity 將視窗分組，同時保留各子視窗狀態。
- 具備有界失效策略的圖示與標題快取。
- 固定顯示 SuperExplorer 入口。

M0 的通知區提供時間、日期及由 Windows 取得的核心狀態。完整第三方 `Shell_NotifyIcon` 相容性、溢出管理、跳躍清單、即時縮圖、拖曳排序、徽章、進度，以及自訂開始與搜尋介面，會在後續工作列相容性增量中完成。

### 6.3 SuperExplorer 整合

M0 使用既有 SuperExplorer 啟動合約，不修改其目前含有未提交變更的工作樹：

- 啟動前必須確認設定的執行檔為確實存在的絕對執行檔路徑。
- 尋找順序依次為：持久化使用者設定、開發產物 `D:\SuperExplorer\target\release\SuperExplorer.exe`、與 SuperDesktop 相鄰安裝的 `SuperExplorer.exe`。
- 啟動檔案系統資料夾時，建立新的 SuperExplorer 程序，將 `EXPLORER_INITIAL_PATH` 設為確實存在的絕對資料夾，且不傳入不受支援的命令列參數。
- 「SuperExplorer」入口在未設定 `EXPLORER_INITIAL_PATH` 的情況下啟動應用程式，且 UI 不得把結果標示為保證導覽至「本機」。
- 每個啟動請求都有 correlation ID，且只會產生一個成功或失敗結果事件。
- 執行檔缺失、無效或啟動失敗時，顯示 GPUI 復原提示並寫入診斷事件。SuperDesktop 不得在資料夾導覽時靜默改用 Windows Explorer。

若要導覽已存在的 SuperExplorer 程序，必須另行建立版本化 IPC 合約。這項工作屬於 SuperExplorer 中獨立且需協調的 OpenSpec change，不包含於 M0。

## 7. 狀態、並行與資料流

`shell-core` 擁有唯一的邏輯快照，其中包含螢幕拓撲、桌面模型、工作列模型、選取與焦點、應用程式 identity、設定 revision 及復原階段。平台工作採非同步方式執行，且每項工作都帶有 request ID 與 generation。來自過期 generation 的結果必須拒絕。

高頻率視窗、前景、標題、圖示及檔案系統事件，會先依穩定 identity 合併，再送往 renderer。有界 queue 必須明確回報 overflow，不得靜默遺失狀態。發生 overflow 時，安排權威對帳：工作列使用 `EnumWindows`，桌面項目使用完整 namespace 重新整理。

具備 COM/OLE apartment affinity 的值留在其擁有的平台執行緒。跨執行緒事件只包含 Rust 擁有的值。關閉時依序停止接受新命令、取消進行中請求、解除外部 callback、移除 AppBar、銷毀 GPUI 視窗、釋放 COM 資源、寫入診斷與設定，最後才釋放 guardian 租約。

## 8. 設定與持久化

設定使用明確版本化 schema，並以成功後原子取代方式寫入。保存內容包含執行模式偏好、工作列列數、螢幕位置、釘選順序、桌布配置、桌面項目座標、SuperExplorer 執行檔路徑、主題及協助工具偏好。

若個別欄位無效且可安全回復，則只對該欄位使用預設值。若設定檔無法讀取或結構無效，將它重新命名為帶時間戳的隔離檔案、記錄診斷，再以安全預設值重建。版面配置資料以穩定螢幕及 Shell 項目 identity 為鍵，避免 DPI 或顯示器順序變更破壞無關的配置。

## 9. 錯誤處理與復原

- 無法建立所有必要桌面表面或 AppBar 時，不得接管 Shell。
- 選用遙測或非必要狀態提供者註冊失敗時，只降級該提供者，並如實顯示可用狀態。
- Shell Hook 事件遺失或 overflow 時，執行權威視窗對帳。
- 桌面 watcher overflow 時，執行權威桌面重新整理，並依穩定 identity 保留選取。
- SuperExplorer 啟動失敗時，桌面與工作列仍須保持回應，並提供修復動作。
- 若第三方 Shell 或通知區提供者可能卡住，相關能力在啟用前必須放入有界 worker 或獨立程序。
- 設定損毀時隔離並重建，不得阻止 Shell 進入可用狀態。
- panic 或程序異常終止時啟動 guardian 復原。復原操作必須具備冪等性，可安全重複執行。
- 診斷預設不得記錄檔案內容、憑證、剪貼簿資料或完整使用者路徑；只有明確啟用的本機偵錯模式可以放寬此限制。

## 10. 協助工具、在地化與視覺行為

每個可互動 GPUI 元素都必須具備穩定的協助工具 identity、role、name、state 與 action。純鍵盤操作涵蓋桌面選取、工作切換、啟動、已實作的右鍵動作及復原提示。焦點必須永遠可見。高對比模式以系統語意角色對應 token，不使用固定顏色覆蓋。

版面配置使用邏輯單位及 per-monitor DPI。必要驗證比例為 100%、125%、150%、175% 及 200%。文字截斷、雙向版面、繁體中文、簡體中文、英文及 IME 互動都屬於第一級限制。M0 至少提供繁體中文與英文的全部可見字串資源。

Windows 10 視覺目標透過語意 token、幾何量測及參考擷取驗證。第三方圖示繪製、字型 rasterization 及 OS provider 像素屬於受控的外部變異。

候選畫面產生前必須凍結視覺比較 contract：100% DPI 的幾何 anchor、工作列/列高與 hit target 容許 ±2 physical px，其他 DPI 依 scale 四捨五入；只有時間、日期、通知數與 fixture window title 可使用預先雜湊的固定矩形遮罩；遮罩外 SSIM 必須至少 0.95，控制 identity 與 state 必須精確相符。看到候選結果後修改容差、遮罩或演算法會使舊視覺證據失效並要求全數重跑。

## 11. 驗證策略

### 11.1 自動化測試

- 測試所有 `shell-core` 狀態轉移、資格規則、群組規則、選取規則、排序規則、generation 檢查及復原轉移。
- 以 property 與序列測試覆蓋重複、缺漏、重排、過期及 overflow 的 Shell 事件。
- 使用假的平台、螢幕、時鐘、設定及檔案總管介面進行合約測試。
- Windows 整合測試只使用受控輔助視窗與受控暫存資料夾。
- 測試 SuperExplorer 路徑解析及環境變數建立，但不得修改 SuperExplorer 儲存庫。
- 在每個接管階段注入 crash 與啟動失敗。
- 測試協助工具樹及 action。
- 為桌面與工作列狀態及各 DPI 建立確定性 GPUI 視覺 fixture。
- 執行 Cargo format、check、以 warning 為錯誤的 clippy、workspace test、架構檢查及依賴與授權稽核。

### 11.2 有畫面與人工測試矩陣

- Windows 11 build 26200.8875 + ExplorerPatcher 26100.8457.70.3 為 M0 UI/互動 reference profile；Windows 10 22H2 x64 為相容性目標。
- 單螢幕為必要本機 gate；虛擬顯示器 topology 用於自動化多螢幕 gate，真實 mixed-DPI 雙螢幕為 release-candidate confirmation gate。
- 淺色、深色及高對比主題。
- 鍵盤、指標、觸控大小 hit target、IME 及 UI Automation 檢查。
- 視窗事件風暴、標題與圖示頻繁變更、卡住的輔助程序、SuperExplorer 缺失、顯示器熱插拔、Explorer 重啟及 SuperDesktop crash 復原。
- 預覽模式共存及 Shell 模式接管與還原。

### 11.3 M0 效能預算

- 在參考機器上，冷啟動至可互動預覽或完成 Shell 健康檢查不得超過 2 秒。
- 穩定後閒置 CPU 中位數低於 0.5%。
- Shell 事件至工作列可見更新的 p95 低於 100 ms。
- 在參考機器上，M0 穩定後 working set 低於 150 MiB。
- 壓力期間事件 queue 保持有界；overflow 後必須能恢復至權威狀態。

## 12. M0 驗收條件

只有在下列項目全部驗證通過後，M0 才算完成：

1. 預覽模式可繪製並操作桌面與工作列，且不改變目前 Windows Shell。
2. Shell 模式完成交易式接管；正常退出時恢復 Explorer 與工作區。
3. 強制終止 SuperDesktop 後，guardian 能恢復可用的 Explorer 工作階段。
4. 真實 Shell 項目的桌面選取、框選、鍵盤導覽、啟動、桌布模式及持久化圖示位置皆能運作。
5. 檔案系統資料夾依已驗證的 `EXPLORER_INITIAL_PATH` 合約啟動 SuperExplorer；SuperExplorer 缺失時顯示可復原的 GPUI 錯誤。
6. 雙列工作列能追蹤、啟用、最小化、還原、群組及釘選真實應用程式視窗，且不會發生不穩定重排。
7. 主要與次要工作列在混合 DPI 顯示器變更期間仍維持正確位置。
8. 在凍結 ExplorerPatcher reference profile 上，開始按鈕能在預覽與 Shell 工作階段呼叫目前開始體驗；若必要能力探測失敗，必須拒絕 Shell 接管。
9. 協助工具、在地化、視覺、生命週期、壓力及效能 gate 均通過並保存證據。
10. 任何測試或復原程序都不得修改或刪除明確受控測試根目錄以外的資料。
11. 相容性矩陣必須如實標示延後的 Windows 10 能力，不得把占位介面宣稱為已完成功能。

## 13. M0 後交付順序

後續變更仍須個別定義與驗證：

1. 桌面相容性：重新命名、選單、拖放、排列與排序、重新整理、資源回收筒及進階 namespace 項目。
2. 工作列相容性：重排、跳躍清單、縮圖、進度、徽章、注意狀態、完整釘選生命週期及多螢幕策略。
3. 開始與搜尋：應用程式索引、Win32/UWP 項目、釘選配置、搜尋、執行、電源及工作階段命令。
4. 通知區與重要訊息中心：第三方通知區相容性、溢出、flyout、時鐘與行事曆、快速操作及通知。
5. Shell 整合：快速鍵、虛擬桌面整合、設定、自動啟動、可控安裝、解除安裝及復原介面。
6. 相容性強化：完整 Windows 10 能力矩陣、協助工具、在地化、效能、soak 及發行證據。

每個增量都必須擁有自己的 OpenSpec proposal、design、delta spec、task plan、implementation 與 verification evidence。

## 14. Brainstorming 階段決策

- 所有產品可見介面使用 GPUI，只保留最小且不可見的 Win32 adapter。
- 同時納入桌面與工作列，並優先重現使用者提供的雙列工作列參考圖。
- Windows 10 風格是產品目標；目前 Windows 11 + ExplorerPatcher 配置是 M0 可重現的 UI/互動 reference profile，Windows 10 22H2 是相容性目標。
- SuperDesktop 與 SuperExplorer 之間保留程序邊界。
- M0 使用既有 `EXPLORER_INITIAL_PATH` 合約，不立即修改 SuperExplorer。
- 預設使用預覽模式；只有明確指定時才進行交易式 Shell 接管。
- 在任何工作階段層級 Shell 取代前，先加入獨立 Rust guardian。
- PExplorer 只作為行為參考，不進行機械式原始碼移植。
- 將完整相容性拆為可獨立規格化的增量，不在第一版宣稱完成全部 Windows 10 相容性。

## 15. M0 OpenSpec 執行分解

M0 不得作為單一巨型 apply 執行。`build-superdesktop-shell-foundation` 是 program change，只凍結跨 change 合約、依賴順序、blocking gate 與整體驗收。實作依序拆為：

1. `bootstrap-superdesktop-workspace`
2. `validate-superdesktop-windows-platform`
3. `build-superdesktop-shell-core`
4. `build-superdesktop-gpui-desktop`
5. `build-superdesktop-gpui-taskbar`
6. `integrate-superexplorer-process-bridge`
7. `add-superdesktop-shell-takeover-recovery`
8. `verify-superdesktop-m0`

第 4、5、6 項可在第 2、3 項完成後平行進行；第 7 項必須等待桌面與工作列整合完成；第 8 項必須等待所有 production change 完成。

目前開發機是 Windows 11 build 26200.8875，已安裝 ExplorerPatcher 26100.8457.70.3，且只有一個使用中螢幕。此環境可作主要 UI/互動 reference，並可使用虛擬顯示器驗證 topology 邏輯；Windows 10 22H2 相容性與真實 mixed-DPI 雙螢幕確認仍需外部環境。缺少外部環境時，只阻擋 release-candidate confirmation，不阻擋前面的實作 change，也不得把未執行的 confirmation 標為完成。
