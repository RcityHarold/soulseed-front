use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub use soulseed_agi_core_models::legacy::dialogue_event::DialogueEvent;
pub use soulseed_agi_core_models::{
    AccessClass, AwarenessDegradationReason, AwarenessEvent, AwarenessEventType,
    AwarenessFork, ConversationScenario, DecisionPlan, DialogueEventType,
    SyncPointKind, SyncPointReport,
};

// 前端不再定义自己的 AwarenessAnchor 和 DecisionPath
// anchor 和 decision_path 字段都使用 Value 类型来避免严格的 ID 验证

// 辅助函数：反序列化可以是字符串或整数的 ID 字段
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(serde::de::Error::custom("expected string or number")),
    }
}

// 辅助函数：反序列化可选的字符串或整数 ID 字段
fn deserialize_optional_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(Value::Number(n)) => Ok(Some(n.to_string())),
        _ => Err(serde::de::Error::custom("expected string, number, or null")),
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimelinePage<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimelinePayload {
    #[serde(default)]
    pub items: Vec<DialogueEvent>,
    #[serde(default)]
    pub awareness: Vec<AwarenessEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AceLane {
    Clarify,
    Tool,
    SelfReason,
    Collab,
}

impl Default for AceLane {
    fn default() -> Self {
        AceLane::Clarify
    }
}

impl AceLane {
    pub fn from_label(label: &str) -> Self {
        match label.to_ascii_lowercase().as_str() {
            "tool" | "tool_lane" => AceLane::Tool,
            "self_reason" | "self" | "selfreason" => AceLane::SelfReason,
            "collab" | "collaboration" => AceLane::Collab,
            _ => AceLane::Clarify,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AceCycleStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl AceCycleStatus {
    pub fn from_label(label: &str) -> Self {
        match label.to_ascii_lowercase().as_str() {
            "pending" => AceCycleStatus::Pending,
            "running" => AceCycleStatus::Running,
            "completed" | "complete" | "success" => AceCycleStatus::Completed,
            "failed" | "failure" | "error" => AceCycleStatus::Failed,
            "cancelled" | "canceled" => AceCycleStatus::Cancelled,
            _ => AceCycleStatus::Running,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AceBudget {
    #[serde(default)]
    pub tokens_allowed: Option<u32>,
    #[serde(default)]
    pub tokens_spent: Option<u32>,
    #[serde(default)]
    pub walltime_ms_allowed: Option<u64>,
    #[serde(default)]
    pub walltime_ms_used: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AceCycleSummary {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    pub lane: AceLane,
    pub status: AceCycleStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    // anchor 也使用 Value 类型，避免严格的 ID 验证
    pub anchor: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<AceBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_sync_point: Option<SyncPointReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_injections: Vec<HitlInjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_path: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleOutcomeSummary {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleTriggerResponse {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleScheduleView {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    pub lane: String,
    // anchor 使用 Value 类型，避免严格的 ID 验证
    pub anchor: Value,
    pub budget: BudgetSnapshotView,
    // created_at 是 OffsetDateTime，序列化为数组格式
    pub created_at: Value,
    // decision_events 使用 Value 类型，因为 AwarenessEvent 包含 flatten 的 AwarenessAnchor，其中有严格的 TenantId
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub decision_events: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explain_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub router_decision: Option<RouterDecisionView>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_cycle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collab_scope_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetSnapshotView {
    pub tokens_allowed: u32,
    pub tokens_spent: u32,
    pub walltime_ms_allowed: u64,
    pub walltime_ms_used: u64,
    #[serde(default)]
    pub external_cost_allowed: f32,
    #[serde(default)]
    pub external_cost_spent: f32,
}

impl From<&BudgetSnapshotView> for AceBudget {
    fn from(snapshot: &BudgetSnapshotView) -> Self {
        AceBudget {
            tokens_allowed: Some(snapshot.tokens_allowed),
            tokens_spent: Some(snapshot.tokens_spent),
            walltime_ms_allowed: Some(snapshot.walltime_ms_allowed),
            walltime_ms_used: Some(snapshot.walltime_ms_used),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncPointInputView {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    pub kind: SyncPointKind,
    // anchor 使用 Value 类型，避免严格的 ID 验证
    pub anchor: Value,
    // events 也使用 Value 类型，因为 DialogueEvent 包含严格的 TenantId 类型
    pub events: Value,
    pub budget: BudgetSnapshotView,
    // timeframe 在后端是 (OffsetDateTime, OffsetDateTime)，序列化为嵌套数组
    // 使用 Value 类型来接收任意 JSON 格式
    pub timeframe: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_injections: Vec<HitlInjectionView>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub context_manifest: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_cycle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collab_scope_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HitlInjectionView {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub injection_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_string_or_number"
    )]
    pub tenant_id: Option<String>,
    pub author_role: String,
    pub priority: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub submitted_at: Value,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutboxMessageView {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub event_id: String,
    // payload 使用 Value 类型，因为 AwarenessEvent 包含严格的 ID 类型
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleSnapshotView {
    pub schedule: CycleScheduleView,
    pub sync_point: SyncPointInputView,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<CycleOutcomeSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outbox: Vec<OutboxMessageView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouterDecisionView {
    pub plan: RoutePlanView,
    // decision_path 现在是 Value 类型，避免严格的 ID 验证
    pub decision_path: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected: Vec<(String, String)>,
    pub context_digest: String,
    // issued_at 是 OffsetDateTime，序列化为数组格式
    pub issued_at: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutePlanView {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    // anchor 使用 Value 类型，避免严格的 ID 验证
    pub anchor: Value,
    pub fork: AwarenessFork,
    pub decision_plan: DecisionPlan,
    pub budget: RouteBudgetEstimate,
    pub priority: f32,
    pub explain: RouteExplainView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteBudgetEstimate {
    pub tokens: u32,
    pub walltime_ms: u32,
    pub external_cost: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteExplainView {
    pub routing_seed: u64,
    pub router_digest: String,
    pub router_config_digest: String,
    #[serde(default)]
    pub indices_used: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
    #[serde(default)]
    pub diagnostics: Value,
    #[serde(default)]
    pub rejected: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HitlInjection {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub injection_id: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub cycle_id: String,
    pub priority: String,
    pub author_role: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextBundleView {
    pub anchor: ContextAnchor,
    #[serde(default)]
    pub segments: Vec<BundleSegment>,
    #[serde(default)]
    pub explain: ExplainBundle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<BundleBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_generation: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestDigestRecord {
    pub manifest_digest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cycle_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<ContextBundleView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_manifest: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextAnchor {
    pub tenant_id: i64,
    pub envelope_id: String,
    pub config_snapshot_hash: String,
    pub config_snapshot_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_class: Option<AccessClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Value>,
    pub schema_v: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<ConversationScenario>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleSegment {
    pub partition: String,
    #[serde(default)]
    pub items: Vec<BundleItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleItem {
    pub ci_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_level: Option<String>,
    pub tokens: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExplainBundle {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indices_used: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleBudget {
    pub target_tokens: u32,
    pub projected_tokens: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExplainIndices {
    pub graph: ExplainSection,
    pub context: ExplainSection,
    pub dfr: DfrExplainSection,
    pub ace: AceExplainSection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantWorkspace {
    pub tenant_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarify_strategy: Option<String>,
    #[serde(default)]
    pub pinned_sessions: Vec<WorkspaceSession>,
    #[serde(default)]
    pub recent_sessions: Vec<WorkspaceSession>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSession {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<ConversationScenario>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active_ms: Option<i64>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalGraphView {
    pub root_event_id: u64,
    #[serde(default)]
    pub nodes: Vec<CausalGraphNode>,
    #[serde(default)]
    pub edges: Vec<CausalGraphEdge>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalGraphNode {
    pub event_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<DialogueEventType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<ConversationScenario>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalGraphEdge {
    pub from: u64,
    pub to: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecallResultView {
    pub event_id: u64,
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExplainSection {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indices_used: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DfrExplainSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub router_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AceExplainSection {
    // sync_point 可以是 SyncPointKind 枚举或字符串，使用 Value 来兼容
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_point: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutboxEnvelope {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub finals: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delta_patches: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub late_receipts: Vec<Value>,
}

// ============================================================================
// 新 API 类型 - 元认知分析、自主延续、DFR、演化等
// ============================================================================

use std::collections::HashMap;

/// 时间窗口
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

/// 趋势方向
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Fluctuating,
}

/// 分页响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

// ---------------------- 元认知分析 ----------------------

/// 元认知分析查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MetacognitionAnalysisQuery {
    pub mode: Option<String>,
    pub ac_id: Option<String>,
    pub session_id: Option<String>,
    pub time_window_start: Option<i64>,
    pub time_window_end: Option<i64>,
}

/// 分析结果响应（匹配后端 metacognition.rs）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisResultResponse {
    /// 分析模式
    pub mode: String,
    /// 成功标志
    #[serde(default)]
    pub success: bool,
    /// 结果数据
    #[serde(default)]
    pub data: serde_json::Value,
    /// 摘要（字符串）
    #[serde(default)]
    pub summary: Option<String>,
    /// 洞察列表
    #[serde(default)]
    pub insights: Vec<InsightResponse>,
    /// 执行时间（毫秒）
    #[serde(default)]
    pub execution_time_ms: u64,
}

/// 洞察响应（匹配后端 InsightResponse）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InsightResponse {
    pub insight_type: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub importance: f32,
    #[serde(default)]
    pub related_entities: Vec<String>,
    #[serde(default)]
    pub suggested_actions: Vec<String>,
}

/// 因果链查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CausalChainQuery {
    pub direction: Option<String>,
    pub max_depth: Option<u32>,
}

/// 因果链响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalChainResponse {
    pub event_id: String,
    pub nodes: Vec<CausalNodeResponse>,
    pub edges: Vec<CausalEdgeResponse>,
    pub root_cause: Option<CausalNodeResponse>,
    pub impact_scope: ImpactScope,
}

/// 因果节点响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalNodeResponse {
    pub event_id: String,
    pub event_type: String,
    pub occurred_at_ms: i64,
    pub depth: i32,
    pub summary: Option<String>,
}

/// 因果边响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalEdgeResponse {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub strength: f32,
    pub delay_ms: Option<i64>,
}

/// 影响范围
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImpactScope {
    pub total_affected_events: u32,
    pub max_depth: u32,
    pub affected_sessions: Vec<String>,
    pub affected_actors: Vec<String>,
}

/// 性能剖析响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceProfileResponse {
    pub ac_id: String,
    pub latency_breakdown: LatencyBreakdown,
    pub bottlenecks: Vec<BottleneckInfo>,
    pub resource_usage: ResourceUsageInfo,
    pub comparison: Option<PerformanceComparison>,
}

/// 延迟分解
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatencyBreakdown {
    pub total_ms: u64,
    pub llm_ms: u64,
    pub tool_execution_ms: u64,
    pub decision_routing_ms: u64,
    pub context_assembly_ms: u64,
    pub other_ms: u64,
}

/// 瓶颈信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BottleneckInfo {
    pub component: String,
    pub severity: String,
    pub latency_ms: u64,
    pub percentage: f32,
    pub suggestion: String,
}

/// 资源使用信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceUsageInfo {
    pub tokens_used: u32,
    pub tokens_limit: u32,
    pub cost_usd: f64,
    pub memory_mb: Option<f32>,
}

/// 性能比较
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub vs_average: f32,
    pub vs_best: f32,
    pub percentile: u8,
}

/// 决策审计响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionAuditResponse {
    pub decision_id: String,
    pub decision_path: String,
    pub rationale: DecisionRationaleAudit,
    pub alternatives_considered: Vec<AlternativeAudit>,
    pub confidence_factors: Vec<ConfidenceFactor>,
    pub risks: Vec<RiskAssessment>,
    pub outcome: Option<DecisionOutcomeAudit>,
}

/// 决策理由审计
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRationaleAudit {
    pub primary_reason: String,
    pub supporting_evidence: Vec<String>,
    pub context_factors: Vec<String>,
}

/// 备选审计
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlternativeAudit {
    pub path: String,
    pub score: f32,
    pub rejection_reason: String,
}

/// 置信度因素
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfidenceFactor {
    pub factor_name: String,
    pub value: f32,
    pub weight: f32,
    pub contribution: f32,
}

/// 风险评估
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_type: String,
    pub severity: String,
    pub probability: f32,
    pub description: String,
    pub mitigation: Option<String>,
}

/// 决策结果审计
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionOutcomeAudit {
    pub was_successful: bool,
    pub actual_path: String,
    pub user_feedback: Option<String>,
}

/// 模式检测查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PatternDetectionQuery {
    pub pattern_types: Option<String>,
    pub session_id: Option<String>,
    pub limit: Option<u32>,
}

/// 模式检测响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternDetectionResponse {
    pub patterns: Vec<DetectedPattern>,
    pub anomalies: Vec<AnomalyInfo>,
    pub summary: PatternSummary,
}

/// 检测到的模式
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern_id: String,
    pub pattern_type: String,
    pub description: String,
    pub frequency: u32,
    pub confidence: f32,
    pub examples: Vec<String>,
    pub first_seen_at_ms: i64,
    pub last_seen_at_ms: i64,
}

/// 异常信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnomalyInfo {
    pub anomaly_id: String,
    pub anomaly_type: String,
    pub severity: String,
    pub description: String,
    pub detected_at_ms: i64,
    pub related_events: Vec<String>,
}

/// 模式摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternSummary {
    pub total_patterns: u32,
    pub total_anomalies: u32,
    pub dominant_pattern_type: Option<String>,
    pub health_score: f32,
}

// ---------------------- 自主延续 ----------------------

/// 启动自主延续请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartAutonomousRequest {
    pub session_id: String,
    #[serde(default)]
    pub agenda_items: Vec<AgendaItemInput>,
    #[serde(default)]
    pub config: AutonomousConfig,
}

/// 议程项输入
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgendaItemInput {
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
}

fn default_priority() -> u8 {
    5
}

/// 自主延续配置
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AutonomousConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_consecutive_ac: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_threshold_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_idle_count: Option<u32>,
}

/// 自主会话响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutonomousSessionResponse {
    pub orchestration_id: String,
    pub session_id: String,
    pub status: AutonomousStatus,
    pub created_at_ms: i64,
}

/// 会话列表响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutonomousSessionListResponse {
    pub sessions: Vec<AutonomousSessionSummary>,
    pub total: usize,
}

/// 会话摘要信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutonomousSessionSummary {
    pub orchestration_id: String,
    pub session_id: String,
    pub status: AutonomousStatus,
    pub cycles_executed: u32,
    pub max_cycles: u32,
    pub total_cost: f64,
    pub created_at_ms: i64,
    pub agenda_count: usize,
    /// 总 token 数
    #[serde(default)]
    pub total_tokens: u64,
    /// 当前议程描述
    #[serde(default)]
    pub current_agenda: Option<String>,
    /// 空转次数
    #[serde(default)]
    pub idle_count: u32,
    /// 最大空转次数
    #[serde(default)]
    pub max_idle: u32,
    /// 最近执行日志
    #[serde(default)]
    pub recent_logs: Vec<ExecutionLogEntry>,
}

/// 执行日志条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionLogEntry {
    /// 周期 ID
    pub cycle_id: u64,
    /// 时间戳
    pub timestamp_ms: i64,
    /// 状态
    pub status: String,
    /// 议程项描述
    #[serde(default)]
    pub agenda_item: Option<String>,
    /// 使用的 token 数
    pub tokens_used: u64,
    /// 成本
    pub cost: f64,
    /// 消息
    pub message: String,
}

/// 自主状态枚举
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousStatus {
    Starting,
    Running,
    Paused,
    Stopping,
    Completed,
    Terminated,
    Failed,
}

/// 自主会话状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutonomousSessionState {
    pub orchestration_id: String,
    pub session_id: String,
    pub status: AutonomousStatus,
    pub mode: String,
    pub agenda_queue: Vec<AgendaItemResponse>,
    pub termination_conditions: TerminationConditions,
    pub ac_stats: AcStats,
    pub created_at_ms: i64,
    pub last_activity_at_ms: i64,
}

/// 议程项响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgendaItemResponse {
    pub item_id: String,
    pub description: String,
    pub priority: u8,
    pub status: AgendaItemStatus,
    pub created_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
}

/// 议程项状态
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgendaItemStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Failed,
}

/// 终止条件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminationConditions {
    pub agenda_exhausted: bool,
    pub idle_count: u32,
    pub max_idle: u32,
    pub cost_spent: f64,
    pub cost_limit: f64,
    pub ac_count: u32,
    pub max_ac: u32,
    pub external_interrupt: bool,
}

/// AC 统计
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcStats {
    pub total_count: u32,
    pub completed_count: u32,
    pub failed_count: u32,
    pub average_duration_ms: u64,
    pub total_cost: f64,
    pub total_tokens: u64,
}

/// 停止自主延续请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StopAutonomousRequest {
    #[serde(default = "default_reason")]
    pub reason: String,
}

fn default_reason() -> String {
    "user_requested".to_string()
}

/// 终止结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminationResult {
    pub orchestration_id: String,
    pub status: AutonomousStatus,
    pub reason: TerminationReason,
    pub terminated_at_ms: i64,
    pub summary: SessionSummary,
}

/// 终止原因
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    AgendaExhausted,
    MaxIdleReached,
    CostLimitExceeded,
    MaxAcReached,
    IdleTimeout,
    ExternalInterrupt,
    UserRequested,
    Error,
}

/// 会话摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub duration_ms: u64,
    pub ac_count: u32,
    pub agenda_completed: u32,
    pub agenda_total: u32,
    pub total_cost: f64,
    pub total_tokens: u64,
}

/// 场景栈状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioStackState {
    pub stack: Vec<ScenarioStackEntry>,
    pub depth: u32,
    pub is_balanced: bool,
    pub path: String,
    pub stats: ScenarioStackStats,
}

/// 场景栈条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioStackEntry {
    pub scenario: String,
    pub entered_at_ms: i64,
    pub trigger_event_id: String,
    pub depth: u32,
}

/// 场景栈统计
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioStackStats {
    pub total_pushes: u32,
    pub total_pops: u32,
    pub max_depth_reached: u32,
    pub scenario_counts: HashMap<String, u32>,
}

/// 延续信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuationInfo {
    pub signal: ContinuationSignal,
    pub driver: ContinuationDriver,
    pub next_action: Option<NextActionInfo>,
    pub termination_check: TerminationCheckResult,
}

/// 延续信号
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationSignal {
    Stop,
    ContinueWithAgenda,
    ContinueWithExtension,
}

/// 延续驱动
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationDriver {
    AgendaDriven,
    DiscourseExtension,
    Hybrid,
}

/// 下一步动作信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NextActionInfo {
    pub action_type: String,
    pub description: String,
    pub target_id: Option<String>,
    pub priority: u8,
}

/// 终止检查结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminationCheckResult {
    pub should_terminate: bool,
    pub reasons: Vec<String>,
    pub conditions_met: Vec<String>,
}

