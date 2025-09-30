use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use crate::models::{
    AceCycleSummary, AwarenessEvent, CausalGraphView, ContextBundleView, ConversationScenario,
    DialogueEvent, ExplainIndices, RecallResultView, TenantWorkspace, WorkspaceSession,
};

pub type AppSignal = Signal<AppState>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TimelineQuery {
    #[serde(default = "TimelineQuery::default_limit")]
    pub limit: usize,
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
}

impl TimelineFilters {
    pub fn clear(&mut self) {
        self.participant_roles.clear();
        self.access_classes.clear();
        self.degradation_reasons.clear();
        self.awareness_types.clear();
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

        true
    }

    pub fn is_empty(&self) -> bool {
        self.participant_roles.is_empty()
            && self.access_classes.is_empty()
            && self.degradation_reasons.is_empty()
            && self.awareness_types.is_empty()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContextState {
    pub bundle: Option<ContextBundleView>,
    pub explain_indices: Option<ExplainIndices>,
    pub is_loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AceState {
    pub cycles: Vec<AceCycleSummary>,
    pub selected_cycle_id: Option<String>,
    pub is_loading: bool,
    pub error: Option<String>,
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
    pub last_trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
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
}

#[derive(Clone)]
pub struct AppActions {
    state: AppSignal,
}

impl AppActions {
    pub fn set_tenant(&self, tenant: Option<String>) {
        let mut state = self.state.write();
        state.tenant_id = tenant;
        state.timeline.clear();
        state.timeline.filters.clear();
        state.context = ContextState::default();
        state.ace = AceState::default();
        state.live_stream = LiveStreamState::default();
        state.graph = GraphState::default();
        state.operation = OperationState::default();
    }

    pub fn set_session(&self, session: Option<String>) {
        let mut state = self.state.write();
        state.session_id = session;
        state.timeline.clear();
        state.timeline.filters.clear();
        state.context = ContextState::default();
        state.ace = AceState::default();
        state.live_stream = LiveStreamState::default();
        state.graph = GraphState::default();
        state.operation = OperationState::default();
    }

    pub fn set_scenario(&self, scenario: Option<ConversationScenario>) {
        if self.state.read().scenario_filter != scenario {
            let mut state = self.state.write();
            state.scenario_filter = scenario;
            state.timeline.clear();
        }
    }

    pub fn set_timeline_loading(&self, loading: bool) {
        self.state.write().timeline.is_loading = loading;
    }

    pub fn set_timeline_error(&self, message: Option<String>) {
        self.state.write().timeline.error = message;
    }

    pub fn toggle_participant_role(&self, role: &str) {
        let mut state = self.state.write();
        state.timeline.filters.toggle_participant_role(role);
    }

    pub fn toggle_access_class(&self, access_class: &str) {
        let mut state = self.state.write();
        state.timeline.filters.toggle_access_class(access_class);
    }

    pub fn toggle_degradation_reason(&self, reason: &str) {
        let mut state = self.state.write();
        state.timeline.filters.toggle_degradation_reason(reason);
    }

    pub fn toggle_awareness_type(&self, event_type: &str) {
        let mut state = self.state.write();
        state.timeline.filters.toggle_awareness_type(event_type);
    }

    pub fn clear_timeline_filters(&self) {
        let mut state = self.state.write();
        state.timeline.filters.clear();
    }

    pub fn add_event_tag(&self, event_id: u64, tag: String) {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return;
        }

        let mut state = self.state.write();
        let entry = state.timeline.tags.entry(event_id).or_default();
        if !entry
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(trimmed))
        {
            entry.push(trimmed.to_string());
        }
    }

    pub fn remove_event_tag(&self, event_id: u64, tag: &str) {
        let mut state = self.state.write();
        if let Some(entry) = state.timeline.tags.get_mut(&event_id) {
            entry.retain(|existing| existing != tag);
            if entry.is_empty() {
                state.timeline.tags.remove(&event_id);
            }
        }
    }

    pub fn playback_sample_timeline(&self) {
        let actions = self.clone();
        #[cfg(target_arch = "wasm32")]
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
        let mut state = self.state.write();

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
        self.state.write().context.is_loading = loading;
    }

    pub fn set_context_error(&self, message: Option<String>) {
        self.state.write().context.error = message;
    }

    pub fn set_context_bundle(
        &self,
        bundle: Option<ContextBundleView>,
        explain_indices: Option<ExplainIndices>,
    ) {
        let mut state = self.state.write();
        state.context.bundle = bundle;
        state.context.explain_indices = explain_indices;
        state.context.is_loading = false;
        state.context.error = None;
    }

    pub fn set_ace_loading(&self, loading: bool) {
        self.state.write().ace.is_loading = loading;
    }

    pub fn set_ace_error(&self, message: Option<String>) {
        self.state.write().ace.error = message;
    }

    pub fn set_ace_cycles(&self, cycles: Vec<AceCycleSummary>) {
        let mut state = self.state.write();
        state.ace.cycles = cycles;
        state.ace.is_loading = false;
        state.ace.error = None;
    }

    pub fn select_ace_cycle(&self, cycle_id: Option<String>) {
        self.state.write().ace.selected_cycle_id = cycle_id;
    }

    pub fn set_operation_success(&self, message: String) {
        let mut state = self.state.write();
        state.operation.last_message = Some(message);
        state.operation.error = None;
        state.operation.last_status = None;
        state.operation.last_trace_id = None;
        state.operation.context = None;
    }

    pub fn set_operation_error(&self, message: String) {
        let mut state = self.state.write();
        state.operation.error = Some(message);
        state.operation.last_message = None;
        state.operation.last_status = None;
        state.operation.last_trace_id = None;
        state.operation.context = None;
    }

    pub fn record_http_failure(
        &self,
        status: u16,
        trace_id: Option<String>,
        context: impl Into<String>,
        detail: Option<String>,
    ) {
        let mut state = self.state.write();
        let context_label = context.into();
        let message = detail.unwrap_or_else(|| http_status_advice(status).to_string());

        state.operation.error = Some(message);
        state.operation.last_message = None;
        state.operation.last_status = Some(status);
        state.operation.last_trace_id = trace_id;
        state.operation.context = Some(context_label);
    }

    pub fn clear_operation_status(&self) {
        let mut state = self.state.write();
        state.operation = OperationState::default();
    }

    pub fn update_cycle_metadata(&self, cycle_id: Option<String>, addition: Value) {
        let mut state = self.state.write();
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
        let mut state = self.state.write();
        if state.graph.query.root_event_id != root {
            state.graph.query.root_event_id = root;
            state.graph.is_loading = false;
            state.graph.error = None;
        }
    }

    pub fn set_graph_loading(&self, loading: bool) {
        self.state.write().graph.is_loading = loading;
    }

    pub fn set_graph_error(&self, message: Option<String>) {
        let mut state = self.state.write();
        state.graph.error = message;
        if message.is_some() {
            state.graph.is_loading = false;
        }
    }

    pub fn set_graph_data(&self, causal: Option<CausalGraphView>, recall: Vec<RecallResultView>) {
        let mut state = self.state.write();
        state.graph.causal = causal;
        state.graph.recall = recall;
        state.graph.is_loading = false;
        state.graph.error = None;
    }

    pub fn set_workspace_loading(&self, loading: bool) {
        self.state.write().workspace.is_loading = loading;
    }

    pub fn set_workspace_error(&self, message: Option<String>) {
        let mut state = self.state.write();
        state.workspace.error = message;
        if message.is_some() {
            state.workspace.is_loading = false;
        }
    }

    pub fn set_workspace_data(&self, tenants: Vec<TenantWorkspace>) {
        let mut normalized = tenants;
        for tenant in normalized.iter_mut() {
            normalize_workspace_tenant(tenant);
        }

        let mut state = self.state.write();
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
        let mut state = self.state.write();
        if let Some(tenant) = state
            .workspace
            .tenants
            .iter_mut()
            .find(|tenant| tenant.tenant_id == tenant_id)
        {
            if let Some(session) = tenant
                .recent_sessions
                .iter_mut()
                .find(|session| session.session_id == session_id)
            {
                session.pinned = pinned;
                update_pinned_sessions(tenant, session.clone());
            }
        }
    }

    pub fn set_live_connected(&self, connected: bool) {
        let mut state = self.state.write();
        state.live_stream.is_connected = connected;
        if connected {
            state.live_stream.error = None;
        }
    }

    pub fn set_live_error(&self, message: Option<String>) {
        let mut state = self.state.write();
        state.live_stream.error = message;
        state.live_stream.is_connected = false;
    }

    pub fn record_live_event(&self, event_id: u64) {
        self.state.write().live_stream.last_event_id = Some(event_id);
    }

    pub fn reset_timeline(&self) {
        self.state.write().timeline.clear();
    }

    pub fn update_next_cursor(&self, cursor: Option<String>) {
        if let Some(cursor) = cursor {
            self.state.write().timeline.next_cursor = Some(cursor);
        }
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

pub fn use_app_state(cx: &ScopeState) -> AppSignal {
    use_context::<AppSignal>(cx).expect("AppState context not provided")
}

pub fn use_app_actions(cx: &ScopeState) -> AppActions {
    let state = use_app_state(cx);
    AppActions { state }
}
