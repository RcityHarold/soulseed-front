use crate::hooks::{live::use_live_stream, timeline::use_timeline_loader};
use crate::models::{
    AwarenessEvent, AwarenessEventType, ConversationScenario, DialogueEvent, DialogueEventType,
};
use crate::state::{
    normalize_filter_value, to_snake_case, use_app_actions, use_app_state, AppActions,
    TimelineFilters, TimelineState,
};
use crate::APP_CONFIG;
use dioxus::prelude::*;
use serde_json::json;
use std::collections::{BTreeSet, HashMap};

const TIMELINE_CONTAINER_CLASS: &str = "space-y-4";
const CHIP_BASE_CLASS: &str = "px-3 py-1 rounded-full border text-xs transition-colors";
const CHIP_ACTIVE_CLASS: &str = "bg-slate-900 text-white border-slate-900";
const CHIP_INACTIVE_CLASS: &str = "bg-white text-slate-700 border-slate-200 hover:border-slate-400";

#[derive(Clone)]
struct FilterOption {
    value: String,
    label: String,
}

pub fn TimelineView(cx: Scope) -> Element {
    use_timeline_loader(cx);
    use_live_stream(cx);

    let app_state = use_app_state(cx);
    let actions = use_app_actions(cx);

    let snapshot = app_state.read().clone();
    drop(app_state);

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

            ScenarioSwitcher { scenario_filter: scenario_filter.clone(), actions: actions.clone() }
            AuditToolbar { timeline: timeline.clone(), actions: actions.clone() }
            FilterToolbar {
                filters: filters.clone(),
                role_options: role_options,
                access_options: access_options,
                degradation_options: degradation_options,
                awareness_options: awareness_options,
                actions: actions.clone(),
            }

            TimelineColumn { timeline: timeline.clone(), filters: filters.clone(), tags: tags, actions: actions.clone() }
            AwarenessColumn { awareness: awareness, filters: filters }
        }
    }
}

#[derive(Clone, Props)]
struct ScenarioSwitcherProps {
    scenario_filter: Option<ConversationScenario>,
    actions: AppActions,
}

fn ScenarioSwitcher(cx: Scope<ScenarioSwitcherProps>) -> Element {
    let actions = cx.props.actions.clone();
    let current = cx.props.scenario_filter.clone();

    rsx! {
        div { class: "flex flex-wrap gap-2",
            for (idx, option) in scenario_options().into_iter().enumerate() {
                let is_active = current == option.scenario;
                let button_class = format!(
                    "{} {}",
                    CHIP_BASE_CLASS,
                    if is_active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                );
                let option_value = option.scenario.clone();
                let actions = actions.clone();

                button {
                    key: format!("scenario-{idx}"),
                    class: button_class,
                    onclick: move |_| actions.set_scenario(option_value.clone()),
                    span { class: "text-xs font-medium", "{option.label}" }
                }
            }
        }
    }
}

#[derive(Clone, Props)]
#[props(no_eq)]
struct AuditToolbarProps {
    timeline: TimelineState,
    actions: AppActions,
}

fn AuditToolbar(cx: Scope<AuditToolbarProps>) -> Element {
    let timeline = cx.props.timeline.clone();
    let actions = cx.props.actions.clone();

    let on_export_json = {
        let actions = actions.clone();
        let data = timeline.clone();
        move |_| {
            let content = timeline_to_json(&data);
            copy_text_to_clipboard(actions.clone(), "时间线 JSON", content);
        }
    };

    let on_export_csv = {
        let actions = actions.clone();
        let data = timeline.clone();
        move |_| {
            let content = timeline_to_csv(&data);
            copy_text_to_clipboard(actions.clone(), "时间线 CSV", content);
        }
    };

    let on_replay = {
        let actions = actions.clone();
        move |_| actions.playback_sample_timeline()
    };

    cx.render(rsx! {
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
    })
}

