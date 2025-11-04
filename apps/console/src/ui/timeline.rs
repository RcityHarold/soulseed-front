use crate::hooks::{live::use_live_stream, timeline::use_timeline_loader};
use crate::models::{
    AwarenessEvent, AwarenessEventType, ConversationScenario, DialogueEvent, DialogueEventType,
};
use crate::state::{
    normalize_filter_value, to_snake_case, use_app_actions, use_app_state, AppActions,
    AuditActionKind, TimelineFilters, TimelineState,
};
use crate::APP_CONFIG;
use dioxus::prelude::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const TIMELINE_CONTAINER_CLASS: &str = "space-y-4";
const CHIP_BASE_CLASS: &str = "px-3 py-1 rounded-full border text-xs transition-colors";
const CHIP_ACTIVE_CLASS: &str = "bg-slate-900 text-white border-slate-900";
#[derive(Clone, Debug, Default)]
struct RouterInsights {
    router_digest: Option<String>,
    indices_used: Vec<String>,
    query_hash: Option<String>,
}

impl RouterInsights {
    fn has_data(&self) -> bool {
        self.router_digest.is_some() || !self.indices_used.is_empty() || self.query_hash.is_some()
    }

    fn merge(&mut self, other: RouterInsights) {
        if self.router_digest.is_none() {
            self.router_digest = other.router_digest;
        }
        if self.query_hash.is_none() {
            self.query_hash = other.query_hash;
        }
        if self.indices_used.is_empty() {
            self.indices_used = other.indices_used;
        } else {
            for value in other.indices_used {
                if !self
                    .indices_used
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&value))
                {
                    self.indices_used.push(value);
                }
            }
        }
    }
}

fn collect_router_insights(value: &Value) -> RouterInsights {
    let mut insights = RouterInsights::default();
    scan_router_fields(value, &mut insights);
    insights
}

fn scan_router_fields(value: &Value, insights: &mut RouterInsights) {
    match value {
        Value::Object(map) => {
            for (key, entry) in map {
                let normalized = normalize_key(key);
                match normalized.as_str() {
                    "router_digest" | "routerdigest" => {
                        if insights.router_digest.is_none() {
                            if let Some(text) = entry.as_str() {
                                insights.router_digest = Some(text.to_string());
                            }
                        }
                    }
                    "indices_used" | "indicesused" => {
                        if insights.indices_used.is_empty() {
                            if let Some(array) = entry.as_array() {
                                for item in array {
                                    if let Some(text) = item.as_str() {
                                        push_unique(&mut insights.indices_used, text);
                                    }
                                }
                            } else if let Some(text) = entry.as_str() {
                                push_unique(&mut insights.indices_used, text);
                            }
                        }
                    }
                    "query_hash" | "queryhash" => {
                        if insights.query_hash.is_none() {
                            if let Some(text) = entry.as_str() {
                                insights.query_hash = Some(text.to_string());
                            }
                        }
                    }
                    _ => {}
                }
                if let Value::Object(_) | Value::Array(_) = entry {
                    scan_router_fields(entry, insights);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                scan_router_fields(item, insights);
            }
        }
        _ => {}
    }
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(value))
    {
        list.push(value.to_string());
    }
}

fn normalize_key(key: &str) -> String {
    key.trim().replace('-', "_").to_ascii_lowercase()
}

fn render_router_insights(insights: &RouterInsights) -> Option<Element> {
    if !insights.has_data() {
        return None;
    }

    let digest_label = insights.router_digest.as_deref().unwrap_or("未提供");

    Some(rsx! {
        div { class: "rounded border border-slate-200 bg-slate-50 p-2 text-[11px] text-slate-600 space-y-1",
            div { class: "flex flex-wrap items-center gap-2",
                span { class: "font-semibold text-slate-700", "路由 Digest" }
                span { class: "font-mono break-all text-slate-600", "{digest_label}" }
            }
            if !insights.indices_used.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold text-slate-500", "Indices Used" }
                    div { class: "flex flex-wrap gap-1",
                        for idx in insights.indices_used.iter() {
                            span { class: "rounded bg-slate-200 px-2 py-0.5 text-[11px] text-slate-700", "{idx}" }
                        }
                    }
                }
            }
            if let Some(query) = insights.query_hash.as_ref() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold text-slate-500", "Query Hash" }
                    span { class: "font-mono text-[11px] text-slate-600 break-all", "{query}" }
                }
            }
        }
    })
}

const CHIP_INACTIVE_CLASS: &str = "bg-white text-slate-700 border-slate-200 hover:border-slate-400";