// ---------------------- 版本链与图谱 ----------------------

/// 版本链查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VersionChainQuery {
    pub entity_type: Option<String>,
}

/// 版本链摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionChainSummary {
    pub entity_id: String,
    pub entity_type: String,
    pub root_version: VersionEntry,
    pub current_version: VersionEntry,
    pub history: Vec<VersionEntry>,
    pub conflicts: Vec<VersionConflict>,
    pub total_versions: u32,
}

/// 版本条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionEntry {
    pub version_id: String,
    pub version_number: u32,
    pub created_at_ms: i64,
    pub created_by: String,
    pub supersedes: Option<String>,
    pub description: Option<String>,
    pub content_hash: Option<String>,
}

/// 版本冲突
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionConflict {
    pub conflict_type: String,
    pub description: String,
    pub involved_versions: Vec<String>,
    pub detected_at_ms: i64,
}

/// 图节点详情
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNodeDetail {
    pub node_id: String,
    pub node_type: String,
    pub properties: HashMap<String, Value>,
    pub incoming_edges: Vec<GraphEdgeRef>,
    pub outgoing_edges: Vec<GraphEdgeRef>,
    pub created_at_ms: i64,
    pub updated_at_ms: Option<i64>,
}

/// 图边引用
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdgeRef {
    pub edge_id: String,
    pub edge_type: String,
    pub other_node_id: String,
    pub other_node_type: String,
    pub weight: Option<f32>,
}

