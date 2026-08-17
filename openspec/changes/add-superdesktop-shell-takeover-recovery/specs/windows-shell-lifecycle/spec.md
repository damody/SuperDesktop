## ADDED Requirements

### Requirement: Composition Root 必須只透過 Typed Contract 整合
`superdesktop-app` SHALL 作為唯一 composition root，將 settings、desktop、taskbar、SuperExplorer bridge 與 Windows effects 經 `shell-core` typed command/event contract 連接；preview composition SHALL 維持 zero Shell mutation。

#### Scenario: 固定入口啟動真實 SuperExplorer
- **WHEN** 使用者在 preview 以 pointer、keyboard 或 UIA invoke 桌面或工作列固定 SuperExplorer 入口
- **THEN** composition root SHALL 將同一 typed command 路由至 bridge，並收到 exactly-once terminal result

#### Scenario: Preview 組裝完成
- **WHEN** desktop、taskbar、bridge 與 settings 使用真實 adapters 完成 preview composition
- **THEN** Explorer、AppBar 與 work area SHALL 維持未變

### Requirement: Preview Mode 必須 Zero-mutation
系統 SHALL 預設 preview，且不得隱藏 Explorer、修改 work area、取得 Shell owner 或改 Shell registry。

#### Scenario: 一般啟動
- **WHEN** 未帶明確 `--shell`
- **THEN** preview 可操作且 Explorer/work area/registry 前後相同

### Requirement: 每個 Session 只能有一個 Owner
系統 SHALL 在任何 AppBar/Explorer mutation 前取得 PID/creation/session/user-token/application-file-identity/nonce fenced atomic lease，並在每次 mutation/cleanup 前 revalidate。

#### Scenario: 同時接管
- **WHEN** 兩個主程序同時要求接管
- **THEN** 只有一個成功，另一個在 mutation 前 already-owned

#### Scenario: Non-owner cleanup
- **WHEN** 非 owner 嘗試 cleanup
- **THEN** 拒絕且 owner surfaces/work area 不變

#### Scenario: Wrong file 或 wrong user token
- **WHEN** PID/creation/session 看似相符但 executable file identity 或 user token 不符或在競態中被替換
- **THEN** lease admission/revalidation MUST 拒絕且不得執行 mutation

### Requirement: Takeover 必須交易式
系統 SHALL 依 guardian、runtime、surfaces/AppBars、hooks/probes、health、Explorer switch 六階段執行；health SHALL 在五秒內確認 desktop/taskbar pointer、keyboard、focus 與必要 Start capability。

#### Scenario: 任一前置失敗
- **WHEN** 第 1–5 階段失敗/逾時
- **THEN** 不切換 Explorer並撤銷自有資源

#### Scenario: Reference Start 不可用
- **WHEN** frozen profile Start probe 失敗
- **THEN** 拒絕 Shell takeover

#### Scenario: Input health timeout
- **WHEN** desktop/taskbar input health 未在五秒內全部成功
- **THEN** rollback 自有資源且 Explorer surface SHALL 保持未切換

### Requirement: Guardian 必須防偽並安全恢復
系統 SHALL 以 inherited process handle/one-time channel 授權，驗證 owner/session/token，並使用 verified Windows Explorer path/explicit application/restricted handles/env。

#### Scenario: Forged/stale lease
- **WHEN** nonce、creation time、file identity、session 或 owner user-token identity 不符或在驗證期間被替換
- **THEN** 不修改 work area/AppBar且不啟動程序

#### Scenario: PATH/CWD substitute explorer
- **WHEN** 搜尋路徑含替代 explorer.exe
- **THEN** 只使用已驗證 system absolute path

### Requirement: Guardian Recovery 必須回復 Shell 且冪等
Guardian SHALL 先移除 SuperDesktop AppBars、恢復每個 monitor 的 work area，再探測並顯示同一互動 session 中既有且 identity 驗證通過的 Explorer Shell；只有不存在可用 Explorer 時才可啟動一次 verified system `explorer.exe`。重複 recovery trigger SHALL 共用單一 terminal 並不得重複啟動 Explorer。

#### Scenario: 既有 Explorer 可用
- **WHEN** crash recovery 發現同 session 已有 identity 驗證通過但被隱藏的 Explorer Shell
- **THEN** guardian SHALL 顯示既有 Shell 並不得 spawn 新 Explorer process

#### Scenario: Explorer 不存在
- **WHEN** AppBar/work area 已回復且同 session 沒有可用 Explorer Shell
- **THEN** guardian SHALL 只啟動一次 verified system Explorer 並等待 input-ready terminal

#### Scenario: 重複 Recovery Trigger
- **WHEN** 多個 callback/guardian path 同時觸發同一 crash recovery
- **THEN** 只有一個 recovery owner與 terminal，Explorer process count 不得因重複觸發增加

### Requirement: Crash Recovery 必須在 10 秒內完成
系統 SHALL 在 frozen reference profile 執行 10 次 forced crash，每次從 T0 至 Explorer pointer/keyboard input-ready 與正確 work area不超過 10 秒。

#### Scenario: 十次皆通過
- **WHEN** 每次保存 T0/ready/work-area/process identity
- **THEN** reference-profile `G-GUARDIAN-RECOVERY-PROVISIONAL` 通過；completion final gate 尚未判定

#### Scenario: 任一次超時
- **WHEN** 任一 run 超過 10 秒或未恢復
- **THEN** gate 失敗且 Shell mode 不可發行

### Requirement: FFI Callback 不得 Unwind 穿越 ABI
系統 SHALL 對每個 extern/system callback 使用 catch_unwind、typed fatal event、ownership validation 與 at-most-once release。

#### Scenario: Callback panic/race
- **WHEN** callback panic 或在 shutdown 時重複/late 到達
- **THEN** 不穿越 ABI、不 double-free，並進入 orderly/guardian recovery

### Requirement: 正常關閉必須有序且冪等
系統 SHALL 停止命令、取消、解除 hooks/AppBars、恢復 Explorer/work area、釋放 COM/GPUI、flush、最後 release lease。

#### Scenario: 重複 cleanup
- **WHEN** cleanup 重複執行
- **THEN** 最終狀態相同且不重複 Explorer/AppBar

### Requirement: M0 不得修改登入 Shell
系統 MUST NOT 改 Shell registry、autostart 或 installer state。

#### Scenario: 測試結束後登入設定
- **WHEN** 所有 takeover test 完成
- **THEN** 原登入 Shell 設定不變

### Requirement: Reference Lifecycle 只能發布 Provisional Disposition
本 change 在凍結 Windows 11＋ExplorerPatcher reference profile 的全部 mandatory lifecycle subchecks 通過後，SHALL 分別發布 `G-SHELL-TAKEOVER-PROVISIONAL` 與 `G-GUARDIAN-RECOVERY-PROVISIONAL`；MUST NOT 未經 completion verifier 的 candidate-bound lifecycle/installer evidence 就發布 final gates。

#### Scenario: Reference lifecycle 全部通過
- **WHEN** takeover failpoints、normal shutdown、10-run crash recovery、safety 與 evidence lineage 全部 passed
- **THEN** 只發布兩個 provisional dispositions，final `G-SHELL-TAKEOVER`/`G-GUARDIAN-RECOVERY` 保持未判定
