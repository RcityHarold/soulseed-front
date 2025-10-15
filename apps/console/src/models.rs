use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use soulseed_agi_core_models::{
    AccessClass, AwarenessAnchor, AwarenessDegradationReason, AwarenessEvent, AwarenessEventType,
    ConversationScenario, DecisionPath, DecisionPlan, DialogueEvent, DialogueEventType,
    SyncPointKind, SyncPointReport,
};

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AceCycleStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
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
    pub cycle_id: String,
    pub lane: AceLane,
    pub status: AceCycleStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<AwarenessAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<AceBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_sync_point: Option<SyncPointReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_injections: Vec<HitlInjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_path: Option<DecisionPath>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HitlInjection {
    pub injection_id: String,
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
    pub explain: ExplainBundle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<BundleBudget>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_point: Option<SyncPointKind>,
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