/// 图边查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphEdgesQuery {
    pub edge_type: Option<String>,
    pub from_node: Option<String>,
    pub to_node: Option<String>,
    pub limit: Option<u32>,
}

/// 图边详情
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdgeDetail {
    pub edge_id: String,
    pub edge_type: String,
    pub edge_family: String,
    pub from_node: NodeRef,
    pub to_node: NodeRef,
    pub weight: Option<f32>,
    pub properties: HashMap<String, Value>,
    pub created_at_ms: i64,
}

/// 节点引用
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeRef {
    pub node_id: String,
    pub node_type: String,
    pub label: Option<String>,
}

// ---------------------- DFR 决策增强 ----------------------

/// 决策详情
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionDetail {
    pub decision_id: String,
    pub cycle_id: String,
    pub path: String,
    pub sticky: Option<StickyDecisionInfo>,
    pub alternatives_considered: Vec<AlternativeInfo>,
    pub rationale: DecisionRationaleInfo,
    pub fingerprint: FingerprintInfo,
    pub decided_at_ms: i64,
    pub outcome: Option<DecisionOutcome>,
}

/// 粘性决策信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StickyDecisionInfo {
    pub duration_type: String,
    pub refinement_allowed: bool,
    pub fallback_path: Option<String>,
    pub remaining_turns: Option<u32>,
    pub reason: String,
}