#[derive(Clone, Props)]
#[props(no_eq)]
struct FilterToolbarProps {
    filters: TimelineFilters,
    role_options: Vec<FilterOption>,
    access_options: Vec<FilterOption>,
    degradation_options: Vec<FilterOption>,
    awareness_options: Vec<FilterOption>,
    actions: AppActions,
}

fn FilterToolbar(cx: Scope<FilterToolbarProps>) -> Element {
    let filters = cx.props.filters.clone();
    let actions = cx.props.actions.clone();

    if cx.props.role_options.is_empty()
        && cx.props.access_options.is_empty()
        && cx.props.degradation_options.is_empty()
        && cx.props.awareness_options.is_empty()
    {
        return cx.render(rsx! {});
    }

    cx.render(rsx! {
        div { class: "space-y-2 rounded-lg border border-slate-200 bg-white p-4 shadow-sm text-xs text-slate-600",
            if !cx.props.role_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "参与者角色" }
                    div { class: "flex flex-wrap gap-2",
                        for option in cx.props.role_options.iter() {
                            let value = option.value.clone();
                            let is_active = filters.participant_roles.contains(&value);
                            let button_class = format!(
                                "{} {}",
                                CHIP_BASE_CLASS,
                                if is_active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                            );
                            let actions = actions.clone();
                            button {
                                key: format!("role-{}", value),
                                class: button_class,
                                onclick: move |_| actions.toggle_participant_role(&value),
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !cx.props.access_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "访问级别" }
                    div { class: "flex flex-wrap gap-2",
                        for option in cx.props.access_options.iter() {
                            let value = option.value.clone();
                            let is_active = filters.access_classes.contains(&value);
                            let button_class = format!(
                                "{} {}",
                                CHIP_BASE_CLASS,
                                if is_active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                            );
                            let actions = actions.clone();
                            button {
                                key: format!("access-{}", value),
                                class: button_class,
                                onclick: move |_| actions.toggle_access_class(&value),
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !cx.props.degradation_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "降级原因" }
                    div { class: "flex flex-wrap gap-2",
                        for option in cx.props.degradation_options.iter() {
                            let value = option.value.clone();
                            let is_active = filters.degradation_reasons.contains(&value);
                            let button_class = format!(
                                "{} {}",
                                CHIP_BASE_CLASS,
                                if is_active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                            );
                            let actions = actions.clone();
                            button {
                                key: format!("degrade-{}", value),
                                class: button_class,
                                onclick: move |_| actions.toggle_degradation_reason(&value),
                                span { class: "text-xs font-medium", "{option.label}" }
                            }
                        }
                    }
                }
            }

            if !cx.props.awareness_options.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "Awareness 类型" }
                    div { class: "flex flex-wrap gap-2",
                        for option in cx.props.awareness_options.iter() {
                            let value = option.value.clone();
                            let is_active = filters.awareness_types.contains(&value);
                            let button_class = format!(
                                "{} {}",
                                CHIP_BASE_CLASS,
                                if is_active { CHIP_ACTIVE_CLASS } else { CHIP_INACTIVE_CLASS }
                            );
                            let actions = actions.clone();
                            button {
                                key: format!("aware-{}", value),
                                class: button_class,
                                onclick: move |_| actions.toggle_awareness_type(&value),
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
    })
}

#[derive(Clone, Props)]
#[props(no_eq)]
struct TimelineColumnProps {
    timeline: TimelineState,
    filters: TimelineFilters,
    tags: HashMap<u64, Vec<String>>,
    actions: AppActions,
}

fn TimelineColumn(cx: Scope<TimelineColumnProps>) -> Element {
    let timeline = cx.props.timeline.clone();
    let filters = cx.props.filters.clone();
    let tags_map = cx.props.tags.clone();
    let actions = cx.props.actions.clone();

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
                        let event_id = event.event_id.0;
                        let event_tags = tags_map.get(&event_id).cloned().unwrap_or_default();
                        li {
                            key: format!("event-{event_id}"),
                            EventCard { event: event.clone(), tags: event_tags, actions: actions.clone() }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Props)]
#[props(no_eq)]
struct EventCardProps {
    event: DialogueEvent,
    tags: Vec<String>,
    actions: AppActions,
}

fn EventCard(cx: Scope<EventCardProps>) -> Element {
    let event = cx.props.event.clone();
    let tags = cx.props.tags.clone();
    let actions = cx.props.actions.clone();

    let event_id = event.event_id.0;
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
                .map(|p| {
                    let mut label = format!("{:?}", p.kind);
                    if let Some(role) = p.role.as_ref() {
                        label.push_str(&format!(" ({})", role));
                    }
                    label
                })
                .collect::<Vec<_>>()
                .join("、"),
        )
    };

    let tag_input = use_signal(cx, || String::new());

    let on_submit = {
        let actions = actions.clone();
        let tag_input = tag_input.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            let value = tag_input.read();
            let trimmed = value.trim();
            if trimmed.is_empty() {
                actions.set_operation_error("标签不能为空".into());
                return;
            }
            actions.add_event_tag(event_id, trimmed.to_string());
            actions.set_operation_success(format!("已为事件 #{event_id} 添加标签"));
            tag_input.set(String::new());
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
            if !tags.is_empty() {
                div { class: "flex flex-wrap gap-2",
                    for tag in tags.iter() {
                        let tag_value = tag.clone();
                        let actions = actions.clone();
                        span { class: "flex items-center gap-1 rounded-full bg-amber-100 px-2 py-1 text-[11px] text-amber-800",
                            "{tag}"
                            button {
                                class: "rounded bg-amber-200 px-1 text-[10px] text-amber-900",
                                onclick: move |_| actions.remove_event_tag(event_id, &tag_value),
                                "×"
                            }
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

#[derive(Clone, Props)]
#[props(no_eq)]
struct AwarenessColumnProps {
    awareness: Vec<AwarenessEvent>,
    filters: TimelineFilters,
}

fn AwarenessColumn(cx: Scope<AwarenessColumnProps>) -> Element {
    let awareness = cx.props.awareness.clone();
    let filters = cx.props.filters.clone();
    let filtered: Vec<_> = awareness
        .iter()
        .filter(|item| filters.matches_awareness(item))
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
                        let event_id = item.event_id.0;
                        let event_type = format_awareness_event_type(&item.event_type);
                        let timestamp = item.occurred_at_ms;
                        let degradation = item
                            .degradation_reason
                            .as_ref()
                            .map(|reason| format!("降级: {:?}", reason));

                        li {
                            key: format!("awareness-{event_id}"),
                            class: "rounded-lg border border-amber-200 bg-amber-50 p-3",
                            div { class: "flex items-center justify-between",
                                span { class: "text-xs font-medium text-amber-900", "{event_type}" }
                                span { class: "text-[11px] text-amber-700", "#{event_id} · {timestamp}" }
                            }
                            if let Some(reason) = degradation {
                                div { class: "mt-1 text-[11px] text-amber-700", "{reason}" }
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

fn LiveStatus(cx: Scope<LiveStatusProps>) -> Element {
    let status_text = if cx.props.connected {
        "实时流: 已连接"
    } else {
        "实时流: 未连接"
    };
    let status_class = if cx.props.connected {
        "text-xs text-green-600"
    } else {
        "text-xs text-slate-500"
    };

    rsx! {
        div { class: "flex flex-wrap items-center gap-2",
            span { class: status_class, "{status_text}" }
            if let Some(ref err) = cx.props.error {
                span { class: "text-xs text-red-500", "错误: {err}" }
            } else if let Some(id) = cx.props.last_event_id {
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
            scenario: Some(ConversationScenario::HumanToHuman),
            label: "人类 ↔ 人类",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::HumanGroup),
            label: "人类群组",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::HumanToAi),
            label: "人类 ↔ AI",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::AiToAi),
            label: "AI ↔ AI",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::AiSelfTalk),
            label: "AI 自对话",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::HumanToMultiAi),
            label: "人类 ↔ 多 AI",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::MultiHumanToMultiAi),
            label: "多 人类 ↔ 多 AI",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::AiGroup),
            label: "AI 群组",
        },
        ScenarioOption {
            scenario: Some(ConversationScenario::AiToSystem),
            label: "AI ↔ 系统",
        },
    ]
}
fn collect_role_options(events: &[DialogueEvent]) -> Vec<FilterOption> {
    let mut set = BTreeSet::new();
    for event in events {
        for participant in &event.participants {
            if let Some(role) = participant.role.as_ref() {
                let normalized = normalize_filter_value(role);
                let label = format_filter_label(role);
                set.insert((normalized, label));
            }
        }
    }
    set.into_iter()
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
        if let Some(result) = event.tool_result.as_ref() {
            if let Some(reason) = result.degradation_reason.as_ref() {
                let normalized = normalize_filter_value(&to_snake_case(reason));
                let label = format_filter_label(reason);
                set.insert((normalized, label));
            }
        }
        if let Some(reason) = event
            .metadata
            .get("degradation_reason")
            .and_then(|value| value.as_str())
        {
            let normalized = normalize_filter_value(&to_snake_case(reason));
            let label = format_filter_label(reason);
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
            result.push_str(&chars.as_str().to_lowercase());
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
        AwarenessEventType::AcStarted => "AC 启动",
        AwarenessEventType::IcStarted => "IC 启动",
        AwarenessEventType::IcEnded => "IC 完成",
        AwarenessEventType::AssessmentProduced => "Assessment",
        AwarenessEventType::DecisionRouted => "决策路由",
        AwarenessEventType::RouteReconsidered => "路径重审",
        AwarenessEventType::RouteSwitched => "路径切换",
        AwarenessEventType::ToolCalled => "工具调用",
        AwarenessEventType::ToolResponded => "工具响应",
        AwarenessEventType::ToolFailed => "工具失败",
        AwarenessEventType::CollabRequested => "协作请求",
        AwarenessEventType::CollabResolved => "协作完成",
        AwarenessEventType::ClarificationIssued => "Clarify 提问",
        AwarenessEventType::ClarificationAnswered => "Clarify 回答",
        AwarenessEventType::HumanInjectionReceived => "HITL 接收",
        AwarenessEventType::InjectionApplied => "注入应用",
        AwarenessEventType::InjectionDeferred => "注入延后",
        AwarenessEventType::InjectionIgnored => "注入忽略",
        AwarenessEventType::DeltaPatchGenerated => "DeltaPatch",
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
        let scenario_label = scenario_title(&event.scenario);
        let summary = event.metadata.to_string();
        let degradation = event_degradation(event)
            .map(|value| format_filter_label(&value))
            .unwrap_or_default();
        let tags = timeline
            .tags
            .get(&event.event_id.0)
            .map(|list| list.join("|"))
            .unwrap_or_default();
        rows.push(
            vec![
                "event".to_string(),
                event.event_id.0.to_string(),
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
        let summary = item.payload.to_string();
        let degradation = item
            .degradation_reason
            .as_ref()
            .map(|reason| format_filter_label(&format!("{:?}", reason)))
            .unwrap_or_default();
        rows.push(
            vec![
                "awareness".to_string(),
                item.event_id.0.to_string(),
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
    rows.join("\n")
}

fn csv_escape(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if value.contains([',', '"', '\n']) {
        let escaped = value.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        value.to_string()
    }
}

#[cfg(target_arch = "wasm32")]
fn copy_text_to_clipboard(actions: AppActions, label: &str, content: String) {
    let label = label.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let window = web_sys::window().ok_or(())?;
            let clipboard = window.navigator().clipboard().ok_or(())?;
            clipboard.write_text(&content).await.map_err(|_| ())
        }
        .await;

        match result {
            Ok(_) => actions.set_operation_success(format!("{label} 已复制")),
            Err(_) => actions.set_operation_error(format!("{label} 复制失败")),
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_text_to_clipboard(actions: AppActions, label: &str, _content: String) {
    actions.set_operation_success(format!("{label} 已复制（模拟）"));
}
