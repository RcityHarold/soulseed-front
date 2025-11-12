use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use crate::models::{
    AceCycleStatus, AceCycleSummary, AceLane, AwarenessEvent, CausalGraphView, ContextBundleView,
    ConversationScenario, CycleOutcomeSummary, CycleSnapshotView, DialogueEvent, ExplainIndices,
    HitlInjection, ManifestDigestRecord, OutboxMessageView, RecallResultView, TenantWorkspace,
    WorkspaceSession,
};

pub type AppSignal = Signal<AppState>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimelineQuery {
    #[serde(default = "TimelineQuery::default_limit")]
    pub limit: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<ConversationScenario>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl TimelineQuery {
    const DEFAULT_LIMIT: usize = 50;

    const fn default_limit() -> usize {
        Self::DEFAULT_LIMIT
    }

    pub fn reset_cursor(&mut self) {
        self.cursor = None;
        self.since_ms = None;
        self.until_ms = None;
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimelineState {
    pub events: Vec<DialogueEvent>,
    pub awareness: Vec<AwarenessEvent>,
    pub next_cursor: Option<String>,
    pub is_loading: bool,
    pub error: Option<String>,
    pub query: TimelineQuery,
    #[serde(default)]
    pub filters: TimelineFilters,
    #[serde(default)]
    pub tags: HashMap<u64, Vec<String>>,
}

impl TimelineState {
    pub fn clear(&mut self) {
        self.events.clear();
        self.awareness.clear();
        self.next_cursor = None;
        self.error = None;
        self.is_loading = false;
        self.query.reset_cursor();
        self.tags.clear();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimelineFilters {
    #[serde(default)]
    pub participant_roles: BTreeSet<String>,
    #[serde(default)]
    pub access_classes: BTreeSet<String>,
    #[serde(default)]
    pub degradation_reasons: BTreeSet<String>,
    #[serde(default)]
    pub awareness_types: BTreeSet<String>,
    #[serde(default)]
    pub router_digests: BTreeSet<String>,
    #[serde(default)]
    pub query_hashes: BTreeSet<String>,
}

impl TimelineFilters {
    pub fn clear(&mut self) {
        self.participant_roles.clear();
        self.access_classes.clear();
        self.degradation_reasons.clear();
        self.awareness_types.clear();
        self.router_digests.clear();
        self.query_hashes.clear();
    }

    pub fn toggle_participant_role(&mut self, role: &str) {
        toggle_value(&mut self.participant_roles, role);
    }

    pub fn toggle_access_class(&mut self, access_class: &str) {
        toggle_value(&mut self.access_classes, access_class);
    }

    pub fn toggle_degradation_reason(&mut self, reason: &str) {
        toggle_value(&mut self.degradation_reasons, reason);
    }

    pub fn toggle_awareness_type(&mut self, event_type: &str) {
        toggle_value(&mut self.awareness_types, event_type);
    }

    pub fn toggle_router_digest(&mut self, digest: &str) {
        toggle_value(&mut self.router_digests, digest);
    }

    pub fn toggle_query_hash(&mut self, hash: &str) {
        toggle_value(&mut self.query_hashes, hash);
    }

    pub fn matches_event(&self, event: &DialogueEvent) -> bool {
        if !self.participant_roles.is_empty() {
            let mut matched = false;
            for participant in &event.participants {
                if let Some(role) = participant.role.as_ref() {
                    if self
                        .participant_roles
                        .contains(&normalize_filter_value(role))
                    {
                        matched = true;
                        break;
                    }
                }
            }
            if !matched {
                return false;
            }
        }

        if !self.access_classes.is_empty() {
            let access_value = normalize_filter_value(&format!("{:?}", event.access_class));
            if !self.access_classes.contains(&access_value) {
                return false;
            }
        }

        if !self.degradation_reasons.is_empty() {
            let degradation = extract_event_degradation(event);
            if !degradation
                .map(|value| self.degradation_reasons.contains(&value))
                .unwrap_or(false)
            {
                return false;
            }
        }

        if !self.router_digests.is_empty() || !self.query_hashes.is_empty() {
            let info = extract_router_fields_from_event(event);
            if !self.router_digests.is_empty() {
                let matches = info
                    .digest
                    .as_ref()
                    .map(|value| {
                        let normalized = normalize_filter_value(value);
                        self.router_digests.contains(normalized.as_str())
                    })
                    .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
            if !self.query_hashes.is_empty() {
                let matches = info
                    .query_hash
                    .as_ref()
                    .map(|value| {
                        let normalized = normalize_filter_value(value);
                        self.query_hashes.contains(normalized.as_str())
                    })
                    .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
        }

        true
    }

    pub fn matches_awareness(&self, awareness: &AwarenessEvent) -> bool {
        if !self.awareness_types.is_empty() {
            let event_type = normalize_filter_value(&format!("{:?}", awareness.event_type));
            if !self.awareness_types.contains(&event_type) {
                return false;
            }
        }

        if !self.degradation_reasons.is_empty() {
            let degradation = awareness
                .degradation_reason
                .as_ref()
                .map(|reason| normalize_filter_value(&format!("{:?}", reason)));
            if !degradation
                .map(|value| self.degradation_reasons.contains(&value))
                .unwrap_or(false)
            {
                return false;
            }
        }

        if !self.router_digests.is_empty() || !self.query_hashes.is_empty() {
            let info = extract_router_fields_from_value(&awareness.payload);
            if !self.router_digests.is_empty() {
                let matches = info
                    .digest
                    .as_ref()
                    .map(|value| {
                        let normalized = normalize_filter_value(value);
                        self.router_digests.contains(normalized.as_str())
                    })
                    .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
            if !self.query_hashes.is_empty() {
                let matches = info
                    .query_hash
                    .as_ref()
                    .map(|value| {
                        let normalized = normalize_filter_value(value);
                        self.query_hashes.contains(normalized.as_str())
                    })
                    .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
        }

        true
    }

    pub fn is_empty(&self) -> bool {
        self.participant_roles.is_empty()
            && self.access_classes.is_empty()
            && self.degradation_reasons.is_empty()
            && self.awareness_types.is_empty()
            && self.router_digests.is_empty()
            && self.query_hashes.is_empty()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContextState {
    pub bundle: Option<ContextBundleView>,
    pub explain_indices: Option<ExplainIndices>,
    pub is_loading: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub manifest_history: Vec<ManifestDigestRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_manifest_digest: Option<String>,
}
impl ContextState {
    pub fn upsert_manifest_entry(&mut self, mut record: ManifestDigestRecord) {
        if record.manifest_digest.is_empty() {
            return;
        }

        let digest_value = record.manifest_digest.clone();

        if let Some(existing) = self
            .manifest_history
            .iter()
            .find(|entry| entry.manifest_digest == digest_value)
            .cloned()
        {
            let mut merged_ids = Vec::new();
            for id in record.cycle_ids.iter() {
                if !id.is_empty() && !merged_ids.contains(id) {
                    merged_ids.push(id.clone());
                }
            }
            for id in existing.cycle_ids {
                if !id.is_empty() && !merged_ids.contains(&id) {
                    merged_ids.push(id);
                }
            }
            if merged_ids.is_empty() && !record.cycle_ids.is_empty() {
                merged_ids = record.cycle_ids.clone();
            }
            record.cycle_ids = merged_ids;

            if record.bundle.is_none() {
                record.bundle = existing.bundle.clone();
            }
            if record.raw_manifest.is_none() {
                record.raw_manifest = existing.raw_manifest.clone();
            }
            if record.seen_at.is_none() {
                record.seen_at = existing.seen_at;
            }
        }

        if record.cycle_ids.len() > 8 {
            record.cycle_ids.truncate(8);
        }

        self.manifest_history
            .retain(|existing| existing.manifest_digest != digest_value);
        self.manifest_history.insert(0, record.clone());
        if self.manifest_history.len() > 20 {
            self.manifest_history.truncate(20);
        }
        if let Some(bundle) = record.bundle.as_ref() {
            self.bundle = Some(bundle.clone());
        }
        self.active_manifest_digest = Some(digest_value);
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AceState {
    pub cycles: Vec<AceCycleSummary>,
    pub selected_cycle_id: Option<String>,
    pub is_loading: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub snapshots: HashMap<String, CycleSnapshotView>,
    #[serde(default)]
    pub outboxes: HashMap<String, Vec<OutboxMessageView>>,
    #[serde(default)]
    pub snapshot_loading: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LiveStreamState {
    pub is_connected: bool,
    pub last_event_id: Option<u64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub tenants: Vec<TenantWorkspace>,
    pub is_loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_event_id: Option<u64>,
    #[serde(default)]
    pub depth: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OperationStageKind {
    TriggerSubmit,
    StreamAwait,
    SnapshotRefresh,
    OutboxReady,
    HitlSubmit,
    ContextSync,
    Unknown,
}

impl Default for OperationStageKind {
    fn default() -> Self {
        OperationStageKind::Unknown
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationStageStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl Default for OperationStageStatus {
    fn default() -> Self {
        OperationStageStatus::Pending
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OperationStage {
    pub kind: OperationStageKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub status: OperationStageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub started_at_epoch_ms: Option<u128>,
    #[serde(skip_serializing, skip_deserializing)]
    pub finished_at_epoch_ms: Option<u128>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuditActionKind {
    Copy,
    Export,
}

impl Default for AuditActionKind {
    fn default() -> Self {
        AuditActionKind::Copy
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: u64,
    pub action: AuditActionKind,
    pub label: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub timestamp: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuditLogState {
    #[serde(default)]
    pub next_id: u64,
    #[serde(default)]
    pub entries: Vec<AuditLogEntry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphState {
    pub query: GraphQuery,
    pub causal: Option<CausalGraphView>,
    pub recall: Vec<RecallResultView>,
    pub is_loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OperationState {
    pub last_message: Option<String>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggered_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_cycle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_outcome: Option<CycleOutcomeSummary>,
    #[serde(default)]
    pub stages: Vec<OperationStage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<OperationStageKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_elapsed_ms: Option<u64>,
    #[serde(default)]
    pub last_indices_used: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_budget: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppState {
    pub tenant_id: Option<String>,
    pub session_id: Option<String>,
    pub scenario_filter: Option<ConversationScenario>,
    pub timeline: TimelineState,
    pub context: ContextState,
    pub ace: AceState,
    pub live_stream: LiveStreamState,
    pub workspace: WorkspaceState,
    pub graph: GraphState,
    pub operation: OperationState,
    pub audit: AuditLogState,
}

#[derive(Clone)]
pub struct AppActions {
    state: AppSignal,
}

impl AppActions {
    pub fn set_tenant(&self, tenant: Option<String>) {
        let mut state = self.state.write_unchecked();
        state.tenant_id = tenant;
        state.timeline.clear();
        state.timeline.filters.clear();
        state.timeline.query.session_id = None;
        state.timeline.query.scenario = None;
        state.timeline.query.cursor = None;
        state.context = ContextState::default();
        state.ace = AceState::default();
        state.live_stream = LiveStreamState::default();
        state.graph = GraphState::default();
        state.operation = OperationState::default();
    }

    pub fn set_session(&self, session: Option<String>) {
        let mut state = self.state.write_unchecked();
        state.session_id = session;
        state.timeline.clear();
        state.timeline.filters.clear();
        state.timeline.query.session_id = state.session_id.clone();
        state.timeline.query.cursor = None;
        state.context = ContextState::default();
        state.ace = AceState::default();
        state.live_stream = LiveStreamState::default();
        state.graph = GraphState::default();
        state.operation = OperationState::default();
    }

    pub fn set_scenario(&self, scenario: Option<ConversationScenario>) {
        if self.state.read().scenario_filter != scenario {
            let mut state = self.state.write_unchecked();
            state.scenario_filter = scenario;
            state.timeline.clear();
            state.timeline.query.scenario = state.scenario_filter.clone();
            state.timeline.query.cursor = None;
        }
    }

    pub fn set_timeline_loading(&self, loading: bool) {
        self.state.write_unchecked().timeline.is_loading = loading;
    }

    pub fn set_timeline_error(&self, message: Option<String>) {
        self.state.write_unchecked().timeline.error = message;
    }

    pub fn toggle_participant_role(&self, role: &str) {
        let mut state = self.state.write_unchecked();
        state.timeline.filters.toggle_participant_role(role);
    }

    pub fn toggle_access_class(&self, access_class: &str) {
        let mut state = self.state.write_unchecked();
        state.timeline.filters.toggle_access_class(access_class);
    }

    pub fn toggle_degradation_reason(&self, reason: &str) {
        let mut state = self.state.write_unchecked();
        state.timeline.filters.toggle_degradation_reason(reason);
    }

    pub fn toggle_awareness_type(&self, event_type: &str) {
        let mut state = self.state.write_unchecked();
        state.timeline.filters.toggle_awareness_type(event_type);
    }

    pub fn toggle_router_digest(&self, digest: &str) {
        let mut state = self.state.write_unchecked();
        state.timeline.filters.toggle_router_digest(digest);
    }

    pub fn toggle_query_hash(&self, hash: &str) {
        let mut state = self.state.write_unchecked();
        state.timeline.filters.toggle_query_hash(hash);
    }

    pub fn clear_timeline_filters(&self) {
        let mut state = self.state.write_unchecked();
        state.timeline.filters.clear();
    }

    pub fn add_event_tag(&self, event_id: u64, tag: String) {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return;
        }

        let mut state = self.state.write_unchecked();
        let entry = state.timeline.tags.entry(event_id).or_default();
        if !entry
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(trimmed))
        {
            entry.push(trimmed.to_string());
        }
    }

    pub fn remove_event_tag(&self, event_id: u64, tag: &str) {
        let mut state = self.state.write_unchecked();
        if let Some(entry) = state.timeline.tags.get_mut(&event_id) {
            entry.retain(|existing| existing != tag);
            if entry.is_empty() {
                state.timeline.tags.remove(&event_id);
            }
        }
    }

    pub fn playback_sample_timeline(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let actions = self.clone();
            wasm_bindgen_futures::spawn_local(async move {
                use crate::fixtures::timeline::sample_timeline_data;
                use gloo_timers::future::TimeoutFuture;

                let (events, awareness) = sample_timeline_data();
                actions.reset_timeline();
                actions.set_timeline_loading(true);

                for event in events {
                    TimeoutFuture::new(200).await;
                    actions.append_timeline(vec![event], Vec::new(), None);
                }

                if !awareness.is_empty() {
                    actions.append_timeline(Vec::new(), awareness, None);
                }

                actions.set_operation_success("时间线回放完成".into());
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::fixtures::timeline::sample_timeline_data;

            let (events, awareness) = sample_timeline_data();
            self.reset_timeline();
            self.append_timeline(events, awareness, None);
            self.set_operation_success("时间线回放完成 (离线)".into());
        }
    }

    pub fn append_timeline(
        &self,
        mut events: Vec<DialogueEvent>,
        mut awareness: Vec<AwarenessEvent>,
        next_cursor: Option<String>,
    ) {
        let mut state = self.state.write_unchecked();

        for event in events.drain(..) {
            if !state
                .timeline
                .events
                .iter()
                .any(|existing| existing.event_id == event.event_id)
            {
                state.timeline.events.push(event);
            }
        }
        state
            .timeline
            .events
            .sort_by_key(|event| event.timestamp_ms);

        if state.graph.query.root_event_id.is_none() {
            if let Some(first) = state.timeline.events.first() {
                state.graph.query.root_event_id = Some(first.event_id.as_u64());
            }
        }

        for item in awareness.drain(..) {
            if !state
                .timeline
                .awareness
                .iter()
                .any(|existing| existing.event_id == item.event_id)
            {
                state.timeline.awareness.push(item);
            }
        }
        state
            .timeline
            .awareness
            .sort_by_key(|event| event.occurred_at_ms);

        if let Some(cursor) = next_cursor {
            state.timeline.next_cursor = Some(cursor);
        }

        state.timeline.error = None;
        state.timeline.is_loading = false;
    }

    pub fn set_context_loading(&self, loading: bool) {
        self.state.write_unchecked().context.is_loading = loading;
    }

    pub fn set_context_error(&self, message: Option<String>) {
        self.state.write_unchecked().context.error = message;
    }

    pub fn set_context_bundle(
        &self,
        bundle: Option<ContextBundleView>,
        explain_indices: Option<ExplainIndices>,
    ) {
        let mut state = self.state.write_unchecked();
        state.context.explain_indices = explain_indices;
        state.context.is_loading = false;
        state.context.error = None;

        if let Some(bundle_value) = bundle.clone() {
            match serde_json::to_value(&bundle_value) {
                Ok(raw) => {
                    if let Some(mut record) = manifest_record_from_value(&raw, None) {
                        if record.bundle.is_none() {
                            record.bundle = Some(bundle_value.clone());
                        }
                        if record.raw_manifest.is_none() {
                            record.raw_manifest = Some(raw);
                        }
                        state.context.upsert_manifest_entry(record);
                    } else {
                        state.context.bundle = Some(bundle_value);
                        state.context.active_manifest_digest = None;
                    }
                }
                Err(_) => {
                    state.context.bundle = Some(bundle_value);
                    state.context.active_manifest_digest = None;
                }
            }
        } else {
            state.context.bundle = None;
            state.context.active_manifest_digest = None;
        }
    }

    pub fn set_ace_loading(&self, loading: bool) {
        self.state.write_unchecked().ace.is_loading = loading;
    }

    pub fn set_ace_error(&self, message: Option<String>) {
        self.state.write_unchecked().ace.error = message;
    }

    pub fn set_ace_cycles(&self, cycles: Vec<AceCycleSummary>) {
        let mut state = self.state.write_unchecked();
        state.ace.cycles = cycles;
        state.ace.is_loading = false;
        state.ace.error = None;
        state.ace.snapshot_loading = false;
    }

    pub fn select_ace_cycle(&self, cycle_id: Option<String>) {
        let mut state = self.state.write_unchecked();
        state.ace.selected_cycle_id = cycle_id;
        state.ace.snapshot_error = None;
    }

    pub fn set_ace_snapshot_loading(&self, loading: bool) {
        self.state.write_unchecked().ace.snapshot_loading = loading;
    }

    pub fn set_ace_snapshot_error(&self, message: Option<String>) {
        let mut state = self.state.write_unchecked();
        state.ace.snapshot_error = message.clone();
        state.ace.snapshot_loading = false;
    }

    pub fn store_ace_snapshot(
        &self,
        cycle_id: String,
        snapshot: CycleSnapshotView,
        outbox: Vec<OutboxMessageView>,
    ) {
        let mut state = self.state.write_unchecked();
        state
            .ace
            .snapshots
            .insert(cycle_id.clone(), snapshot.clone());
        state.ace.outboxes.insert(cycle_id.clone(), outbox);
        state.ace.snapshot_loading = false;
        state.ace.snapshot_error = None;

        if let Some(summary) = state
            .ace
            .cycles
            .iter_mut()
            .find(|cycle| cycle.cycle_id == cycle_id)
        {
            // 优先从 outcomes 获取实际执行状态，而不是 schedule.status
            summary.status = snapshot
                .outcomes
                .last()
                .and_then(|outcome| {
                    if outcome.status == "completed" {
                        Some(AceCycleStatus::Completed)
                    } else if outcome.status == "failed" || outcome.status == "rejected" {
                        Some(AceCycleStatus::Failed)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| AceCycleStatus::from_label(&snapshot.schedule.status));
            summary.lane = AceLane::from_label(&snapshot.schedule.lane);
            summary.anchor = Some(snapshot.schedule.anchor.clone());
            summary.budget = Some((&snapshot.schedule.budget).into());
            summary.decision_path = snapshot
                .schedule
                .router_decision
                .as_ref()
                .map(|decision| decision.decision_path.clone());
            summary.pending_injections = snapshot
                .sync_point
                .pending_injections
                .iter()
                .map(|injection| HitlInjection {
                    injection_id: injection.injection_id.clone(),
                    cycle_id: summary.cycle_id.clone(),
                    priority: injection.priority.clone(),
                    author_role: injection.author_role.clone(),
                    payload: injection.payload.clone(),
                    status: if injection.submitted_at.is_null() {
                        None
                    } else {
                        Some(injection.submitted_at.to_string())
                    },
                })
                .collect();
            if !snapshot.sync_point.context_manifest.is_null() {
                summary.metadata = Some(snapshot.sync_point.context_manifest.clone());
            }
        }

        if let Some(record) = manifest_record_from_value(
            &snapshot.sync_point.context_manifest,
            Some(cycle_id.clone()),
        ) {
            state.context.upsert_manifest_entry(record);
        }
    }

    pub fn set_operation_success(&self, message: String) {
        let mut state = self.state.write_unchecked();
        state.operation.last_message = Some(message);
        state.operation.error = None;
        state.operation.last_status = None;
        state.operation.error_code = None;
        state.operation.context = None;
        state.operation.last_indices_used.clear();
        state.operation.last_budget = None;
    }

    pub fn set_operation_error(&self, message: String) {
        let mut state = self.state.write_unchecked();
        state.operation.error = Some(message);
        state.operation.last_message = None;
        state.operation.last_status = None;
        state.operation.error_code = None;
        state.operation.trace_id = None;
        state.operation.context = None;
    }

    pub fn record_http_failure(
        &self,
        status: u16,
        trace_id: Option<String>,
        error_code: Option<String>,
        context: impl Into<String>,
        detail: Option<String>,
    ) {
        let mut state = self.state.write_unchecked();
        let context_label = context.into();
        let message = detail.unwrap_or_else(|| http_status_advice(status).to_string());

        state.operation.error = Some(message);
        state.operation.last_message = None;
        state.operation.last_status = Some(status);
        state.operation.error_code = error_code;
        state.operation.trace_id = trace_id;
        state.operation.context = Some(context_label);
    }

    pub fn set_operation_trace(&self, trace_id: Option<String>) {
        self.state.write_unchecked().operation.trace_id = trace_id;
    }

    pub fn set_operation_context(&self, context: Option<String>) {
        self.state.write_unchecked().operation.context = context;
    }

    pub fn set_operation_triggered(&self, triggered_at: Option<String>) {
        self.state.write_unchecked().operation.triggered_at = triggered_at;
    }

    pub fn set_operation_cycle(&self, cycle_id: Option<String>) {
        self.state.write_unchecked().operation.last_cycle_id = cycle_id;
    }

    pub fn record_audit_event(
        &self,
        action: AuditActionKind,
        label: impl Into<String>,
        target: impl Into<String>,
    ) {
        let mut state = self.state.write_unchecked();
        let entry = AuditLogEntry {
            id: state.audit.next_id,
            action,
            label: label.into(),
            target: target.into(),
            tenant_id: state.tenant_id.clone(),
            session_id: state.session_id.clone(),
            timestamp: now_iso_timestamp(),
        };
        state.audit.next_id = state.audit.next_id.saturating_add(1);
        state.audit.entries.insert(0, entry);
        if state.audit.entries.len() > 200 {
            state.audit.entries.truncate(200);
        }
    }

    pub fn clear_audit_logs(&self) {
        let mut state = self.state.write_unchecked();
        state.audit.entries.clear();
        state.audit.next_id = 0;
    }

    pub fn set_operation_outcome(&self, outcome: Option<CycleOutcomeSummary>) {
        self.state.write_unchecked().operation.last_outcome = outcome;
    }

    pub fn operation_stage_reset(&self) {
        let mut state = self.state.write_unchecked();
        state.operation.stages.clear();
        state.operation.current_stage = None;
        state.operation.total_elapsed_ms = None;
    }

    pub fn operation_stage_start(&self, kind: OperationStageKind, label: impl Into<String>) {
        let mut state = self.state.write_unchecked();
        let label = label.into();
        let now_label = now_iso_timestamp();
        let now_epoch = now_epoch_ms();

        if let Some(stage) = state
            .operation
            .stages
            .iter_mut()
            .find(|stage| stage.kind == kind)
        {
            stage.status = OperationStageStatus::Running;
            stage.label = label;
            if stage.started_at.is_none() {
                stage.started_at = Some(now_label);
            }
            stage.started_at_epoch_ms = Some(now_epoch);
            stage.finished_at = None;
            stage.finished_at_epoch_ms = None;
            stage.duration_ms = None;
            stage.detail = None;
        } else {
            let mut stage = OperationStage::default();
            stage.kind = kind.clone();
            stage.label = label;
            stage.status = OperationStageStatus::Running;
            stage.started_at = Some(now_label);
            stage.started_at_epoch_ms = Some(now_epoch);
            state.operation.stages.push(stage);
        }

        state.operation.current_stage = Some(kind);
    }

    pub fn operation_stage_complete(&self, kind: OperationStageKind, detail: Option<String>) {
        let mut state = self.state.write_unchecked();
        let now_label = now_iso_timestamp();
        let now_epoch = now_epoch_ms();

        if let Some(stage) = state
            .operation
            .stages
            .iter_mut()
            .find(|stage| stage.kind == kind)
        {
            stage.status = OperationStageStatus::Completed;
            stage.finished_at = Some(now_label);
            stage.finished_at_epoch_ms = Some(now_epoch);
            stage.detail = detail;
            if let Some(start_epoch) = stage.started_at_epoch_ms {
                let duration = now_epoch.saturating_sub(start_epoch);
                stage.duration_ms = Some((duration.min(u64::MAX as u128)) as u64);
            }
        } else {
            let mut stage = OperationStage::default();
            stage.kind = kind.clone();
            stage.status = OperationStageStatus::Completed;
            stage.finished_at = Some(now_label);
            stage.finished_at_epoch_ms = Some(now_epoch);
            stage.detail = detail;
            state.operation.stages.push(stage);
        }

        if state.operation.current_stage.as_ref() == Some(&kind) {
            state.operation.current_stage = None;
        }

        let total: u64 = state
            .operation
            .stages
            .iter()
            .filter_map(|stage| stage.duration_ms)
            .sum();
        state.operation.total_elapsed_ms = if total > 0 { Some(total) } else { None };
    }

    pub fn operation_stage_fail(&self, kind: OperationStageKind, detail: Option<String>) {
        let mut state = self.state.write_unchecked();
        let now_label = now_iso_timestamp();
        let now_epoch = now_epoch_ms();

        if let Some(stage) = state
            .operation
            .stages
            .iter_mut()
            .find(|stage| stage.kind == kind)
        {
            stage.status = OperationStageStatus::Failed;
            stage.finished_at = Some(now_label);
            stage.finished_at_epoch_ms = Some(now_epoch);
            stage.detail = detail;
            if let Some(start_epoch) = stage.started_at_epoch_ms {
                let duration = now_epoch.saturating_sub(start_epoch);
                stage.duration_ms = Some((duration.min(u64::MAX as u128)) as u64);
            }
        } else {
            let mut stage = OperationStage::default();
            stage.kind = kind.clone();
            stage.status = OperationStageStatus::Failed;
            stage.finished_at = Some(now_label);
            stage.finished_at_epoch_ms = Some(now_epoch);
            stage.detail = detail;
            state.operation.stages.push(stage);
        }

        if state.operation.current_stage.as_ref() == Some(&kind) {
            state.operation.current_stage = None;
        }

        let total: u64 = state
            .operation
            .stages
            .iter()
            .filter_map(|stage| stage.duration_ms)
            .sum();
        state.operation.total_elapsed_ms = if total > 0 { Some(total) } else { None };
    }

    pub fn set_operation_diagnostics(&self, indices: Vec<String>, budget: Option<String>) {
        let mut state = self.state.write_unchecked();
        state.operation.last_indices_used = indices;
        state.operation.last_budget = budget;
    }

    pub fn clear_operation_status(&self) {
        let mut state = self.state.write_unchecked();
        state.operation = OperationState::default();
    }

    pub fn update_cycle_metadata(&self, cycle_id: Option<String>, addition: Value) {
        let mut state = self.state.write_unchecked();
        if state.ace.cycles.is_empty() {
            return;
        }

        let target_id = cycle_id
            .or_else(|| state.ace.selected_cycle_id.clone())
            .or_else(|| state.ace.cycles.first().map(|cycle| cycle.cycle_id.clone()));

        if let Some(target_id) = target_id {
            if let Some(cycle) = state
                .ace
                .cycles
                .iter_mut()
                .find(|cycle| cycle.cycle_id == target_id)
            {
                cycle.metadata = merge_metadata(cycle.metadata.take(), addition);
            }
        }
    }

    pub fn set_graph_root(&self, root: Option<u64>) {
        let mut state = self.state.write_unchecked();
        if state.graph.query.root_event_id != root {
            state.graph.query.root_event_id = root;
            state.graph.is_loading = false;
            state.graph.error = None;
        }
    }

    pub fn set_graph_loading(&self, loading: bool) {
        self.state.write_unchecked().graph.is_loading = loading;
    }

    pub fn set_graph_error(&self, message: Option<String>) {
        let mut state = self.state.write_unchecked();
        let has_error = message.is_some();
        state.graph.error = message;
        if has_error {
            state.graph.is_loading = false;
        }
    }

    pub fn set_graph_data(&self, causal: Option<CausalGraphView>, recall: Vec<RecallResultView>) {
        let mut state = self.state.write_unchecked();
        state.graph.causal = causal;
        state.graph.recall = recall;
        state.graph.is_loading = false;
        state.graph.error = None;
    }

    pub fn set_workspace_loading(&self, loading: bool) {
        self.state.write_unchecked().workspace.is_loading = loading;
    }

    pub fn set_workspace_error(&self, message: Option<String>) {
        let mut state = self.state.write_unchecked();
        let has_error = message.is_some();
        state.workspace.error = message;
        if has_error {
            state.workspace.is_loading = false;
        }
    }

    pub fn set_workspace_data(&self, tenants: Vec<TenantWorkspace>) {
        let mut normalized = tenants;
        for tenant in normalized.iter_mut() {
            normalize_workspace_tenant(tenant);
        }

        let mut state = self.state.write_unchecked();
        state.workspace.tenants = normalized;
        state.workspace.is_loading = false;
        state.workspace.error = None;
        if state.tenant_id.is_none() {
            if let Some(first) = state.workspace.tenants.first() {
                state.tenant_id = Some(first.tenant_id.clone());
            }
        }
    }

    pub fn set_session_pin(&self, tenant_id: &str, session_id: &str, pinned: bool) {
        let mut state = self.state.write_unchecked();
        if let Some(tenant) = state
            .workspace
            .tenants
            .iter_mut()
            .find(|tenant| tenant.tenant_id == tenant_id)
        {
            if let Some(index) = tenant
                .recent_sessions
                .iter()
                .position(|session| session.session_id == session_id)
            {
                let session_clone = {
                    let session = &mut tenant.recent_sessions[index];
                    session.pinned = pinned;
                    session.clone()
                };
                update_pinned_sessions(tenant, session_clone);
            }
        }
    }

    pub fn set_live_connected(&self, connected: bool) {
        let mut state = self.state.write_unchecked();
        state.live_stream.is_connected = connected;
        if connected {
            state.live_stream.error = None;
        }
    }

    pub fn set_live_error(&self, message: Option<String>) {
        let mut state = self.state.write_unchecked();
        state.live_stream.error = message;
        state.live_stream.is_connected = false;
    }

    pub fn record_live_event(&self, event_id: u64) {
        self.state.write_unchecked().live_stream.last_event_id = Some(event_id);
    }

    pub fn reset_timeline(&self) {
        self.state.write_unchecked().timeline.clear();
    }

    pub fn update_next_cursor(&self, cursor: Option<String>) {
        let mut state = self.state.write_unchecked();
        state.timeline.next_cursor = cursor.clone();
        state.timeline.query.cursor = cursor;
    }
}

fn normalize_workspace_tenant(tenant: &mut TenantWorkspace) {
    if tenant.pinned_sessions.is_empty() {
        for session in tenant.recent_sessions.iter() {
            if session.pinned {
                tenant.pinned_sessions.push(session.clone());
            }
        }
    } else {
        tenant.pinned_sessions.retain(|session| {
            tenant
                .recent_sessions
                .iter()
                .any(|recent| recent.session_id == session.session_id)
        });
        for session in tenant.recent_sessions.iter() {
            if session.pinned
                && !tenant
                    .pinned_sessions
                    .iter()
                    .any(|pinned| pinned.session_id == session.session_id)
            {
                tenant.pinned_sessions.push(session.clone());
            }
        }
    }

    for pinned in tenant.pinned_sessions.iter() {
        if let Some(recent) = tenant
            .recent_sessions
            .iter_mut()
            .find(|recent| recent.session_id == pinned.session_id)
        {
            recent.pinned = true;
        }
    }

    tenant.pinned_sessions.sort_by(|a, b| {
        let left = a.last_active_ms.unwrap_or(0);
        let right = b.last_active_ms.unwrap_or(0);
        right.cmp(&left)
    });
}

fn update_pinned_sessions(tenant: &mut TenantWorkspace, session: WorkspaceSession) {
    if let Some(recent) = tenant
        .recent_sessions
        .iter_mut()
        .find(|recent| recent.session_id == session.session_id)
    {
        recent.pinned = session.pinned;
    }

    if session.pinned {
        if let Some(existing) = tenant
            .pinned_sessions
            .iter_mut()
            .find(|pinned| pinned.session_id == session.session_id)
        {
            *existing = session;
        } else {
            tenant.pinned_sessions.push(session);
        }
    } else {
        tenant
            .pinned_sessions
            .retain(|pinned| pinned.session_id != session.session_id);
    }

    tenant.pinned_sessions.sort_by(|a, b| {
        let left = a.last_active_ms.unwrap_or(0);
        let right = b.last_active_ms.unwrap_or(0);
        right.cmp(&left)
    });
}

fn toggle_value(set: &mut BTreeSet<String>, value: &str) {
    let key = normalize_filter_value(value);
    if !set.insert(key.clone()) {
        set.remove(&key);
    }
}

#[derive(Default)]
struct RouterFilterFields {
    digest: Option<String>,
    query_hash: Option<String>,
}

fn extract_router_fields_from_event(event: &DialogueEvent) -> RouterFilterFields {
    let mut fields = RouterFilterFields::default();
    if !event.metadata.is_null() {
        visit_router_fields(&event.metadata, &mut fields);
    }
    if let Some(invocation) = event.tool_invocation.as_ref() {
        if let Ok(value) = serde_json::to_value(invocation) {
            visit_router_fields(&value, &mut fields);
        }
    }
    if let Some(result) = event.tool_result.as_ref() {
        if let Ok(value) = serde_json::to_value(result) {
            visit_router_fields(&value, &mut fields);
        }
    }
    if let Some(reflection) = event.self_reflection.as_ref() {
        if let Ok(value) = serde_json::to_value(reflection) {
            visit_router_fields(&value, &mut fields);
        }
    }
    fields
}

fn extract_router_fields_from_value(value: &Value) -> RouterFilterFields {
    let mut fields = RouterFilterFields::default();
    visit_router_fields(value, &mut fields);
    fields
}

fn visit_router_fields(value: &Value, fields: &mut RouterFilterFields) {
    match value {
        Value::Object(map) => {
            for (key, entry) in map {
                let normalized = normalized_router_key(key);
                if fields.digest.is_none() && normalized == "router_digest" {
                    if let Some(text) = entry.as_str() {
                        fields.digest = Some(text.to_string());
                    }
                }
                if fields.query_hash.is_none() && normalized == "query_hash" {
                    if let Some(text) = entry.as_str() {
                        fields.query_hash = Some(text.to_string());
                    }
                }
                if matches!(entry, Value::Object(_) | Value::Array(_)) {
                    visit_router_fields(entry, fields);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                visit_router_fields(item, fields);
            }
        }
        _ => {}
    }
}

fn normalized_router_key(key: &str) -> String {
    key.trim().replace('-', "_").to_ascii_lowercase()
}

fn extract_event_degradation(event: &DialogueEvent) -> Option<String> {
    if let Some(result) = event.tool_result.as_ref() {
        if let Some(reason) = result.degradation_reason.as_ref() {
            return Some(normalize_filter_value(&to_snake_case(reason)));
        }
    }

    if let Some(reason) = event
        .metadata
        .get("degradation_reason")
        .and_then(|value| value.as_str())
    {
        return Some(normalize_filter_value(&to_snake_case(reason)));
    }

    None
}

#[cfg(target_arch = "wasm32")]
fn now_iso_timestamp() -> String {
    js_sys::Date::new(&JsValue::from_f64(js_sys::Date::now()))
        .to_iso_string()
        .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn now_iso_timestamp() -> String {
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc();
    format!("{}.{:03}Z", now.unix_timestamp(), now.millisecond())
}

#[cfg(target_arch = "wasm32")]
fn now_epoch_ms() -> u128 {
    js_sys::Date::now() as u128
}

#[cfg(not(target_arch = "wasm32"))]
fn now_epoch_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis())
        .unwrap_or(0)
}

fn manifest_record_from_value(
    manifest: &Value,
    cycle_id: Option<String>,
) -> Option<ManifestDigestRecord> {
    if manifest.is_null() {
        return None;
    }

    let digest = manifest
        .get("manifest_digest")
        .and_then(|value| value.as_str())
        .or_else(|| manifest.get("digest").and_then(|value| value.as_str()))?
        .to_string();

    let mut cycle_ids = Vec::new();
    if let Some(id) = cycle_id {
        if !id.is_empty() {
            cycle_ids.push(id);
        }
    }

    let bundle = serde_json::from_value::<ContextBundleView>(manifest.clone()).ok();

    Some(ManifestDigestRecord {
        manifest_digest: digest,
        cycle_ids,
        seen_at: Some(now_iso_timestamp()),
        bundle,
        raw_manifest: Some(manifest.clone()),
    })
}

pub(crate) fn to_snake_case(value: &str) -> String {
    let mut snake = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if ch.is_uppercase() {
            if idx != 0 {
                snake.push('_');
            }
            for lower in ch.to_lowercase() {
                snake.push(lower);
            }
        } else {
            snake.push(ch);
        }
    }
    snake
}

pub(crate) fn normalize_filter_value(value: &str) -> String {
    value.trim().to_lowercase()
}

fn http_status_advice(status: u16) -> &'static str {
    match status {
        401 => "401 未授权：请检查 Token 是否过期或缺失，重新登录后重试。",
        403 => "403 权限不足：确认当前租户权限及角色配置。",
        409 => "409 冲突：可能存在重复提交，刷新时间线或调整 sequence_number。",
        429 => "429 频率受限：稍候重试，可降低请求频率或提升配额。",
        _ => "请求失败，请查看 trace_id 并在监控面板中排查。",
    }
}

fn merge_metadata(existing: Option<Value>, addition: Value) -> Option<Value> {
    match (existing, addition) {
        (Some(Value::Object(mut base)), Value::Object(new)) => {
            for (key, value) in new {
                base.insert(key, value);
            }
            Some(Value::Object(base))
        }
        (None, Value::Object(new)) => Some(Value::Object(new)),
        (_, other) => Some(other),
    }
}

pub fn use_app_state() -> AppSignal {
    use_context::<AppSignal>()
}

pub fn use_app_actions() -> AppActions {
    let state = use_app_state();
    AppActions { state }
}