/// 备选方案信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlternativeInfo {
    pub path: String,
    pub score: f32,
    pub rejection_reason: String,
    pub score_delta: Option<f32>,
    pub would_have_required: Vec<String>,
    pub risk_assessment: Option<f32>,
}

/// 决策理由信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRationaleInfo {
    pub primary_reason: String,
    pub supporting_evidence: Vec<EvidenceInfo>,
    pub confidence_factors: Vec<ConfidenceFactorInfo>,
    pub potential_risks: Vec<RiskInfo>,
    pub overall_confidence: f32,
}

/// 证据信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvidenceInfo {
    pub evidence_type: String,
    pub source: String,
    pub relevance: f32,
    pub description: String,
}

/// 置信度因素信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfidenceFactorInfo {
    pub factor_name: String,
    pub value: f32,
    pub weight: f32,
    pub contribution: f32,
}

/// 风险信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskInfo {
    pub risk_type: String,
    pub severity: String,
    pub probability: f32,
    pub description: String,
    pub mitigation: Option<String>,
}

/// 指纹信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintInfo {
    pub fingerprint_id: String,
    pub hash: String,
    pub features: Vec<FeatureInfo>,
    pub created_at_ms: i64,
}

/// 特征信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureInfo {
    pub name: String,
    pub feature_type: String,
    pub value: Value,
    pub weight: f32,
}