#[derive(Clone)]
struct FilterOption {
    value: String,
    label: String,
}

#[component]
pub fn TimelineView() -> Element {
    use_timeline_loader();
    use_live_stream();

    let actions = use_app_actions();
    let snapshot = use_app_state().read().clone();

    let tenant_label = snapshot
        .tenant_id
        .clone()
        .or_else(|| {
            APP_CONFIG
                .get()
                .and_then(|cfg| cfg.default_tenant_id.clone())
        })
        .unwrap_or_else(|| "未配置".to_string());

    let session_label = snapshot
        .session_id
        .clone()
        .unwrap_or_else(|| "未选择会话".to_string());

    let scenario_filter = snapshot.scenario_filter.clone();
    let timeline = snapshot.timeline.clone();
    let filters = timeline.filters.clone();
    let tags = timeline.tags.clone();
    let awareness = timeline.awareness.clone();
    let live_state = snapshot.live_stream.clone();

    let role_options = collect_role_options(&timeline.events);
    let access_options = collect_access_options(&timeline.events);
    let degradation_options = collect_degradation_options(&timeline.events, &awareness);
    let awareness_options = collect_awareness_options(&awareness);
    let router_digest_options = collect_router_digest_options(&timeline.events, &awareness);
    let query_hash_options = collect_query_hash_options(&timeline.events, &awareness);

    rsx! {
        section { class: TIMELINE_CONTAINER_CLASS,
            header { class: "flex flex-col gap-1",
                h1 { class: "text-lg font-semibold text-slate-900", "时间线概览" }
                p { class: "text-xs text-slate-500", "租户: {tenant_label} · 会话: {session_label}" }
                LiveStatus {
                    connected: live_state.is_connected,
                    error: live_state.error.clone(),
                    last_event_id: live_state.last_event_id,
                }
            }

            ScenarioSwitcher { scenario_filter, actions: actions.clone() }
            AuditToolbar { timeline: timeline.clone(), actions: actions.clone() }
            FilterToolbar {
                filters: filters.clone(),
                role_options,
                access_options,
                degradation_options,
                awareness_options,
                router_digest_options,
                query_hash_options,
                actions: actions.clone(),
            }

            TimelineColumn {
                timeline: timeline.clone(),
                filters: filters.clone(),
                tags,
                actions: actions.clone(),
            }
            AwarenessColumn { awareness, filters }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct ScenarioSwitcherProps {
    scenario_filter: Option<ConversationScenario>,
    actions: AppActions,
}

impl PartialEq for ScenarioSwitcherProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for ScenarioSwitcherProps {}

#[component]
fn ScenarioSwitcher(props: ScenarioSwitcherProps) -> Element {
    let current = props.scenario_filter.clone();
    let actions = props.actions.clone();

    rsx! {
        div { class: "flex flex-wrap gap-2",
            for option in scenario_options().into_iter() {
                button {
                    key: format!("scenario-{}", option.label),
                    class: {
                        let active = current == option.scenario;
                        format!(
                            "{} {}",
                            CHIP_BASE_CLASS,
                            if active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                        )
                    },
                    onclick: {
                        let actions = actions.clone();
                        let option_value = option.scenario.clone();
                        move |_| actions.set_scenario(option_value.clone())
                    },
                    span { class: "text-xs font-medium", "{option.label}" }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct AuditToolbarProps {
    timeline: TimelineState,
    actions: AppActions,
}

impl PartialEq for AuditToolbarProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for AuditToolbarProps {}

#[component]
fn AuditToolbar(props: AuditToolbarProps) -> Element {
    let timeline = props.timeline.clone();
    let actions = props.actions.clone();

    let on_export_json = {
        let actions = actions.clone();
        let data = timeline.clone();
        move |_| {
            let content = timeline_to_json(&data);
            actions.record_audit_event(
                AuditActionKind::Export,
                "时间线 JSON",
                "timeline:export_json",
            );
            copy_text_to_clipboard(
                actions.clone(),
                "时间线 JSON",
                "timeline:json_copy",
                content,
            );
        }
    };

    let on_export_csv = {
        let actions = actions.clone();
        let data = timeline.clone();
        move |_| {
            let content = timeline_to_csv(&data);
            actions.record_audit_event(
                AuditActionKind::Export,
                "时间线 CSV",
                "timeline:export_csv",
            );
            copy_text_to_clipboard(actions.clone(), "时间线 CSV", "timeline:csv_copy", content);
        }
    };

    let on_replay = {
        let actions = actions.clone();
        move |_| actions.playback_sample_timeline()
    };

    rsx! {
        div { class: "flex flex-wrap gap-2",
            button {
                class: "rounded border border-slate-300 bg-white px-3 py-1 text-xs text-slate-700 hover:bg-slate-100",
                onclick: on_export_json,
                "导出 JSON"
            }
            button {
                class: "rounded border border-slate-300 bg-white px-3 py-1 text-xs text-slate-700 hover:bg-slate-100",
                onclick: on_export_csv,
                "导出 CSV"
            }
            button {
                class: "rounded bg-emerald-600 px-3 py-1 text-xs font-semibold text-white hover:bg-emerald-500",
                onclick: on_replay,
                "回放示例"
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct FilterToolbarProps {
    filters: TimelineFilters,
    role_options: Vec<FilterOption>,
    access_options: Vec<FilterOption>,
    degradation_options: Vec<FilterOption>,
    awareness_options: Vec<FilterOption>,
    router_digest_options: Vec<FilterOption>,
    query_hash_options: Vec<FilterOption>,
    actions: AppActions,
}

impl PartialEq for FilterToolbarProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for FilterToolbarProps {}

#[component]
fn FilterToolbar(props: FilterToolbarProps) -> Element {
    let filters = props.filters.clone();
    let actions = props.actions.clone();

    if props.role_options.is_empty()
        && props.access_options.is_empty()
        && props.degradation_options.is_empty()
        && props.awareness_options.is_empty()
        && props.router_digest_options.is_empty()
        && props.query_hash_options.is_empty()
    {
        return rsx! { div {} };
    }

    rsx! {
        div { class: "space-y-2 rounded-lg border border-slate-200 bg-white p-4 shadow-sm text-xs text-slate-600",
            if !props.role_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "参与者角色" }
                    div { class: "flex flex-wrap gap-2",
                        for option in props.role_options.iter() {
                            button {
                                key: format!("role-{}", option.value),
                                class: {
                                    let active = filters.participant_roles.contains(&option.value);
                                    format!(
                                        "{} {}",
                                        CHIP_BASE_CLASS,
                                        if active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                                    )
                                },
                                onclick: {
                                    let actions = actions.clone();
                                    let value = option.value.clone();
                                    move |_| actions.toggle_participant_role(&value)
                                },
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !props.access_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "访问级别" }
                    div { class: "flex flex-wrap gap-2",
                        for option in props.access_options.iter() {
                            button {
                                key: format!("access-{}", option.value),
                                class: {
                                    let active = filters.access_classes.contains(&option.value);
                                    format!(
                                        "{} {}",
                                        CHIP_BASE_CLASS,
                                        if active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                                    )
                                },
                                onclick: {
                                    let actions = actions.clone();
                                    let value = option.value.clone();
                                    move |_| actions.toggle_access_class(&value)
                                },
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !props.degradation_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "降级原因" }
                    div { class: "flex flex-wrap gap-2",
                        for option in props.degradation_options.iter() {
                            button {
                                key: format!("degrade-{}", option.value),
                                class: {
                                    let active = filters.degradation_reasons.contains(&option.value);
                                    format!(
                                        "{} {}",
                                        CHIP_BASE_CLASS,
                                        if active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                                    )
                                },
                                onclick: {
                                    let actions = actions.clone();
                                    let value = option.value.clone();
                                    move |_| actions.toggle_degradation_reason(&value)
                                },
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !props.awareness_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "Awareness 类型" }
                    div { class: "flex flex-wrap gap-2",
                        for option in props.awareness_options.iter() {
                            button {
                                key: format!("aware-{}", option.value),
                                class: {
                                    let active = filters.awareness_types.contains(&option.value);
                                    format!(
                                        "{} {}",
                                        CHIP_BASE_CLASS,
                                        if active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                                    )
                                },
                                onclick: {
                                    let actions = actions.clone();
                                    let value = option.value.clone();
                                    move |_| actions.toggle_awareness_type(&value)
                                },
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !props.router_digest_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "Router Digest" }
                    div { class: "flex flex-wrap gap-2",
                        for option in props.router_digest_options.iter() {
                            button {
                                key: format!("router-{}", option.value),
                                class: {
                                    let active = filters.router_digests.contains(&option.value);
                                    format!(
                                        "{} {}",
                                        CHIP_BASE_CLASS,
                                        if active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                                    )
                                },
                                onclick: {
                                    let actions = actions.clone();
                                    let value = option.value.clone();
                                    move |_| actions.toggle_router_digest(&value)
                                },
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !props.query_hash_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "Query Hash" }
                    div { class: "flex flex-wrap gap-2",
                        for option in props.query_hash_options.iter() {
                            button {
                                key: format!("query-{}", option.value),
                                class: {
                                    let active = filters.query_hashes.contains(&option.value);
                                    format!(
                                        "{} {}",
                                        CHIP_BASE_CLASS,
                                        if active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                                    )
                                },
                                onclick: {
                                    let actions = actions.clone();
                                    let value = option.value.clone();
                                    move |_| actions.toggle_query_hash(&value)
                                },
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !filters.is_empty() {
                button {
                    class: "rounded bg-slate-100 px-3 py-1 text-[11px] text-slate-600 hover:bg-slate-200",
                    onclick: move |_| actions.clear_timeline_filters(),
                    "清空筛选"
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct TimelineColumnProps {
    timeline: TimelineState,
    filters: TimelineFilters,
    tags: HashMap<u64, Vec<String>>,
    actions: AppActions,
}

impl PartialEq for TimelineColumnProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for TimelineColumnProps {}

#[component]
fn TimelineColumn(props: TimelineColumnProps) -> Element {
    let timeline = props.timeline.clone();
    let filters = props.filters.clone();
    let tags_map = props.tags.clone();
    let actions = props.actions.clone();

    let filtered_events: Vec<_> = timeline
        .events
        .iter()
        .filter(|event| filters.matches_event(event))
        .cloned()
        .collect();

    rsx! {
        div { class: "space-y-3",
            h2 { class: "text-sm font-semibold text-slate-800", "对话事件" }
            if timeline.is_loading {
                p { class: "text-xs text-slate-500", "正在加载时间线..." }
            }
            if let Some(err) = timeline.error.clone() {
                p { class: "text-xs text-red-500", "加载失败: {err}" }
            }
            ul { class: "space-y-3",
                if filtered_events.is_empty() && !timeline.is_loading {
                    li { class: "text-xs text-slate-500 italic", "当前过滤条件下暂无对话事件" }
                } else {
                    for event in filtered_events.iter() {
                        li {
                            key: format!("event-{}", event.event_id.as_u64()),
                            EventCard {
                                event: event.clone(),
                                tags: {
                                    let id = event.event_id.as_u64();
                                    tags_map.get(&id).cloned().unwrap_or_default()
                                },
                                actions: actions.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct EventCardProps {
    event: DialogueEvent,
    tags: Vec<String>,
    actions: AppActions,
}

impl PartialEq for EventCardProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for EventCardProps {}

#[component]
fn EventCard(props: EventCardProps) -> Element {
    let event = props.event.clone();
    let tags = props.tags.clone();
    let actions = props.actions.clone();

    let event_id = event.event_id.as_u64();
    let event_type = format_dialogue_event_type(&event.event_type);
    let scenario_label = scenario_title(&event.scenario);
    let timestamp_ms = event.timestamp_ms;
    let subject_text = format!("{:?}", event.subject);
    let participant_text = if event.participants.is_empty() {
        None
    } else {
        Some(
            event
                .participants
                .iter()
                .map(|participant| {
                    let mut label = format!("{:?}", participant.kind);
                    if let Some(role) = participant.role.as_ref() {
                        label.push_str(&format!(" ({})", role));
                    }
                    label
                })
                .collect::<Vec<_>>()
                .join("、"),
        )
    };
    let mut router_insights = collect_router_insights(&event.metadata);
    if let Some(tool_invocation) = event.tool_invocation.as_ref() {
        if let Ok(value) = serde_json::to_value(tool_invocation) {
            router_insights.merge(collect_router_insights(&value));
        }
    }
    if let Some(tool_result) = event.tool_result.as_ref() {
        if let Ok(value) = serde_json::to_value(tool_result) {
            router_insights.merge(collect_router_insights(&value));
        }
    }
    if let Some(self_reflection) = event.self_reflection.as_ref() {
        if let Ok(value) = serde_json::to_value(self_reflection) {
            router_insights.merge(collect_router_insights(&value));
        }
    }

    let mut tag_input = use_signal(|| String::new());

    let on_submit = {
        let actions = actions.clone();
        let mut tag_input_signal = tag_input.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            let current_value = tag_input_signal.read().clone();
            let trimmed = current_value.trim();
            if trimmed.is_empty() {
                actions.set_operation_error("标签不能为空".into());
                return;
            }
            let tag_text = trimmed.to_string();
            actions.add_event_tag(event_id, tag_text);
            actions.set_operation_success(format!("已为事件 #{event_id} 添加标签"));
            tag_input_signal.set(String::new());
        }
    };

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-3 shadow-sm space-y-2",
            div { class: "flex items-center justify-between",
                span { class: "text-sm font-medium text-slate-900", "{event_type}" }
                span { class: "text-xs text-slate-500", "#{event_id} · {timestamp_ms}" }
            }
            div { class: "flex flex-wrap gap-2 text-xs text-slate-600",
                span { class: "rounded bg-slate-100 px-2 py-1 text-slate-700", "{scenario_label}" }
                span { class: "px-2 py-1", "主语: {subject_text}" }
                if let Some(participants) = participant_text {
                    span { class: "px-2 py-1", "参与者: {participants}" }
                }
            }
            if !event.metadata.is_null() {
                div { class: "rounded bg-slate-50 p-2 text-[11px] text-slate-500 break-words",
                    "metadata: {event.metadata}"
                }
            }
            if let Some(router_view) = render_router_insights(&router_insights) {
                {router_view}
            }
            if !tags.is_empty() {
                div { class: "flex flex-wrap gap-2",
                    for tag in tags.iter().cloned() {
                        EventTagPill {
                            key: format!("event-{event_id}-tag-{tag}"),
                            tag,
                            event_id,
                            actions: actions.clone(),
                        }
                    }
                }
            }
            form { class: "flex items-center gap-2 text-[11px]", onsubmit: on_submit,
                input {
                    class: "flex-1 rounded border border-slate-300 px-2 py-1",
                    placeholder: "添加标签",
                    value: "{tag_input.read()}",
                    oninput: move |evt| tag_input.set(evt.value().to_string()),
                }
                button {
                    class: "rounded bg-slate-900 px-3 py-1 text-[11px] font-semibold text-white hover:bg-slate-800",
                    r#type: "submit",
                    "添加"
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct EventTagPillProps {
    tag: String,
    event_id: u64,
    actions: AppActions,
}

impl PartialEq for EventTagPillProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for EventTagPillProps {}

#[component]
fn EventTagPill(props: EventTagPillProps) -> Element {
    let label = props.tag.clone();
    let remove_label = label.clone();
    let actions = props.actions.clone();
    let event_id = props.event_id;

    rsx! {
        span { class: "flex items-center gap-1 rounded-full bg-amber-100 px-2 py-1 text-[11px] text-amber-800",
            "{label}"
            button {
                class: "rounded bg-amber-200 px-1 text-[10px] text-amber-900",
                onclick: move |_| actions.remove_event_tag(event_id, &remove_label),
                "×"
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct AwarenessColumnProps {
    awareness: Vec<AwarenessEvent>,
    filters: TimelineFilters,
}

impl PartialEq for AwarenessColumnProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for AwarenessColumnProps {}

#[component]
fn AwarenessColumn(props: AwarenessColumnProps) -> Element {
    let filtered: Vec<_> = props
        .awareness
        .iter()
        .filter(|item| props.filters.matches_awareness(item))
        .cloned()
        .collect();

    rsx! {
        div { class: "space-y-3",
            h2 { class: "text-sm font-semibold text-slate-800", "Awareness 事件" }
            ul { class: "space-y-3",
                if filtered.is_empty() {
                    li { class: "text-xs text-slate-500 italic", "当前过滤条件下暂无 Awareness 数据" }
                } else {
                    for item in filtered.iter() {
                        li {
                            key: format!("awareness-{}", item.event_id.as_u64()),
                            class: "rounded-lg border border-amber-200 bg-amber-50 p-3",
                            div { class: "flex items-center justify-between",
                                span { class: "text-xs font-medium text-amber-900", "{format_awareness_event_type(&item.event_type)}" }
                                span { class: "text-[11px] text-amber-700", "#{item.event_id.as_u64()} · {item.occurred_at_ms}" }
                            }
                            if let Some(reason) = item
                                .degradation_reason
                                .as_ref()
                                .map(|reason| format!("降级: {:?}", reason))
                            {
                                div { class: "mt-1 text-[11px] text-amber-700", "{reason}" }
                            }
                            if let Some(router_view) = render_router_insights(&collect_router_insights(&item.payload)) {
                                {router_view}
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct LiveStatusProps {
    connected: bool,
    error: Option<String>,
    last_event_id: Option<u64>,
}

impl PartialEq for LiveStatusProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for LiveStatusProps {}

#[component]
fn LiveStatus(props: LiveStatusProps) -> Element {
    let status_text = if props.connected {
        "实时流: 已连接"
    } else {
        "实时流: 未连接"
    };
    let status_class = if props.connected {
        "text-xs text-green-600"
    } else {
        "text-xs text-slate-500"
    };

    rsx! {
        div { class: "flex flex-wrap items-center gap-2",
            span { class: status_class, "{status_text}" }
            if let Some(ref err) = props.error {
                span { class: "text-xs text-red-500", "错误: {err}" }
            } else if let Some(id) = props.last_event_id {
                span { class: "text-xs text-slate-500", "最后事件 #{id}" }
            }
        }
    }
}

struct ScenarioOption {
    scenario: Option<ConversationScenario>,
    label: &'static str,
}

fn scenario_options() -> Vec<ScenarioOption> {
    vec![
        ScenarioOption {
            scenario: None,
            label: "全部场景",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::HumanToAi),
            label: "Human ↔ AI",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::HumanToHuman),
            label: "Human ↔ Human",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::AiToAi),
            label: "AI ↔ AI",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::HumanGroup),
            label: "Human 群组",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::AiGroup),
            label: "AI 群组",
        },
    ]
}

fn router_insights_from_event(event: &DialogueEvent) -> RouterInsights {
    let mut insights = collect_router_insights(&event.metadata);
    if let Some(tool_invocation) = event.tool_invocation.as_ref() {
        if let Ok(value) = serde_json::to_value(tool_invocation) {
            insights.merge(collect_router_insights(&value));
        }
    }
    if let Some(tool_result) = event.tool_result.as_ref() {
        if let Ok(value) = serde_json::to_value(tool_result) {
            insights.merge(collect_router_insights(&value));
        }
    }
    if let Some(self_reflection) = event.self_reflection.as_ref() {
        if let Ok(value) = serde_json::to_value(self_reflection) {
            insights.merge(collect_router_insights(&value));
        }
    }
    insights
}

fn collect_router_digest_options(
    events: &[DialogueEvent],
    awareness: &[AwarenessEvent],
) -> Vec<FilterOption> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();

    for event in events {
        let insights = router_insights_from_event(event);
        if let Some(digest) = insights.router_digest.as_ref() {
            let normalized = normalize_filter_value(digest);
            map.entry(normalized).or_insert_with(|| digest.to_string());
        }
    }

    for item in awareness {
        let insights = collect_router_insights(&item.payload);
        if let Some(digest) = insights.router_digest.as_ref() {
            let normalized = normalize_filter_value(digest);
            map.entry(normalized).or_insert_with(|| digest.to_string());
        }
    }

    map.into_iter()
        .map(|(value, label)| FilterOption { value, label })
        .collect()
}

fn collect_query_hash_options(
    events: &[DialogueEvent],
    awareness: &[AwarenessEvent],
) -> Vec<FilterOption> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();

    for event in events {
        let insights = router_insights_from_event(event);
        if let Some(query_hash) = insights.query_hash.as_ref() {
            let normalized = normalize_filter_value(query_hash);
            map.entry(normalized)
                .or_insert_with(|| query_hash.to_string());
        }
    }

    for item in awareness {
        let insights = collect_router_insights(&item.payload);
        if let Some(query_hash) = insights.query_hash.as_ref() {
            let normalized = normalize_filter_value(query_hash);
            map.entry(normalized)
                .or_insert_with(|| query_hash.to_string());
        }
    }

    map.into_iter()
        .map(|(value, label)| FilterOption { value, label })
        .collect()
}

fn collect_role_options(events: &[DialogueEvent]) -> Vec<FilterOption> {
    let mut roles = BTreeSet::new();
    for event in events {
        for participant in &event.participants {
            if let Some(role) = participant.role.as_ref() {
                let normalized = normalize_filter_value(role);
                let label = format_filter_label(role);
                roles.insert((normalized, label));
            }
        }
    }

    roles
        .into_iter()
        .map(|(value, label)| FilterOption { value, label })
        .collect()
}

fn collect_access_options(events: &[DialogueEvent]) -> Vec<FilterOption> {
    let mut set = BTreeSet::new();
    for event in events {
        let raw = format!("{:?}", event.access_class);
        let normalized = normalize_filter_value(&to_snake_case(&raw));
        let label = format_filter_label(&raw);
        set.insert((normalized, label));
    }
    set.into_iter()
        .map(|(value, label)| FilterOption { value, label })
        .collect()
}

fn collect_degradation_options(
    events: &[DialogueEvent],
    awareness: &[AwarenessEvent],
) -> Vec<FilterOption> {
    let mut set = BTreeSet::new();

    for event in events {
        if let Some(reason) = event_degradation(event) {
            let normalized = normalize_filter_value(&to_snake_case(&reason));
            let label = format_filter_label(&reason);
            set.insert((normalized, label));
        }
    }

    for item in awareness {
        if let Some(reason) = item.degradation_reason.as_ref() {
            let raw = format!("{:?}", reason);
            let normalized = normalize_filter_value(&to_snake_case(&raw));
            let label = format_filter_label(&raw);
            set.insert((normalized, label));
        }
    }

    set.into_iter()
        .map(|(value, label)| FilterOption { value, label })
        .collect()
}

fn collect_awareness_options(awareness: &[AwarenessEvent]) -> Vec<FilterOption> {
    let mut set = BTreeSet::new();
    for item in awareness {
        let raw = format!("{:?}", item.event_type);
        let normalized = normalize_filter_value(&to_snake_case(&raw));
        let label = format_filter_label(&raw);
        set.insert((normalized, label));
    }
    set.into_iter()
        .map(|(value, label)| FilterOption { value, label })
        .collect()
}

fn format_filter_label(value: &str) -> String {
    let mut words = Vec::new();
    if value.contains('_') {
        for part in value.split('_') {
            if part.is_empty() {
                continue;
            }
            words.push(capitalize(part));
        }
    } else if value.chars().any(|ch| ch.is_uppercase()) {
        let mut current = String::new();
        for ch in value.chars() {
            if ch.is_uppercase() && !current.is_empty() {
                words.push(capitalize(&current));
                current.clear();
            }
            current.push(ch);
        }
        if !current.is_empty() {
            words.push(capitalize(&current));
        }
    } else {
        words.push(capitalize(value));
    }

    if words.is_empty() {
        value.to_string()
    } else {
        words.join(" ")
    }
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => {
            let mut result = first.to_uppercase().collect::<String>();
            let rest = chars.as_str().to_lowercase();
            result.push_str(&rest);
            result
        }
        None => String::new(),
    }
}

fn scenario_title(scenario: &ConversationScenario) -> &'static str {
    match scenario {
        ConversationScenario::HumanToHuman => "人类 ↔ 人类",
        ConversationScenario::HumanGroup => "人类群组",
        ConversationScenario::HumanToAi => "人类 ↔ AI",
        ConversationScenario::AiToAi => "AI ↔ AI",
        ConversationScenario::AiSelfTalk => "AI 自对话",
        ConversationScenario::HumanToMultiAi => "人类 ↔ 多 AI",
        ConversationScenario::MultiHumanToMultiAi => "多 人类 ↔ 多 AI",
        ConversationScenario::AiGroup => "AI 群组",
        ConversationScenario::AiToSystem => "AI ↔ 系统",
    }
}

fn format_dialogue_event_type(event_type: &DialogueEventType) -> &'static str {
    match event_type {
        DialogueEventType::Message => "消息",
        DialogueEventType::ToolCall => "工具调用",
        DialogueEventType::ToolResult => "工具结果",
        DialogueEventType::SelfReflection => "自反",
        DialogueEventType::Decision => "决策",
        DialogueEventType::Lifecycle => "生命周期",
        DialogueEventType::System => "系统",
    }
}

fn format_awareness_event_type(event_type: &AwarenessEventType) -> &'static str {
    match event_type {
        AwarenessEventType::AwarenessCycleStarted => "AC 启动",
        AwarenessEventType::AwarenessCycleEnded => "AC 结束",
        AwarenessEventType::InferenceCycleStarted => "IC 启动",
        AwarenessEventType::InferenceCycleCompleted => "IC 完成",
        AwarenessEventType::AssessmentProduced => "Assessment",
        AwarenessEventType::DecisionRouted => "决策路由",
        AwarenessEventType::RouteReconsidered => "路径重审",
        AwarenessEventType::RouteSwitched => "路径切换",
        AwarenessEventType::ToolPathDecided => "工具路径决策",
        AwarenessEventType::ToolCalled => "工具调用",
        AwarenessEventType::ToolResponded => "工具响应",
        AwarenessEventType::ToolFailed => "工具失败",
        AwarenessEventType::ToolBarrierReached => "工具栅栏触达",
        AwarenessEventType::ToolBarrierReleased => "工具栅栏释放",
        AwarenessEventType::ToolBarrierTimeout => "工具栅栏超时",
        AwarenessEventType::CollabRequested => "协作请求",
        AwarenessEventType::CollabResolved => "协作完成",
        AwarenessEventType::ClarificationIssued => "Clarify 提问",
        AwarenessEventType::ClarificationAnswered => "Clarify 回答",
        AwarenessEventType::HumanInjectionReceived => "HITL 接收",
        AwarenessEventType::HumanInjectionApplied => "注入应用",
        AwarenessEventType::HumanInjectionDeferred => "注入延后",
        AwarenessEventType::HumanInjectionIgnored => "注入忽略",
        AwarenessEventType::DeltaPatchGenerated => "DeltaPatch",
        AwarenessEventType::ContextBuilt => "Context 构建",
        AwarenessEventType::DeltaMerged => "Delta 合并",
        AwarenessEventType::SyncPointMerged => "同步点合并",
        AwarenessEventType::SyncPointReported => "同步点",
        AwarenessEventType::Finalized => "Finalized",
        AwarenessEventType::Rejected => "Rejected",
        AwarenessEventType::LateReceiptObserved => "迟到回执",
        AwarenessEventType::EnvironmentSnapshotRecorded => "环境快照",
    }
}

fn event_degradation(event: &DialogueEvent) -> Option<String> {
    if let Some(result) = event.tool_result.as_ref() {
        if let Some(reason) = result.degradation_reason.as_ref() {
            return Some(reason.to_string());
        }
    }
    event
        .metadata
        .get("degradation_reason")
        .and_then(|value| value.as_str())
        .map(|reason| reason.to_string())
}

fn timeline_to_json(timeline: &TimelineState) -> String {
    match serde_json::to_string_pretty(&json!({
        "events": timeline.events,
        "awareness": timeline.awareness,
        "tags": timeline.tags,
    })) {
        Ok(content) => content,
        Err(_) => "{}".into(),
    }
}

fn timeline_to_csv(timeline: &TimelineState) -> String {
    let mut rows = Vec::new();
    rows.push("kind,event_id,timestamp,scenario,summary,degradation,tags".to_string());

    for event in &timeline.events {
        let event_id = event.event_id.as_u64();
        let scenario_label = scenario_title(&event.scenario);
        let summary = event.metadata.to_string();
        let degradation = event_degradation(event)
            .map(|value| format_filter_label(&value))
            .unwrap_or_default();
        let tags = timeline
            .tags
            .get(&event_id)
            .map(|list| list.join("|"))
            .unwrap_or_default();
        rows.push(
            vec![
                "event".to_string(),
                event_id.to_string(),
                event.timestamp_ms.to_string(),
                scenario_label.to_string(),
                summary,
                degradation,
                tags,
            ]
            .into_iter()
            .map(|value| csv_escape(&value))
            .collect::<Vec<_>>()
            .join(","),
        );
    }

    for item in &timeline.awareness {
        let event_id = item.event_id.as_u64();
        let summary = item.payload.to_string();
        let degradation = item
            .degradation_reason
            .as_ref()
            .map(|reason| format_filter_label(&format!("{:?}", reason)))
            .unwrap_or_default();
        rows.push(
            vec![
                "awareness".to_string(),
                event_id.to_string(),
                item.occurred_at_ms.to_string(),
                "-".to_string(),
                summary,
                degradation,
                String::new(),
            ]
            .into_iter()
            .map(|value| csv_escape(&value))
            .collect::<Vec<_>>()
            .join(","),
        );
    }
    rows.join(
        "
",
    )
}

fn csv_escape(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        let escaped = value.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        value.to_string()
    }
}

#[cfg(target_arch = "wasm32")]
fn copy_text_to_clipboard(actions: AppActions, label: &str, target: &str, content: String) {
    let label_text = label.to_string();
    let target_text = target.to_string();
    actions.record_audit_event(AuditActionKind::Copy, label_text.clone(), target_text);
    let actions_clone = actions.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let window = web_sys::window().ok_or(())?;
            let clipboard = window.navigator().clipboard();
            let promise = clipboard.write_text(&content);
            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map(|_| ())
                .map_err(|_| ())
        }
        .await;

        match result {
            Ok(_) => actions_clone.set_operation_success(format!("{label_text} 已复制")),
            Err(_) => actions_clone.set_operation_error(format!("{label_text} 复制失败")),
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_text_to_clipboard(actions: AppActions, label: &str, target: &str, _content: String) {
    actions.record_audit_event(AuditActionKind::Copy, label.to_string(), target.to_string());
    actions.set_operation_success(format!("{label} 已复制（模拟）"));
}
