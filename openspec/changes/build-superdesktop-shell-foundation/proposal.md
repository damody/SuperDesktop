## Why

目前 `D:\SuperDesktop` 只有可供行為研究的舊 C++/Win32 PExplorer，尚無可維護、可驗證且以 Rust/GPUI 實作的 Windows 桌面環境。現在需要先建立 SuperDesktop 的 M0 基礎，使桌面、雙列工作列、SuperExplorer 整合及 Shell 異常復原形成可實際套用與驗收的產品骨架，再於後續 change 擴充完整 Windows 10 相容性。

## What Changes

- 建立獨立 Windows-only Rust Cargo workspace，所有 SuperDesktop 擁有的可見介面皆由 GPUI 繪製，Win32/COM 僅位於不可見平台介面層。
- 建立預覽模式與交易式 Shell 模式；Shell 接管前必須通過健康檢查，異常結束時由獨立 guardian 恢復 Explorer 與螢幕工作區。
- 建立每螢幕 GPUI 桌面表面，提供 Windows 桌布模式、使用者與公用桌面 Shell 項目、選取、鍵盤導覽、啟動及位置持久化。
- 建立依參考圖設計的底部高密度雙列工作列，並支援一至三列、AppBar、多螢幕、視窗追蹤、群組、啟用、最小化、還原及釘選。
- 建立 SuperExplorer 程序橋接；以既有 `EXPLORER_INITIAL_PATH` 合約開啟檔案系統資料夾，且在執行檔缺失或啟動失敗時提供可復原錯誤。
- 建立版本化設定、原子寫入、損毀隔離、事件 overflow 對帳、隱私安全診斷與有序關閉。
- 建立 Windows 10 22H2 參考驗證、Windows 11 可用性檢查、混合 DPI 雙螢幕、協助工具、在地化、效能、壓力與復原證據體系。
- 明確延後完整開始功能表與搜尋、完整第三方通知區、跳躍清單、縮圖、桌面重新命名與拖放、虛擬桌面、登錄安裝與完整相容性強化；這些能力各自使用後續 OpenSpec change。

## Capabilities

### New Capabilities

- `windows-shell-lifecycle`：預覽模式、交易式 Shell 接管、AppBar/Shell Hook 生命週期、guardian 復原與有序關閉。
- `gpui-desktop-surface`：每螢幕 GPUI 桌面、桌布、Shell 項目、選取、鍵盤啟動、變更監看及位置持久化。
- `gpui-taskbar-window-management`：一至三列 GPUI 工作列、視窗資格、追蹤、排序、群組、啟用、最小化、還原、釘選及多螢幕配置。
- `superexplorer-process-bridge`：SuperExplorer 執行檔解析、路徑啟動合約、correlation ID、單一終端事件及可復原失敗。
- `shell-settings-and-reconciliation`：版本化設定、原子持久化、損毀隔離、generation/stale-state 拒絕及 overflow 權威對帳。
- `shell-foundation-verification`：Windows 版本、DPI、螢幕、協助工具、在地化、生命週期、安全、效能及證據追溯 gate。

### Modified Capabilities

無；此儲存庫目前沒有既有 OpenSpec capability。

## Impact

- 新增 `D:\SuperDesktop\SuperDesktop` 內的 Cargo workspace、GPUI-CE 固定依賴、Windows API bindings、測試支援、診斷與建置腳本。
- 執行期間會使用 HWND、Shell Hook、AppBar、COM/OLE、Known Folder、Shell identity、monitor/DPI 及程序啟動 API；M0 不修改 Windows 登入 Shell 登錄值。
- `D:\SuperExplorer` 維持獨立且不被本 change 修改；整合只透過已存在的程序與 `EXPLORER_INITIAL_PATH` 環境變數合約。
- `D:\SuperDesktop\PExplorer` 只作行為與 API 研究來源，不複製或機械式翻譯 LGPL 程式碼。
- Shell 模式可能影響目前使用者工作階段的桌面與工作區，因此所有接管、退出與 crash 路徑都受 blocking recovery gate 約束。