/// 决策结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub was_successful: bool,
    pub actual_path_taken: String,
    pub duration_ms: u64,
    pub cost: f64,
    pub user_feedback: Option<String>,
}

/// 指纹查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FingerprintQuery {
    pub session_id: Option<String>,
    pub similarity_threshold: Option<f32>,
    pub limit: Option<u32>,
}

/// 指纹列表响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintListResponse {
    pub fingerprints: Vec<FingerprintSummary>,
    pub total: u32,
    pub matches: Option<Vec<FingerprintMatch>>,
}

/// 指纹摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintSummary {
    pub fingerprint_id: String,
    pub hash: String,
    pub decision_path: String,
    pub scenario: String,
    pub usage_count: u32,
    pub success_rate: f32,
    pub created_at_ms: i64,
    pub last_used_at_ms: i64,
}

/// 指纹匹配请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintMatchRequest {
    pub context: DecisionContextInput,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    #[serde(default = "default_max_matches")]
    pub max_matches: u32,
}

fn default_threshold() -> f32 {
    0.7
}

fn default_max_matches() -> u32 {
    10
}

/// 决策上下文输入
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionContextInput {
    pub scenario: String,
    pub intent_signals: Vec<String>,
    pub context_features: HashMap<String, Value>,
    #[serde(default)]
    pub available_tools: Vec<String>,
}

/// 指纹匹配
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintMatch {
    pub fingerprint_id: String,
    pub similarity: f32,
    pub decision_path: String,
    pub historical_success_rate: f32,
    pub usage_count: u32,
}

/// 指纹匹配结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FingerprintMatchResult {
    pub matches: Vec<FingerprintMatch>,
    pub best_match: Option<FingerprintMatch>,
    pub confidence: f32,
    pub recommendation: Option<String>,
}

// ---------------------- SurrealDB 原生功能 ----------------------

/// 向量搜索请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorSearchRequest {
    pub query_embedding: Option<Vec<f32>>,
    pub query_text: Option<String>,
    #[serde(default = "default_top_k")]
    pub top_k: u16,
    #[serde(default = "default_search_threshold")]
    pub threshold: f32,
    #[serde(default = "default_metric")]
    pub metric: String,
    #[serde(default)]
    pub filters: VectorSearchFilters,
}

fn default_top_k() -> u16 {
    10
}

fn default_search_threshold() -> f32 {
    0.7
}

fn default_metric() -> String {
    "cosine".to_string()
}

/// 向量搜索过滤
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VectorSearchFilters {
    pub journey_id: Option<String>,
    pub session_id: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub time_window: Option<TimeWindow>,
}

/// 向量搜索响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorSearchResponse {
    pub results: Vec<VectorSearchResult>,
    pub total_found: u32,
    pub search_time_ms: u64,
    pub metric_used: String,
}

/// 向量搜索结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub chunk_id: String,
    pub score: f32,
    pub content: Option<String>,
    pub metadata: HashMap<String, Value>,
}

