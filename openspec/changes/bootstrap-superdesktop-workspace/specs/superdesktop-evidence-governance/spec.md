## ADDED Requirements

### Requirement: 每個完成 leaf 必須有有效證據
系統 SHALL 讓每個 change 擁有自己的 append-only `evidence/index.jsonl`，並以全域唯一 `<change-name>/<L3-id>` task_id 或 immutable shared record/subcheck 記錄 command/procedure、expected、actual、status/reviewer、hash、capability ID、requirement ID、scenario ID、gate、adjustment 與 timestamp。Capability、requirement 與 scenario ID SHALL 由版本化 machine-readable coverage manifest 提供。

Contract hash manifest SHALL 使用 repository-relative canonical path，逐列重算 input SHA-256 並拒絕不存在、漂移或逃逸 workspace 的 path；只驗證 manifest 自身 hash 不構成有效 contract evidence。

#### Scenario: Task ID 只包含局部編號
- **WHEN** evidence record 的 task_id 只有 `3.1.3` 而沒有 change-name namespace
- **THEN** validator MUST 以 ambiguous identity 拒絕該 record

#### Scenario: Coverage 欄位缺失或漂移
- **WHEN** evidence 缺少 capability/requirement/scenario ID、引用未知 ID，或與 coverage manifest 不符
- **THEN** validator MUST 拒絕 record，且 replacement 不得視為保留相同 coverage

#### Scenario: 完成 leaf 缺少證據
- **WHEN** task 被勾選但 index 無有效紀錄
- **THEN** validator 失敗且 task 必須重開

### Requirement: Mandatory leaf 不得以 N/A 或無效 replacement 規避
系統 MUST 拒絕 mandatory `not-applicable`，以及 dangling、cyclic、非 mandatory、coverage 不同或尚未 passed 的 superseded replacement。

#### Scenario: 循環 replacement
- **WHEN** A supersedes B 且 B 直接或間接 supersedes A
- **THEN** validator 拒絕兩者完成

#### Scenario: 合法 replacement
- **WHEN** replacement 存在、mandatory、同 requirement/scenario/gate coverage 且已有有效 passed evidence
- **THEN** 原 leaf 才可記錄 superseded lineage

### Requirement: 調整必須保留 lineage
系統 SHALL 記錄 A/B/C adjustment；B 級使相依證據 stale，C 級在使用者核准前不得套用。

Adjustment 的 stale 與 replacement lineage MUST 引用 immutable `task_id#subcheck` record identity；validator SHALL 驗證雙向連結、同 coverage、replacement passed、完整受影響集合，以及 C 級 adjustment 的有效使用者核准 record。

#### Scenario: B 級規格修正
- **WHEN** 已完成 evidence 所依賴的 requirement 被 B 級修正
- **THEN** 舊 evidence 保留但標 stale，受影響 task 重開並指向 replacement evidence
