# superdesktop-evidence-governance Specification

## Purpose
TBD - created by archiving change bootstrap-superdesktop-workspace. Update Purpose after archive.
## Requirements
### Requirement: 每個完成 leaf 必須有有效證據
系統 SHALL 讓每個 change 擁有自己的 append-only `evidence/index.jsonl`，並以全域唯一 `<change-name>/<L3-id>` task_id 或 immutable shared record/subcheck 記錄 command/procedure、expected、actual、status/reviewer、hash、capability ID、requirement ID、scenario ID、gate、adjustment 與 timestamp。Capability、requirement 與 scenario ID SHALL 由版本化 machine-readable coverage manifest 提供。

Contract hash manifest SHALL 使用 repository-relative canonical path，逐列重算 input SHA-256 並拒絕不存在、漂移或逃逸 workspace 的 path；只驗證 manifest 自身 hash 不構成有效 contract evidence。

Corrective replacement manifest SHALL 涵蓋被取代 contract 的完整 effective input set，逐 input 通過同一 production verifier；replacement evidence SHALL 直接引用該 replacement manifest，而非只引用通用摘要或 corrective script manifest。

每個 predecessor manifest 的 path set SHALL 是 replacement 的不可縮減下限。Dependency/vendor更新後，locked metadata、license inventory及direct dependency provenance MUST 同步重生並通過source-boundary audit。Handoff aggregate contract MUST 提供明確 inputs manifest、hash演算法與可由production verifier重算的檔案hash。

Evidence record schema 與 coverage manifest schema SHALL 由支援 schema 所宣告 draft 的真正 JSON Schema engine 驗證，包括頂層型別、`additionalProperties`、format 與巢狀型別；所有負面 fixture MUST mutation/copy 真實資料後進入相同 production validation path。

#### Scenario: Task ID 只包含局部編號
- **WHEN** evidence record 的 task_id 只有 `3.1.3` 而沒有 change-name namespace
- **THEN** validator MUST 以 ambiguous identity 拒絕該 record

#### Scenario: Coverage 欄位缺失或漂移
- **WHEN** evidence 缺少 capability/requirement/scenario ID、引用未知 ID，或與 coverage manifest 不符
- **THEN** validator MUST 拒絕 record，且 replacement 不得視為保留相同 coverage

#### Scenario: 完成 leaf 缺少證據
- **WHEN** task 被勾選但 index 無有效紀錄
- **THEN** validator 失敗且 task 必須重開

#### Scenario: 尚未完成的 mandatory leaf 已先列入 coverage
- **WHEN** mandatory task 尚未勾選且尚無 passed record
- **THEN** validator 接受其規劃 coverage，但該 task 不得視為完成；一旦勾選就必須有相同 coverage 的有效 passed evidence

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

Adjustment validator SHALL 驗證 append-only supersession graph 的 identity 存在性、無循環與完整集合繼承；任何仍使用 artifact path/hash 的舊式 lineage，只有在有效 successor 明確 supersede 且以完整 immutable record mapping 取代後才不再生效。

#### Scenario: B 級規格修正
- **WHEN** 已完成 evidence 所依賴的 requirement 被 B 級修正
- **THEN** 舊 evidence 保留但標 stale，受影響 task 重開並指向 replacement evidence

#### Scenario: Corrective manifest 遺漏原 contract input
- **WHEN** replacement manifest 只涵蓋 corrective scripts，或少於 predecessor 的 effective input set
- **THEN** contract verifier MUST 拒絕 replacement，且相關 gate 保持 failed

#### Scenario: 依賴 inventory 與 lock/vendor 漂移
- **WHEN** locked package/version/vendor path沒有唯一且正確的license inventory/provenance coverage，或source-boundary audit失敗
- **THEN** dependency、source-boundary及aggregate contract MUST保持stale

#### Scenario: Fixture 旁路 production validator
- **WHEN** fixture 依名稱或 fault metadata 直接產生預期錯誤，而未 mutation/copy 資料並執行正式 schema、coverage、replacement 或 adjustment path
- **THEN** fixture matrix MUST 失敗且不得作為 passed evidence

#### Scenario: 專屬 replacement failure 被通用錯誤遮蔽
- **WHEN** dangling、cycle、nonmandatory、coverage-drift或unpassed fixture只觸發mandatory-without-evidence等通用前置錯誤
- **THEN** fixture MUST失敗；validator與mutation必須讓案例命中其專屬semantic diagnostic

#### Scenario: Adjustment successor 未完整取代 predecessor
- **WHEN** successor 不存在、形成循環、未 supersede 所有 legacy adjustment，或未完整涵蓋 predecessor effective stale set
- **THEN** validator MUST 拒絕 lineage 並保持所有相關 replacement 無效