/// 内容索引请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexContentRequest {
    /// 要索引的文本内容
    pub content: String,
    /// 内容来源类型 (如 "document", "note", "manual")
    pub source_type: String,
    /// 内容来源 ID
    pub source_id: String,
    /// 关联的旅程 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journey_id: Option<String>,
    /// 关联的会话 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 额外元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// 内容索引响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexContentResponse {
    /// 生成的 chunk ID
    pub chunk_id: String,
    /// 索引状态
    pub status: String,
    /// 内容长度
    pub content_length: usize,
    /// 嵌入维度
    pub embedding_dim: u32,
}

/// 时序聚合查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimeSeriesAggregateQuery {
    pub table: Option<String>,
    pub timestamp_field: Option<String>,
    pub granularity: Option<String>,
    pub aggregate: Option<String>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub group_by: Option<String>,
}

/// 时序聚合响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesAggregateResponse {
    pub buckets: Vec<TimeSeriesBucket>,
    pub summary: TimeSeriesSummary,
    #[serde(default)]
    pub trend: Option<TrendResult>,
}

/// 时序桶
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesBucket {
    pub timestamp_ms: i64,
    pub value: f64,
    #[serde(default)]
    pub count: Option<u64>,
    #[serde(default)]
    pub group: Option<String>,
}

/// 时序摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesSummary {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub sum: f64,
    pub count: u64,
    #[serde(default)]
    pub stddev: Option<f64>,
}

/// 趋势结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrendResult {
    pub direction: String,
    pub slope: f64,
    pub r_squared: f64,
    pub confidence: f64,
}

/// 时序趋势查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimeSeriesTrendQuery {
    pub table: Option<String>,
    pub value_field: Option<String>,
    pub timestamp_field: Option<String>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub granularity: Option<String>,
}

/// 时序趋势响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesTrendResponse {
    pub trend: TrendResultInfo,
    pub data_points: Vec<TrendDataPoint>,
    pub forecast: Option<Vec<TrendDataPoint>>,
}

/// 趋势结果信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrendResultInfo {
    pub direction: TrendDirection,
    pub slope: f64,
    pub r_squared: f64,
    pub percent_change: f64,
}

/// 趋势数据点
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub timestamp_ms: i64,
    pub value: f64,
    pub is_forecast: bool,
}

/// 时序模式查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimeSeriesPatternsQuery {
    pub table: Option<String>,
    pub value_field: Option<String>,
    pub pattern_types: Option<String>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub sensitivity: Option<f32>,
}

/// 时序模式响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesPatternsResponse {
    pub patterns: Vec<TimeSeriesPattern>,
    pub summary: PatternAnalysisSummary,
}

/// 时序模式
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesPattern {
    pub pattern_type: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub magnitude: f64,
    pub confidence: f32,
    pub description: String,
}

/// 模式分析摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternAnalysisSummary {
    pub total_patterns: u32,
    pub spikes_count: u32,
    pub drops_count: u32,
    pub anomalies_count: u32,
    pub overall_stability: f32,
}

/// 实时订阅请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealtimeSubscribeRequest {
    pub tables: Vec<String>,
    #[serde(default)]
    pub filters: SubscriptionFilters,
    #[serde(default)]
    pub config: SubscriptionConfig,
}

/// 订阅过滤
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SubscriptionFilters {
    pub session_id: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub actors: Option<Vec<String>>,
}

/// 订阅配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionConfig {
    #[serde(default = "default_heartbeat")]
    pub heartbeat_ms: u32,
    #[serde(default = "default_buffer")]
    pub max_buffer: u32,
    #[serde(default = "default_ttl")]
    pub ttl_ms: u64,
}

fn default_heartbeat() -> u32 {
    30000
}

fn default_buffer() -> u32 {
    1000
}

fn default_ttl() -> u64 {
    3600000
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            heartbeat_ms: default_heartbeat(),
            max_buffer: default_buffer(),
            ttl_ms: default_ttl(),
        }
    }
}

/// 订阅响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub subscription_id: String,
    pub status: String,
    pub tables: Vec<String>,
    pub stream_url: String,
    pub created_at_ms: i64,
    pub expires_at_ms: i64,
}

/// 取消订阅响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnsubscribeResponse {
    pub subscription_id: String,
    pub status: String,
    pub cancelled_at_ms: i64,
}

/// 订阅列表响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionListResponse {
    pub subscriptions: Vec<SubscriptionInfo>,
    pub total: u32,
}

/// 订阅信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    pub subscription_id: String,
    pub tables: Vec<String>,
    pub status: String,
    pub created_at_ms: i64,
    pub expires_at_ms: i64,
    pub events_received: u64,
    pub last_event_at_ms: Option<i64>,
}

// ---------------------- 演化事件 ----------------------

/// 群组演化查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GroupEvolutionQuery {
    pub group_id: Option<String>,
    pub event_types: Option<String>,
    pub time_window_start: Option<i64>,
    pub time_window_end: Option<i64>,
    pub limit: Option<u32>,
}

/// 群组演化列表响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupEvolutionListResponse {
    pub events: Vec<GroupEvolutionEvent>,
    pub summary: GroupEvolutionSummary,
    pub total: u32,
}

/// 群组演化事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupEvolutionEvent {
    pub event_id: String,
    pub event_type: GroupEvolutionEventType,
    pub group_id: String,
    pub group_name: Option<String>,
    pub occurred_at_ms: i64,
    pub details: GroupEvolutionDetails,
    pub actor: String,
}

/// 群组演化事件类型
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupEvolutionEventType {
    GroupCreated,
    GroupMemberJoined,
    GroupMemberLeft,
    GroupPermissionChanged,
    GroupDisbanded,
    GroupRenamed,
}

/// 群组演化详情
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupEvolutionDetails {
    pub member_id: Option<String>,
    pub member_name: Option<String>,
    pub old_name: Option<String>,
    pub new_name: Option<String>,
    pub permission_changes: Option<Vec<PermissionChange>>,
    pub reason: Option<String>,
}

/// 权限变更
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionChange {
    pub permission: String,
    pub old_value: bool,
    pub new_value: bool,
}

/// 群组演化摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupEvolutionSummary {
    pub total_events: u32,
    pub events_by_type: HashMap<String, u32>,
    pub active_groups: u32,
    pub total_members: u32,
}

/// AI 演化查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AiEvolutionQuery {
    pub dimension: Option<String>,
    pub time_window_start: Option<i64>,
    pub time_window_end: Option<i64>,
    pub limit: Option<u32>,
}

/// AI 演化列表响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiEvolutionListResponse {
    pub ai_id: String,
    pub personality_adjustments: Vec<PersonalityAdjustment>,
    pub skill_updates: Vec<SkillUpdate>,
    pub growth_milestones: Vec<GrowthMilestone>,
    pub summary: AiEvolutionSummary,
}

/// 人格调整
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonalityAdjustment {
    pub adjustment_id: String,
    pub dimension: String,
    pub old_value: f32,
    pub new_value: f32,
    pub reason: String,
    pub occurred_at_ms: i64,
    pub trigger_event_id: Option<String>,
}

/// 技能更新
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillUpdate {
    pub update_id: String,
    pub skill_name: String,
    pub update_type: SkillUpdateType,
    pub old_level: Option<f32>,
    pub new_level: f32,
    pub occurred_at_ms: i64,
    pub evidence: Vec<String>,
}

/// 技能更新类型
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillUpdateType {
    Acquired,
    Improved,
    Degraded,
    Mastered,
}

/// 成长里程碑
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrowthMilestone {
    pub milestone_id: String,
    pub milestone_type: String,
    pub description: String,
    pub achieved_at_ms: i64,
    pub significance: String,
}

/// AI 演化摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiEvolutionSummary {
    pub total_adjustments: u32,
    pub total_skill_updates: u32,
    pub total_milestones: u32,
    pub growth_rate: f32,
    pub stability_score: f32,
}

/// 关系演化查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RelationshipEvolutionQuery {
    pub subject_a: Option<String>,
    pub subject_b: Option<String>,
    pub relationship_type: Option<String>,
    pub time_window_start: Option<i64>,
    pub time_window_end: Option<i64>,
    pub limit: Option<u32>,
}

/// 关系演化列表响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipEvolutionListResponse {
    pub events: Vec<RelationshipEvolutionEvent>,
    pub summary: RelationshipEvolutionSummary,
    pub total: u32,
}

/// 关系演化事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipEvolutionEvent {
    pub event_id: String,
    pub event_type: RelationshipEvolutionEventType,
    pub subject_a: SubjectRef,
    pub subject_b: SubjectRef,
    pub occurred_at_ms: i64,
    pub details: RelationshipEvolutionDetails,
}

/// 关系演化事件类型
#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipEvolutionEventType {
    RelationshipEstablished,
    RelationshipStrengthened,
    RelationshipWeakened,
    RelationshipEnded,
    TrustIncreased,
    TrustDecreased,
    AiRelationshipAdjusted,
}

/// 主体引用
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubjectRef {
    pub subject_type: String,
    pub subject_id: String,
    pub name: Option<String>,
}

/// 关系演化详情
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipEvolutionDetails {
    pub relationship_type: Option<String>,
    pub old_strength: Option<f32>,
    pub new_strength: Option<f32>,
    pub old_trust: Option<f32>,
    pub new_trust: Option<f32>,
    pub trigger_event_id: Option<String>,
    pub reason: Option<String>,
}

/// 关系演化摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipEvolutionSummary {
    pub total_events: u32,
    pub events_by_type: HashMap<String, u32>,
    pub average_relationship_strength: f32,
    pub average_trust_level: f32,
}

// ---------------------- 额外 UI 所需类型 ----------------------

/// 因果节点 (UI 使用)
pub type CausalNode = CausalNodeResponse;

/// 订阅请求 (简化版)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub table: String,
    pub filter: Option<String>,
    pub fields: Option<Vec<String>>,
}

/// 演化时间线查询
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvolutionTimelineQuery {
    pub event_types: Option<Vec<String>>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub limit: Option<u32>,
}

/// 演化时间线响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvolutionTimelineResponse {
    pub events: Vec<EvolutionTimelineEvent>,
    pub total: u32,
}

/// 演化时间线事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvolutionTimelineEvent {
    pub event_id: String,
    pub event_type: String,
    pub occurred_at: String,
    pub description: String,
    pub significance: f32,
}

/// 版本差异
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionDiff {
    pub entity_id: String,
    pub entity_type: String,
    pub from_version: u32,
    pub to_version: u32,
    pub changes: Vec<VersionChange>,
    pub additions: u32,
    pub modifications: u32,
    pub deletions: u32,
}

/// 版本变更项
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionChange {
    pub field: String,
    pub change_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// AI 演化事件 (UI 使用的简化版)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiEvolutionEvent {
    pub event_id: String,
    pub event_type: String,
    pub from_version: u32,
    pub to_version: u32,
    pub occurred_at: String,
    pub description: String,
    pub changes: Vec<String>,
}

/// AI 演化列表响应扩展字段
impl AiEvolutionListResponse {
    pub fn ai_name(&self) -> &str {
        &self.ai_id
    }

    pub fn current_version(&self) -> u32 {
        self.growth_milestones
            .last()
            .map(|_| self.total_events())
            .unwrap_or(1)
    }

    pub fn total_events(&self) -> u32 {
        self.personality_adjustments.len() as u32
            + self.skill_updates.len() as u32
            + self.growth_milestones.len() as u32
    }

    pub fn events(&self) -> Vec<AiEvolutionEvent> {
        let mut events = Vec::new();

        for adj in &self.personality_adjustments {
            events.push(AiEvolutionEvent {
                event_id: adj.adjustment_id.clone(),
                event_type: "personality_shift".to_string(),
                from_version: 0,
                to_version: 1,
                occurred_at: format!("{}ms", adj.occurred_at_ms),
                description: format!("{}: {} -> {}", adj.dimension, adj.old_value, adj.new_value),
                changes: vec![adj.reason.clone()],
            });
        }

        for update in &self.skill_updates {
            events.push(AiEvolutionEvent {
                event_id: update.update_id.clone(),
                event_type: format!("{:?}", update.update_type).to_lowercase(),
                from_version: update.old_level.map(|l| l as u32).unwrap_or(0),
                to_version: update.new_level as u32,
                occurred_at: format!("{}ms", update.occurred_at_ms),
                description: format!("技能 {} 更新", update.skill_name),
                changes: update.evidence.clone(),
            });
        }

        events
    }
}

/// 群体演化事件 (UI 使用的简化版)
impl GroupEvolutionEvent {
    pub fn event_type_str(&self) -> String {
        format!("{:?}", self.event_type).to_lowercase()
    }

    pub fn occurred_at_str(&self) -> String {
        format!("{}ms", self.occurred_at_ms)
    }

    pub fn description(&self) -> String {
        match self.event_type {
            GroupEvolutionEventType::GroupCreated => "群组已创建".to_string(),
            GroupEvolutionEventType::GroupMemberJoined => {
                format!("成员 {} 加入", self.details.member_name.as_deref().unwrap_or("未知"))
            }
            GroupEvolutionEventType::GroupMemberLeft => {
                format!("成员 {} 离开", self.details.member_name.as_deref().unwrap_or("未知"))
            }
            GroupEvolutionEventType::GroupPermissionChanged => "权限已变更".to_string(),
            GroupEvolutionEventType::GroupDisbanded => "群组已解散".to_string(),
            GroupEvolutionEventType::GroupRenamed => {
                format!(
                    "群组重命名: {} -> {}",
                    self.details.old_name.as_deref().unwrap_or(""),
                    self.details.new_name.as_deref().unwrap_or("")
                )
            }
        }
    }

    pub fn significance(&self) -> f32 {
        match self.event_type {
            GroupEvolutionEventType::GroupCreated | GroupEvolutionEventType::GroupDisbanded => 1.0,
            GroupEvolutionEventType::GroupMemberJoined | GroupEvolutionEventType::GroupMemberLeft => 0.5,
            GroupEvolutionEventType::GroupPermissionChanged => 0.7,
            GroupEvolutionEventType::GroupRenamed => 0.3,
        }
    }

    pub fn participants(&self) -> Vec<String> {
        let mut participants = vec![self.actor.clone()];
        if let Some(ref member) = self.details.member_id {
            participants.push(member.clone());
        }
        participants
    }

    pub fn impact(&self) -> HashMap<String, String> {
        let mut impact = HashMap::new();
        if let Some(ref reason) = self.details.reason {
            impact.insert("原因".to_string(), reason.clone());
        }
        impact
    }
}

/// 关系演化事件 (UI 使用的简化版)
impl RelationshipEvolutionEvent {
    pub fn event_type_str(&self) -> String {
        format!("{:?}", self.event_type).to_lowercase()
    }

    pub fn occurred_at_str(&self) -> String {
        format!("{}ms", self.occurred_at_ms)
    }

    pub fn entity_a(&self) -> String {
        self.subject_a.name.clone().unwrap_or_else(|| self.subject_a.subject_id.clone())
    }

    pub fn entity_b(&self) -> String {
        self.subject_b.name.clone().unwrap_or_else(|| self.subject_b.subject_id.clone())
    }

    pub fn relationship_type(&self) -> String {
        self.details.relationship_type.clone().unwrap_or_else(|| "未知".to_string())
    }

    pub fn from_strength(&self) -> f32 {
        self.details.old_strength.unwrap_or(0.0)
    }

    pub fn to_strength(&self) -> f32 {
        self.details.new_strength.unwrap_or(0.0)
    }

    pub fn reason(&self) -> Option<String> {
        self.details.reason.clone()
    }
}
