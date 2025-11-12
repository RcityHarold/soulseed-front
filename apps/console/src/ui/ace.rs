use crate::hooks::ace::use_ace_cycles;
use crate::models::{
    AceCycleStatus, AceCycleSummary, AceLane, AwarenessDegradationReason, AwarenessEvent,
    AwarenessEventType, AwarenessFork, BudgetSnapshotView, CycleSnapshotView, DialogueEvent,
    HitlInjectionView, OutboxMessageView, RouteBudgetEstimate, RouterDecisionView,
    SyncPointInputView,
};
use crate::state::{use_app_actions, use_app_state, AppActions, AuditActionKind};
use dioxus::prelude::*;
use serde_json::{to_string_pretty, Value};
use std::collections::HashMap;
use std::rc::Rc;
use time::{Duration, OffsetDateTime};

#[component]
pub fn AcePanel() -> Element {
    use_ace_cycles();

    let actions = use_app_actions();
    let ace_state = use_app_state().read().ace.clone();

    let selected_cycle_id = ace_state.selected_cycle_id.clone();
    let selected_snapshot = selected_cycle_id
        .as_ref()
        .and_then(|id| ace_state.snapshots.get(id).cloned());
    let selected_outbox = selected_cycle_id
        .as_ref()
        .and_then(|id| ace_state.outboxes.get(id).cloned())
        .unwrap_or_default();
    let snapshot_loading = ace_state.snapshot_loading;
    let snapshot_error = ace_state.snapshot_error.clone();

    let body = if ace_state.is_loading {
        rsx! { p { class: "text-xs text-slate-500", "正在加载 ACE 周期..." } }
    } else if let Some(ref err) = ace_state.error {
        rsx! { p { class: "text-xs text-red-500", "ACE 数据加载失败: {err}" } }
    } else if ace_state.cycles.is_empty() {
        rsx! { p { class: "text-xs text-slate-500 italic", "暂无 ACE 周期数据" } }
    } else {
        let selected_cycle = ace_state
            .cycles
            .iter()
            .find(|cycle| Some(&cycle.cycle_id) == ace_state.selected_cycle_id.as_ref())
            .cloned();

        rsx! {
            div { class: "flex flex-col gap-4 md:flex-row",
                CycleList {
                    cycles: ace_state.cycles.clone(),
                    selected_cycle_id: ace_state.selected_cycle_id.clone(),
                    actions: actions.clone(),
                    snapshot_loading,
                }
                CycleDetail {
                    cycle: selected_cycle,
                    snapshot: selected_snapshot,
                    outbox: selected_outbox,
                    snapshot_loading,
                    snapshot_error,
                    actions: actions.clone(),
                }
            }
        }
    };

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "ACE 周期与 HITL" }
                p { class: "text-xs text-slate-500", "展示 Clarify/Tool/SelfReason 等周期的预算与当前状态" }
            }
            {body}
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct CycleListProps {
    cycles: Vec<AceCycleSummary>,
    selected_cycle_id: Option<String>,
    actions: AppActions,
    snapshot_loading: bool,
}

impl PartialEq for CycleListProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for CycleListProps {}

#[component]
fn CycleList(props: CycleListProps) -> Element {
    rsx! {
        div { class: "md:w-1/3 space-y-2",
            for cycle in props.cycles.iter().cloned() {
                {
                    let cycle_id = cycle.cycle_id.clone();
                    rsx! {
                        CycleListItem {
                            key: cycle_id,
                            cycle,
                            selected_cycle_id: props.selected_cycle_id.clone(),
                            actions: props.actions.clone(),
                            snapshot_loading: props.snapshot_loading,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct CycleListItemProps {
    cycle: AceCycleSummary,
    selected_cycle_id: Option<String>,
    actions: AppActions,
    snapshot_loading: bool,
}

impl PartialEq for CycleListItemProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for CycleListItemProps {}

#[component]
fn CycleListItem(props: CycleListItemProps) -> Element {
    let is_active = props
        .selected_cycle_id
        .as_ref()
        .map(|selected| selected == &props.cycle.cycle_id)
        .unwrap_or(false);

    let card_class = if is_active {
        "border-slate-900 bg-slate-900 text-white"
    } else {
        "border-slate-200 bg-white text-slate-800 hover:border-slate-400"
    };

    let cycle_id = props.cycle.cycle_id.clone();
    let actions = props.actions.clone();

    rsx! {
        button {
            class: format!(
                "w-full rounded-lg border px-3 py-2 text-left text-xs shadow-sm transition-colors {}",
                card_class
            ),
            onclick: move |_| actions.select_ace_cycle(Some(cycle_id.clone())),
            div { class: "flex items-center justify-between",
                span { class: "font-semibold", "{props.cycle.cycle_id}" }
                span { class: "text-[11px]", "{format_status(&props.cycle.status)}" }
                if is_active && props.snapshot_loading {
                    span { class: "text-[11px] text-amber-400", "加载中…" }
                }
            }
            p { class: "mt-1", "Lane: {format_lane(&props.cycle.lane)}" }
            if let Some(budget) = props.cycle.budget.as_ref() {
                div { class: "mt-1 flex flex-wrap gap-2 text-[11px]",
                    span { class: "rounded bg-violet-100 px-2 py-0.5 text-violet-700 font-mono", "Tokens {budget.tokens_spent.unwrap_or(0)} / {budget.tokens_allowed.unwrap_or(0)}" }
                    if let Some(allowed) = budget.walltime_ms_allowed {
                        span { class: "rounded bg-indigo-100 px-2 py-0.5 text-indigo-700 font-mono", "Wall {budget.walltime_ms_used.unwrap_or(0)} / {allowed} ms" }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct CycleDetailProps {
    cycle: Option<AceCycleSummary>,
    snapshot: Option<CycleSnapshotView>,
    outbox: Vec<OutboxMessageView>,
    snapshot_loading: bool,
    snapshot_error: Option<String>,
    actions: AppActions,
}

impl PartialEq for CycleDetailProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for CycleDetailProps {}

#[component]
fn CycleDetail(props: CycleDetailProps) -> Element {
    let actions = props.actions.clone();
    let Some(cycle) = props.cycle.clone() else {
        return rsx! {
            div { class: "md:w-2/3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                p { class: "text-xs text-slate-500 italic", "请选择一个 ACE 周期" }
            }
        };
    };

    if props.snapshot_loading {
        return rsx! {
            div { class: "md:w-2/3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                p { class: "text-xs text-slate-500", "正在加载周期 {cycle.cycle_id} 的详情..." }
            }
        };
    }

    if let Some(err) = props.snapshot_error.clone() {
        return rsx! {
            div { class: "md:w-2/3 rounded-lg border border-red-200 bg-red-50 p-4 shadow-sm",
                p { class: "text-xs text-red-600", "无法获取周期详情: {err}" }
            }
        };
    }

    let snapshot = match props.snapshot.clone() {
        Some(snapshot) => snapshot,
        None => {
            return rsx! {
                div { class: "md:w-2/3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                    p { class: "text-xs text-slate-500", "尚未加载周期 {cycle.cycle_id} 的快照" }
                }
            };
        }
    };

    let outbox = props.outbox.clone();
    let router_decision = snapshot.schedule.router_decision.clone();
    let created_at_str = format_offset_datetime_array(&snapshot.schedule.created_at);
    let outcomes = snapshot.outcomes.clone();

    rsx! {
        div { class: "md:w-2/3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-3",
            h3 { class: "text-sm font-semibold text-slate-800", "周期详情" }
            ul { class: "space-y-1 text-xs text-slate-600",
                li { "周期 ID: {cycle.cycle_id}" }
                li { "Lane: {format_lane(&cycle.lane)}" }
                li { "状态: {format_status(&cycle.status)}" }
                li { "创建时间: {created_at_str}" }
            }
            if let Some(budget) = cycle.budget.as_ref() {
                div { class: "flex flex-wrap gap-2 text-[11px]",
                    span { class: "rounded bg-violet-100 px-2 py-0.5 text-violet-700 font-mono", "Tokens {budget.tokens_spent.unwrap_or(0)} / {budget.tokens_allowed.unwrap_or(0)}" }
                    if let Some(allowed) = budget.walltime_ms_allowed {
                        span { class: "rounded bg-indigo-100 px-2 py-0.5 text-indigo-700 font-mono", "Wall {budget.walltime_ms_used.unwrap_or(0)} / {allowed} ms" }
                    }
                }
            }
            if let Some(decision) = router_decision.as_ref() {
                { render_router_decision(decision) }
            }
            { render_sync_point_section(&snapshot.sync_point, actions.clone()) }
            if !outcomes.is_empty() {
                div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-700 space-y-1",
                    h4 { class: "text-xs font-semibold text-slate-800", "Outcome 记录" }
                    for outcome in outcomes.iter() {
                        p { "# {outcome.cycle_id} => {outcome.status} ({outcome.manifest_digest.clone().unwrap_or_default()})" }
                    }
                }
            }
            if let Some(metadata) = cycle.metadata.as_ref() {
                div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-600 break-words",
                    "metadata: {metadata}"
                }
            }
            { render_outbox_section(&outbox, actions.clone()) }
        }
    }
}

fn format_lane(lane: &AceLane) -> &'static str {
    match lane {
        AceLane::Clarify => "Clarify",
        AceLane::Tool => "Tool",
        AceLane::SelfReason => "Self Reason",
        AceLane::Collab => "Collab",
    }
}

fn to_pretty_json(value: &serde_json::Value) -> String {
    to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn awareness_event_label(event_type: &AwarenessEventType) -> String {
    format!("{:?}", event_type)
}

fn render_router_decision(decision: &RouterDecisionView) -> Element {
    #[derive(Clone)]
    struct ForkRow {
        label: &'static str,
        score_text: String,
        components: Option<String>,
        degradation: String,
        budget: String,
        is_selected: bool,
        is_alert: bool,
    }

    let plan = &decision.plan;
    let fork_scores = extract_fork_scores(&plan.explain.diagnostics);
    let degradation_map = collect_degradation_reasons(decision);

    // 从 decision_path (Value 类型) 中提取 budget_plan
    let budget_plan_value = decision.decision_path.get("budget_plan").cloned().unwrap_or(Value::Null);
    let selected_budget_text = format_route_budget(&plan.budget, &budget_plan_value);

    let fork_rows: Vec<ForkRow> = ROUTER_FORKS
        .iter()
        .map(|fork| {
            let score_info = fork_scores.get(fork);
            let score_text = score_info
                .map(|info| format!("{:.2}", info.score))
                .unwrap_or_else(|| "--".into());
            let components = score_info.and_then(|info| summarize_components(&info.components));
            let degradation = degradation_map
                .get(fork)
                .cloned()
                .unwrap_or_else(|| "正常".into());
            let is_selected = *fork == plan.fork;
            let budget = if is_selected {
                selected_budget_text.clone()
            } else {
                "N/A".into()
            };
            let is_alert = degradation != "正常";
            ForkRow {
                label: fork_display_name(fork),
                score_text,
                components,
                degradation,
                budget,
                is_selected,
                is_alert,
            }
        })
        .collect();

    let indices_text = if plan.explain.indices_used.is_empty() {
        None
    } else {
        Some(plan.explain.indices_used.join(", "))
    };

    let query_hash = plan.explain.query_hash.clone();
    let selected_degradation = degradation_map.get(&plan.fork).cloned();

    let plan_detail_json = serde_json::to_value(&plan.decision_plan)
        .ok()
        .map(|value| to_pretty_json(&value));
    let diagnostics_json = if plan.explain.diagnostics.is_null() {
        None
    } else {
        Some(to_pretty_json(&plan.explain.diagnostics))
    };
    let explain_rejected = if plan.explain.rejected.is_empty() {
        None
    } else {
        Some(plan.explain.rejected.clone())
    };
    let rejected = decision.rejected.clone();
    let priority_text = format!("{:.2}", plan.priority);

    // 从 decision_path (Value 类型) 中提取 confidence
    let confidence = decision.decision_path.get("confidence")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(0.0);
    let confidence_text = format!("{:.2}", confidence);
    let issued_at_str = format_offset_datetime_array(&decision.issued_at);

    rsx! {
        div { class: "space-y-3 rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-700",
            h4 { class: "text-xs font-semibold text-slate-800", "决策信息" }
            div { class: "grid gap-2 md:grid-cols-2",
                div { class: "space-y-1",
                    p { "上下文摘要: {decision.context_digest}" }
                    p { "发出时间: {issued_at_str}" }
                    p { "当前权重: {priority_text}" }
                    if let Some(ref degrade) = selected_degradation {
                        p { class: "text-amber-600", "降级: {degrade}" }
                    }
                }
                div { class: "space-y-1",
                    p { "路由指纹: {plan.explain.router_digest}" }
                    p { "配置指纹: {plan.explain.router_config_digest}" }
                    p { "Routing Seed: {plan.explain.routing_seed}" }
                    if let Some(indices) = indices_text {
                        p { "Indices: {indices}" }
                    }
                    if let Some(hash) = query_hash {
                        p { "Query Hash: {hash}" }
                    }
                }
            }
            div { class: "space-y-2",
                h5 { class: "text-[11px] font-semibold text-slate-800 uppercase tracking-wide", "四重分叉权重" }
                ul { class: "grid gap-2 sm:grid-cols-2",
                    for row in fork_rows.iter() {
                        li { class: format!("rounded border px-3 py-2 shadow-sm {}", if row.is_selected { "border-slate-900 bg-white" } else { "border-slate-200 bg-white" }),
                            div { class: "flex items-center justify-between gap-2",
                                span { class: "font-semibold", "{row.label}" }
                                if row.is_selected {
                                    span { class: "rounded bg-slate-900 px-2 py-0.5 text-[11px] font-semibold text-white", "已选" }
                                }
                            }
                            p { class: "text-[11px] text-slate-600", "权重: {row.score_text}" }
                            p { class: "text-[11px] text-slate-600", "预算: {row.budget}" }
                            p { class: if row.is_alert { "text-[11px] text-amber-600" } else { "text-[11px] text-slate-500" }, "降级: {row.degradation}" }
                            if let Some(ref comp) = row.components {
                                p { class: "text-[10px] text-slate-500", "贡献: {comp}" }
                            }
                        }
                    }
                }
            }
            div { class: "space-y-1 rounded border border-slate-100 bg-white p-3",
                h5 { class: "text-[11px] font-semibold text-slate-800 uppercase tracking-wide", "DecisionPath" }
                // 从 decision_path (Value 类型) 中提取 fork
                if let Some(fork_str) = decision.decision_path.get("fork").and_then(|v| v.as_str()) {
                    if let Ok(fork) = serde_json::from_value::<AwarenessFork>(Value::String(fork_str.to_string())) {
                        p { "分叉: {fork_display_name(&fork)}" }
                    }
                }
                p { "信心: {confidence_text}" }
                // budget_plan_value 已经在前面提取了
                if let Some(text) = format_decision_budget_estimate(&budget_plan_value) {
                    p { "预算估计: {text}" }
                }
                // 从 decision_path (Value 类型) 中提取 degradation_reason
                if let Some(reason_value) = decision.decision_path.get("degradation_reason") {
                    if let Ok(reason) = serde_json::from_value::<AwarenessDegradationReason>(reason_value.clone()) {
                        p { class: "text-[11px] text-amber-600", "降级: {awareness_degradation_label(&reason)}" }
                    }
                }
            }
            if let Some(json) = plan_detail_json {
                details { class: "rounded border border-slate-200 bg-white p-3",
                    summary { class: "cursor-pointer text-[11px] font-semibold text-slate-800", "DecisionPlan JSON" }
                    pre { class: "mt-2 overflow-x-auto rounded bg-slate-900 p-3 text-[11px] text-slate-100", "{json}" }
                }
            }
            if let Some(json) = diagnostics_json {
                details { class: "rounded border border-slate-200 bg-white p-3",
                    summary { class: "cursor-pointer text-[11px] font-semibold text-slate-800", "Diagnostics" }
                    pre { class: "mt-2 overflow-x-auto rounded bg-slate-900 p-3 text-[11px] text-slate-100", "{json}" }
                }
            }
            if !rejected.is_empty() {
                div { class: "rounded border border-amber-200 bg-amber-50 p-3 text-[11px] text-amber-700 space-y-1",
                    h5 { class: "font-semibold", "Rejected 候选" }
                    for (code, reason) in rejected.iter() {
                        p { "- {code}: {reason}" }
                    }
                }
            }
            if let Some(rejected) = explain_rejected {
                if !rejected.is_empty() {
                    div { class: "rounded border border-amber-200 bg-stone-50 p-3 text-[11px] text-amber-700 space-y-1",
                        h5 { class: "font-semibold", "Explain Rejected" }
                        for (code, reason) in rejected.iter() {
                            p { "- {code}: {reason}" }
                        }
                    }
                }
            }
        }
    }
}

const ROUTER_FORKS: &[AwarenessFork] = &[
    AwarenessFork::Clarify,
    AwarenessFork::ToolPath,
    AwarenessFork::SelfReason,
    AwarenessFork::Collab,
];

#[derive(Clone, Debug)]
struct ForkScoreInfo {
    score: f32,
    components: Value,
}

fn extract_fork_scores(value: &Value) -> HashMap<AwarenessFork, ForkScoreInfo> {
    let mut map = HashMap::new();
    let Some(obj) = value.as_object() else {
        return map;
    };
    let Some(scores) = obj.get("fork_scores").and_then(Value::as_object) else {
        return map;
    };
    for (key, entry) in scores {
        if let Some(fork) = fork_from_key(key) {
            let score = entry
                .get("score")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(0.0);
            let components = entry.get("components").cloned().unwrap_or(Value::Null);
            map.insert(fork, ForkScoreInfo { score, components });
        }
    }
    map
}

fn summarize_components(value: &Value) -> Option<String> {
    let obj = value.as_object()?;
    let mut parts = Vec::new();
    for (key, val) in obj.iter() {
        if let Some(num) = val.as_f64() {
            parts.push(format!("{} {:.2}", key, num));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

fn format_route_budget(
    budget: &RouteBudgetEstimate,
    decision_budget: &Value,
) -> String {
    let mut parts = vec![
        format!("tokens {}", budget.tokens),
        format!("wall {} ms", budget.walltime_ms),
    ];
    if budget.external_cost > 0.0 {
        parts.push(format!("cost {:.2}", budget.external_cost));
    }
    if let Some(plan_text) = format_decision_budget_estimate(decision_budget) {
        parts.push(format!("plan {}", plan_text));
    }
    parts.join(" | ")
}

fn format_decision_budget_estimate(budget: &Value) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(tokens) = budget.get("tokens").and_then(|v| v.as_u64()) {
        parts.push(format!("tokens ≤ {}", tokens));
    }
    if let Some(wall) = budget.get("walltime_ms").and_then(|v| v.as_u64()) {
        parts.push(format!("wall ≤ {} ms", wall));
    }
    if let Some(cost) = budget.get("external_cost").and_then(|v| v.as_f64()) {
        parts.push(format!("cost ≤ {:.2}", cost));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" / "))
    }
}

fn collect_degradation_reasons(decision: &RouterDecisionView) -> HashMap<AwarenessFork, String> {
    let mut map = HashMap::new();
    let plan = &decision.plan;

    if let Some(reason) = plan.explain.degradation_reason.as_ref() {
        if !reason.trim().is_empty() {
            map.insert(plan.fork, prettify_reason(reason));
        }
    }

    // 从 decision_path (Value 类型) 中提取 degradation_reason 和 fork
    if let Some(reason_value) = decision.decision_path.get("degradation_reason") {
        if let Ok(reason) = serde_json::from_value::<AwarenessDegradationReason>(reason_value.clone()) {
            if let Some(fork_str) = decision.decision_path.get("fork").and_then(|v| v.as_str()) {
                if let Ok(fork) = serde_json::from_value::<AwarenessFork>(Value::String(fork_str.to_string())) {
                    map.entry(fork)
                        .or_insert_with(|| awareness_degradation_label(&reason).to_string());
                }
            }
        }
    }

    if let Some((fork, reason)) = extract_budget_exceeded(&plan.explain.diagnostics) {
        map.entry(fork).or_insert(reason);
    }

    for (code, reason) in decision.rejected.iter() {
        if let Some(fork) = infer_fork_from_code(code) {
            map.entry(fork).or_insert_with(|| prettify_reason(reason));
        }
    }

    map
}

fn extract_budget_exceeded(diagnostics: &Value) -> Option<(AwarenessFork, String)> {
    let obj = diagnostics.as_object()?;
    let diag = obj.get("budget_exceeded")?.as_object()?;
    let fork_label = diag.get("fork").and_then(|v| v.as_str())?;
    let fork = fork_from_key(fork_label)?;
    let mut reason = diag
        .get("reason")
        .and_then(|v| v.as_str())
        .map(prettify_reason)
        .unwrap_or_else(|| "budget_exceeded".to_string());
    if let Some(origin) = diag.get("origin").and_then(|v| v.as_str()) {
        reason.push_str(&format!(" (origin {origin})"));
    }
    Some((fork, reason))
}

fn fork_display_name(fork: &AwarenessFork) -> &'static str {
    match fork {
        AwarenessFork::Clarify => "Clarify · 澄清",
        AwarenessFork::ToolPath => "Tool · 工具",
        AwarenessFork::SelfReason => "SelfReason · 自反",
        AwarenessFork::Collab => "Collab · 协作",
    }
}

fn awareness_degradation_label(reason: &AwarenessDegradationReason) -> &'static str {
    match reason {
        AwarenessDegradationReason::BudgetTokens => "预算 Token 超限",
        AwarenessDegradationReason::BudgetWalltime => "预算时间超限",
        AwarenessDegradationReason::BudgetExternalCost => "预算成本超限",
        AwarenessDegradationReason::EmptyCatalog => "候选集为空",
        AwarenessDegradationReason::PrivacyBlocked => "隐私阻断",
        AwarenessDegradationReason::InvalidPlan => "计划无效",
        AwarenessDegradationReason::ClarifyExhausted => "Clarify 耗尽",
        AwarenessDegradationReason::GraphDegraded => "Graph 降级",
        AwarenessDegradationReason::EnvctxDegraded => "EnvCtx 降级",
    }
}

fn fork_from_key(key: &str) -> Option<AwarenessFork> {
    match key {
        "clarify" => Some(AwarenessFork::Clarify),
        "tool" | "tool_path" => Some(AwarenessFork::ToolPath),
        "self" | "self_reason" => Some(AwarenessFork::SelfReason),
        "collab" => Some(AwarenessFork::Collab),
        _ => None,
    }
}

fn infer_fork_from_code(code: &str) -> Option<AwarenessFork> {
    let lower = code.to_ascii_lowercase();
    if lower.contains("clarify") {
        Some(AwarenessFork::Clarify)
    } else if lower.contains("tool") {
        Some(AwarenessFork::ToolPath)
    } else if lower.contains("self") {
        Some(AwarenessFork::SelfReason)
    } else if lower.contains("collab") {
        Some(AwarenessFork::Collab)
    } else {
        None
    }
}

fn prettify_reason(reason: &str) -> String {
    reason
        .replace('_', " ")
        .replace(':', " → ")
        .replace('|', " / ")
}

fn render_sync_point_section(sync_point: &SyncPointInputView, actions: AppActions) -> Element {
    let manifest_digest = sync_point
        .context_manifest
        .get("manifest_digest")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let segments_count = sync_point
        .context_manifest
        .get("segments")
        .and_then(|v| v.as_array())
        .map(|segments| segments.len());
    let timeframe_label = format_timeframe(&sync_point.timeframe);
    let budget_label = format_sync_point_budget(&sync_point.budget);
    let kind_label = format!("{:?}", sync_point.kind);

    // events 现在是 Value 类型
    let event_rows = build_sync_event_rows_from_value(&sync_point.events);
    let events_count = event_rows.len();
    let pending = sync_point.pending_injections.clone();
    let pending_count = pending.len();

    let events_json = serde_json::to_string_pretty(&sync_point.events)
        .ok()
        .map(Rc::new);
    let manifest_json = if sync_point.context_manifest.is_null() {
        None
    } else {
        serde_json::to_string_pretty(&sync_point.context_manifest)
            .ok()
            .map(Rc::new)
    };
    let pending_json = if pending.is_empty() {
        None
    } else {
        serde_json::to_string_pretty(&pending).ok().map(Rc::new)
    };
    let sync_point_json = serde_json::to_string_pretty(sync_point).ok().map(Rc::new);

    let events_filename = Rc::new(format!("syncpoint-{}-events.json", sync_point.cycle_id));
    let manifest_filename = Rc::new(format!("syncpoint-{}-manifest.json", sync_point.cycle_id));
    let pending_filename = Rc::new(format!("syncpoint-{}-pending.json", sync_point.cycle_id));
    let sync_point_filename = Rc::new(format!("syncpoint-{}.json", sync_point.cycle_id));

    let sync_point_buttons = sync_point_json.clone().map(|json| {
        let copy_actions = actions.clone();
        let export_actions = actions.clone();
        let json_for_copy = json.clone();
        let json_for_export = json.clone();
        let filename_for_export = sync_point_filename.clone();
        rsx! {
            div { class: "flex flex-wrap gap-2 pt-2",
                button {
                    class: "rounded bg-slate-900 px-2 py-1 text-[11px] font-semibold text-white hover:bg-slate-800",
                    onclick: move |_| copy_text_to_clipboard(copy_actions.clone(), "同步点 JSON", "ace:sync_point:full", (*json_for_copy).clone()),
                    "复制同步点 JSON"
                }
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| export_text_as_file(export_actions.clone(), "同步点 JSON", "ace:sync_point:full", (*filename_for_export).clone(), (*json_for_export).clone()),
                    "导出同步点 JSON"
                }
            }
        }
    });

    rsx! {
        div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-700 space-y-2",
            h4 { class: "text-xs font-semibold text-slate-800", "同步点" }
            ul { class: "space-y-1 text-xs text-slate-600",
                li { "类型: {kind_label}" }
                li { "时间窗口: {timeframe_label}" }
                li { "预算快照: {budget_label}" }
                if let Some(ref digest) = manifest_digest {
                    li { "Manifest Digest: {digest}" }
                }
                if let Some(count) = segments_count {
                    li { "Segments: {count}" }
                }
                li { "事件数量: {events_count}" }
                li { "待处理注入: {pending_count}" }
            }
            { render_sync_point_events_section(&event_rows, events_json.clone(), events_filename.clone(), actions.clone()) }
            { render_sync_point_pending_section(&pending, pending_json.clone(), pending_filename.clone(), actions.clone()) }
            { render_sync_point_manifest_section(&sync_point.context_manifest, manifest_json.clone(), manifest_filename.clone(), actions.clone()) }
            if let Some(buttons) = sync_point_buttons {
                {buttons}
            }
        }
    }
}

fn render_sync_point_events_section(
    rows: &[SyncEventRow],
    events_json: Option<Rc<String>>,
    filename: Rc<String>,
    actions: AppActions,
) -> Element {
    let summary_label = format!("事件列表 ({})", rows.len());
    let button_row = events_json.clone().map(|json| {
        let copy_actions = actions.clone();
        let export_actions = actions.clone();
        let json_for_copy = json.clone();
        let json_for_export = json.clone();
        let filename_for_export = filename.clone();
        rsx! {
            div { class: "flex flex-wrap gap-2",
                button {
                    class: "rounded bg-slate-900 px-2 py-1 text-[11px] font-semibold text-white hover:bg-slate-800",
                    onclick: move |_| copy_text_to_clipboard(copy_actions.clone(), "SyncPoint 事件 JSON", "ace:sync_point:events", (*json_for_copy).clone()),
                    "复制事件 JSON"
                }
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| export_text_as_file(export_actions.clone(), "SyncPoint 事件", "ace:sync_point:events", (*filename_for_export).clone(), (*json_for_export).clone()),
                    "导出事件 JSON"
                }
            }
        }
    });

    rsx! {
        details { class: "rounded border border-slate-200 bg-white p-3 text-xs text-slate-700 space-y-2",
            summary { class: "cursor-pointer text-[11px] font-semibold text-slate-800", "{summary_label}" }
            if rows.is_empty() {
                p { class: "text-[11px] text-slate-500", "暂无事件" }
            } else {
                div { class: "space-y-2",
                    for row in rows.iter() {
                        div { class: "rounded border border-slate-200 bg-slate-50 p-2 space-y-1",
                            div { class: "flex flex-wrap items-center justify-between gap-2",
                                span { class: "font-semibold", "Seq {row.sequence}" }
                                span { class: "text-[11px] text-slate-500", "{row.event_id}" }
                                span { class: "rounded bg-slate-900 px-2 py-0.5 text-[10px] font-semibold text-white", "{row.event_type}" }
                            }
                            if let Some(channel) = row.channel.as_ref() {
                                p { class: "text-[10px] text-slate-500", "Channel: {channel}" }
                            }
                            p { class: "text-[11px] text-slate-600 break-words", "{row.summary}" }
                        }
                    }
                }
            }
            if let Some(row) = button_row {
                {row}
            }
        }
    }
}

fn render_sync_point_pending_section(
    pending: &[HitlInjectionView],
    pending_json: Option<Rc<String>>,
    filename: Rc<String>,
    actions: AppActions,
) -> Element {
    let summary_label = format!("Pending HITL ({})", pending.len());
    let button_row = pending_json.clone().map(|json| {
        let copy_actions = actions.clone();
        let export_actions = actions.clone();
        let json_for_copy = json.clone();
        let json_for_export = json.clone();
        let filename_for_export = filename.clone();
        rsx! {
            div { class: "flex flex-wrap gap-2",
                button {
                    class: "rounded bg-slate-900 px-2 py-1 text-[11px] font-semibold text-white hover:bg-slate-800",
                    onclick: move |_| copy_text_to_clipboard(copy_actions.clone(), "Pending HITL JSON", "ace:sync_point:pending", (*json_for_copy).clone()),
                    "复制注入 JSON"
                }
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| export_text_as_file(export_actions.clone(), "Pending HITL JSON", "ace:sync_point:pending", (*filename_for_export).clone(), (*json_for_export).clone()),
                    "导出注入 JSON"
                }
            }
        }
    });

    rsx! {
        details { class: "rounded border border-slate-200 bg-white p-3 text-xs text-slate-700 space-y-2",
            summary { class: "cursor-pointer text-[11px] font-semibold text-slate-800", "{summary_label}" }
            if pending.is_empty() {
                p { class: "text-[11px] text-slate-500", "暂无待处理注入" }
            } else {
                div { class: "space-y-2",
                    for item in pending.iter() {
                        div { class: "rounded border border-slate-200 bg-slate-50 p-2 space-y-1",
                            p { class: "font-semibold", "{format_pending_injection_label(item)}" }
                            if !item.submitted_at.is_null() {
                                p { class: "text-[10px] text-slate-500", "Submitted At: {item.submitted_at}" }
                            }
                            pre { class: "overflow-x-auto rounded bg-slate-900 p-2 text-[10px] text-slate-100", "{to_pretty_json(&item.payload)}" }
                        }
                    }
                }
            }
            if let Some(row) = button_row {
                {row}
            }
        }
    }
}

fn render_sync_point_manifest_section(
    manifest: &Value,
    manifest_json: Option<Rc<String>>,
    filename: Rc<String>,
    actions: AppActions,
) -> Element {
    if manifest.is_null() {
        return rsx! {
            details { class: "rounded border border-slate-200 bg-white p-3 text-xs text-slate-700",
                summary { class: "cursor-pointer text-[11px] font-semibold text-slate-800", "Context Manifest" }
                p { class: "text-[11px] text-slate-500", "暂无 Context Manifest" }
            }
        };
    }

    let manifest_pretty = manifest_json.clone();
    let button_row = manifest_json.clone().map(|json| {
        let copy_actions = actions.clone();
        let export_actions = actions.clone();
        let json_for_copy = json.clone();
        let json_for_export = json.clone();
        let filename_for_export = filename.clone();
        rsx! {
            div { class: "flex flex-wrap gap-2",
                button {
                    class: "rounded bg-slate-900 px-2 py-1 text-[11px] font-semibold text-white hover:bg-slate-800",
                    onclick: move |_| copy_text_to_clipboard(copy_actions.clone(), "Manifest JSON", "ace:sync_point:manifest", (*json_for_copy).clone()),
                    "复制 Manifest"
                }
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| export_text_as_file(export_actions.clone(), "Manifest JSON", "ace:sync_point:manifest", (*filename_for_export).clone(), (*json_for_export).clone()),
                    "导出 Manifest"
                }
            }
        }
    });

    rsx! {
        details { class: "rounded border border-slate-200 bg-white p-3 text-xs text-slate-700 space-y-2",
            summary { class: "cursor-pointer text-[11px] font-semibold text-slate-800", "Context Manifest JSON" }
            if let Some(json) = manifest_pretty {
                pre { class: "overflow-x-auto rounded bg-slate-900 p-2 text-[10px] text-slate-100", "{(*json)}" }
            }
            if let Some(row) = button_row {
                {row}
            }
        }
    }
}
fn render_outbox_section(outbox: &[OutboxMessageView], actions: AppActions) -> Element {
    let summary_label = format!("Outbox 消息 ({})", outbox.len());
    let rows = build_outbox_rows(outbox);
    let button_row = serde_json::to_string_pretty(outbox)
        .ok()
        .map(Rc::new)
        .map(|json| {
            let copy_actions = actions.clone();
            let export_actions = actions.clone();
            let json_for_copy = json.clone();
            let json_for_export = json.clone();
            let filename = Rc::new("outbox-messages.json".to_string());
            rsx! {
                div { class: "flex flex-wrap gap-2",
                    button {
                        class: "rounded bg-slate-900 px-2 py-1 text-[11px] font-semibold text-white hover:bg-slate-800",
                        onclick: move |_| copy_text_to_clipboard(copy_actions.clone(), "Outbox JSON", "ace:outbox:list", (*json_for_copy).clone()),
                        "复制 Outbox JSON"
                    }
                    button {
                        class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                        onclick: move |_| export_text_as_file(export_actions.clone(), "Outbox JSON", "ace:outbox:list", (*filename).clone(), (*json_for_export).clone()),
                        "导出 Outbox JSON"
                    }
                }
            }
        });

    rsx! {
        details { class: "rounded border border-slate-200 bg-white p-3 text-xs text-slate-700 space-y-2",
            summary { class: "cursor-pointer text-[11px] font-semibold text-slate-800", "{summary_label}" }
            if rows.is_empty() {
                p { class: "text-[11px] text-slate-500", "暂无 Outbox 信息" }
            } else {
                div { class: "space-y-2",
                    for row in rows.iter().cloned() {
                        { render_outbox_row(row, actions.clone()) }
                    }
                }
            }
            if let Some(row) = button_row {
                {row}
            }
        }
    }
}
fn render_outbox_row(row: OutboxRow, actions: AppActions) -> Element {
    let timeline_actions = actions.clone();
    let event_for_timeline = row.event.clone();
    let event_id_label = row.event_id.clone();
    let event_type_label = row.event_type_label.clone();
    let occurred_at_label = row.occurred_at.clone();
    let payload_summary = row.payload_summary.clone();
    let degradation_label = row.degradation.clone();
    let cycle_id = row.cycle_id;

    let event_buttons = row.event_json.clone().map(|json| {
        let copy_actions = actions.clone();
        let export_actions = actions.clone();
        let json_for_copy = json.clone();
        let json_for_export = json.clone();
        let filename = Rc::new(format!("outbox-event-{}.json", event_id_label.clone()));
        rsx! {
            Fragment {
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| copy_text_to_clipboard(copy_actions.clone(), "Outbox Event JSON", "ace:outbox:event", (*json_for_copy).clone()),
                    "复制事件 JSON"
                }
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| export_text_as_file(export_actions.clone(), "Outbox Event JSON", "ace:outbox:event", (*filename).clone(), (*json_for_export).clone()),
                    "导出事件 JSON"
                }
            }
        }
    });

    let payload_buttons = row.payload_json.clone().map(|json| {
        let copy_actions = actions.clone();
        let export_actions = actions.clone();
        let json_for_copy = json.clone();
        let json_for_export = json.clone();
        let filename = Rc::new(format!("outbox-payload-{}.json", event_id_label.clone()));
        rsx! {
            Fragment {
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| copy_text_to_clipboard(copy_actions.clone(), "Outbox Payload JSON", "ace:outbox:payload", (*json_for_copy).clone()),
                    "复制 Payload"
                }
                button {
                    class: "rounded border border-slate-300 px-2 py-1 text-[11px] text-slate-700 hover:bg-slate-100",
                    onclick: move |_| export_text_as_file(export_actions.clone(), "Outbox Payload JSON", "ace:outbox:payload", (*filename).clone(), (*json_for_export).clone()),
                    "导出 Payload"
                }
            }
        }
    });

    rsx! {
        div { class: "rounded border border-slate-200 bg-slate-50 p-2 space-y-1",
            div { class: "flex flex-wrap items-center justify-between gap-2",
                span { class: "font-semibold", "{event_type_label}" }
                span { class: "text-[11px] text-slate-500", "# {event_id_label}" }
                span { class: "text-[11px] text-slate-500", "{occurred_at_label}" }
            }
            p { class: "text-[10px] text-slate-500", "Cycle ID: {cycle_id}" }
            if let Some(degrade) = degradation_label {
                p { class: "text-[10px] text-amber-600", "降级: {degrade}" }
            }
            p { class: "text-[11px] text-slate-600 break-words", "{payload_summary}" }
            div { class: "flex flex-wrap gap-2 pt-1",
                button {
                    class: "rounded bg-emerald-600 px-2 py-1 text-[11px] font-semibold text-white hover:bg-emerald-500",
                    onclick: move |_| {
                        let event_id_for_msg = event_id_label.clone();
                        // 尝试将 Value 反序列化为 AwarenessEvent
                        match serde_json::from_value::<AwarenessEvent>(event_for_timeline.clone()) {
                            Ok(awareness_event) => {
                                timeline_actions.append_timeline(Vec::new(), vec![awareness_event], None);
                                timeline_actions.set_operation_success(format!("Outbox 事件 {} 已写入时间线", event_id_for_msg));
                            }
                            Err(e) => {
                                timeline_actions.set_operation_error(format!("无法将事件写入时间线: {}", e));
                            }
                        }
                    },
                    "写入时间线"
                }
                if let Some(buttons) = event_buttons {
                    {buttons}
                }
                if let Some(buttons) = payload_buttons {
                    {buttons}
                }
            }
        }
    }
}

#[derive(Clone)]
struct OutboxRow {
    event: Value,
    event_id: String,
    cycle_id: u64,
    occurred_at: String,
    event_type_label: String,
    degradation: Option<String>,
    payload_summary: String,
    payload_json: Option<Rc<String>>,
    event_json: Option<Rc<String>>,
}

fn build_outbox_rows(outbox: &[OutboxMessageView]) -> Vec<OutboxRow> {
    use soulseed_agi_core_models::AwarenessCycleId;
    use std::str::FromStr;

    outbox
        .iter()
        .map(|item| {
            let event = item.payload.clone();

            // 从 Value 中提取字段
            let event_id = event
                .get("event_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let event_type_str = event
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let event_type_label = event_type_str.to_string();

            let occurred_at_ms = event
                .get("occurred_at_ms")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let occurred_at = format_timestamp_ms(occurred_at_ms);

            let degradation = event
                .get("degradation_reason")
                .and_then(|v| serde_json::from_value::<AwarenessDegradationReason>(v.clone()).ok())
                .map(|reason| awareness_degradation_label(&reason).to_string());

            let payload_value = event.get("payload").cloned().unwrap_or(Value::Null);
            let payload_summary = summarize_outbox_payload(&payload_value);
            let payload_json = serde_json::to_string_pretty(&payload_value)
                .ok()
                .map(Rc::new);
            let event_json = serde_json::to_string_pretty(&event).ok().map(Rc::new);

            // 将 Base36 字符串转换为 u64
            let cycle_id = AwarenessCycleId::from_str(&item.cycle_id)
                .map(|id| id.as_u64())
                .unwrap_or_else(|_| {
                    // 如果解析失败，尝试直接解析为 u64
                    item.cycle_id.parse::<u64>().unwrap_or(0)
                });

            OutboxRow {
                event,
                event_id,
                cycle_id,
                occurred_at,
                event_type_label,
                degradation,
                payload_summary,
                payload_json,
                event_json,
            }
        })
        .collect()
}

fn summarize_outbox_payload(value: &Value) -> String {
    if value.is_null() {
        return "-".into();
    }
    if let Some(obj) = value.as_object() {
        if let Some(kind) = obj.get("kind").and_then(|v| v.as_str()) {
            return kind.to_string();
        }
        if let Some(content) = obj
            .get("message")
            .and_then(|v| v.as_object())
            .and_then(|msg| msg.get("content"))
            .and_then(|v| v.as_str())
        {
            return truncate_text(content, 96);
        }
        if let Some(summary) = obj.get("summary").and_then(|v| v.as_str()) {
            return truncate_text(summary, 96);
        }
        if let Some(status) = obj.get("status").and_then(|v| v.as_str()) {
            return status.to_string();
        }
        let keys: Vec<_> = obj.keys().take(3).cloned().collect();
        if !keys.is_empty() {
            return format!("{{{}}}", keys.join(", "));
        }
    }
    truncate_text(&value.to_string(), 96)
}

fn format_timestamp_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let millis = ms.rem_euclid(1000);
    if let Ok(dt) = OffsetDateTime::from_unix_timestamp(secs) {
        let dt = dt + Duration::milliseconds(millis);
        return format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}Z",
            dt.year(),
            u8::from(dt.month()),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second(),
            millis.abs()
        );
    }
    format!("{} ms", ms)
}

#[derive(Clone)]
struct SyncEventRow {
    event_id: String,
    event_type: String,
    sequence: u64,
    channel: Option<String>,
    summary: String,
}

fn build_sync_event_rows(events: &[DialogueEvent]) -> Vec<SyncEventRow> {
    events
        .iter()
        .map(|event| {
            let event_id = event.event_id.to_base36();
            let event_type = format!("{:?}", event.event_type);
            let channel = event
                .metadata
                .get("channel")
                .and_then(|value| value.as_str())
                .map(|s| s.to_string());
            let summary = extract_event_summary(event);

            SyncEventRow {
                event_id,
                event_type,
                sequence: event.sequence_number,
                channel,
                summary,
            }
        })
        .collect()
}

fn build_sync_event_rows_from_value(events_value: &Value) -> Vec<SyncEventRow> {
    let Some(events) = events_value.as_array() else {
        return Vec::new();
    };

    events
        .iter()
        .filter_map(|event| {
            let event_id = event
                .get("event_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let event_type = event
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let sequence = event.get("sequence_number").and_then(|v| v.as_u64()).unwrap_or(0);
            let channel = event
                .get("metadata")
                .and_then(|m| m.get("channel"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let summary = extract_event_summary_from_value(event);

            Some(SyncEventRow {
                event_id,
                event_type,
                sequence,
                channel,
                summary,
            })
        })
        .collect()
}

fn extract_event_summary(event: &DialogueEvent) -> String {
    if let Some(text) = event.metadata.get("text").and_then(|value| value.as_str()) {
        return truncate_text(text, 96);
    }
    if let Some(note) = event.metadata.get("note").and_then(|value| value.as_str()) {
        return truncate_text(note, 96);
    }
    if let Some(kind) = event.metadata.get("kind").and_then(|value| value.as_str()) {
        return kind.to_string();
    }
    "-".into()
}

fn extract_event_summary_from_value(event: &Value) -> String {
    if let Some(metadata) = event.get("metadata") {
        if let Some(text) = metadata.get("text").and_then(|v| v.as_str()) {
            return truncate_text(text, 96);
        }
        if let Some(note) = metadata.get("note").and_then(|v| v.as_str()) {
            return truncate_text(note, 96);
        }
        if let Some(kind) = metadata.get("kind").and_then(|v| v.as_str()) {
            return kind.to_string();
        }
    }
    "-".into()
}

fn truncate_text(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(limit).collect::<String>())
    }
}

fn format_timeframe(timeframe: &Value) -> String {
    // timeframe 是 (OffsetDateTime, OffsetDateTime) 序列化后的嵌套数组
    // [[year, day_of_year, hour, minute, second, nanosecond, ...], [year, day_of_year, ...]]
    if let Some(arr) = timeframe.as_array() {
        if arr.len() >= 2 {
            let start_str = format_offset_datetime_array(&arr[0]);
            let end_str = format_offset_datetime_array(&arr[1]);
            return format!("{} → {}", start_str, end_str);
        }
    }
    "Unknown → Unknown".to_string()
}

fn format_offset_datetime_array(arr: &Value) -> String {
    // OffsetDateTime 数组: [year, day_of_year, hour, minute, second, nanosecond, ...]
    if let Some(components) = arr.as_array() {
        if components.len() >= 5 {
            let year = components[0].as_i64().unwrap_or(0);
            let day = components[1].as_i64().unwrap_or(0);
            let hour = components[2].as_i64().unwrap_or(0);
            let minute = components[3].as_i64().unwrap_or(0);
            let second = components[4].as_i64().unwrap_or(0);
            return format!("{}-{:03}T{:02}:{:02}:{:02}", year, day, hour, minute, second);
        }
    }
    "Unknown".to_string()
}

fn format_sync_point_budget(budget: &BudgetSnapshotView) -> String {
    format!(
        "tokens {}/{} | wall {} / {} ms | cost {:.2} / {:.2}",
        budget.tokens_spent,
        budget.tokens_allowed,
        budget.walltime_ms_used,
        budget.walltime_ms_allowed,
        budget.external_cost_spent,
        budget.external_cost_allowed
    )
}

fn format_pending_injection_label(item: &HitlInjectionView) -> String {
    format!("{} · {}", item.priority, item.author_role)
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

#[cfg(target_arch = "wasm32")]
fn export_text_as_file(
    actions: AppActions,
    label: &str,
    target: &str,
    filename: String,
    content: String,
) {
    let label_text = label.to_string();
    let target_text = target.to_string();
    actions.record_audit_event(AuditActionKind::Export, label_text.clone(), target_text);
    let actions_clone = actions.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = (|| -> Result<(), ()> {
            use js_sys::Array;
            use wasm_bindgen::JsCast;
            use wasm_bindgen::JsValue;

            let window = web_sys::window().ok_or(())?;
            let document = window.document().ok_or(())?;
            let body = document.body().ok_or(())?;

            let parts = Array::new();
            parts.push(&JsValue::from_str(&content));
            let blob = web_sys::Blob::new_with_str_sequence(&parts).map_err(|_| ())?;
            let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| ())?;

            let link = document
                .create_element("a")
                .map_err(|_| ())?
                .dyn_into::<web_sys::HtmlAnchorElement>()
                .map_err(|_| ())?;
            link.set_href(&url);
            link.set_download(&filename);
            let _ = link.set_attribute("style", "display: none");
            body.append_child(&link).map_err(|_| ())?;
            link.click();
            let _ = body.remove_child(&link);
            let _ = web_sys::Url::revoke_object_url(&url);
            Ok(())
        })();

        match result {
            Ok(_) => actions_clone.set_operation_success(format!("{label_text} 已导出")),
            Err(_) => actions_clone.set_operation_error(format!("{label_text} 导出失败")),
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn export_text_as_file(
    actions: AppActions,
    label: &str,
    target: &str,
    filename: String,
    _content: String,
) {
    actions.record_audit_event(
        AuditActionKind::Export,
        label.to_string(),
        target.to_string(),
    );
    actions.set_operation_success(format!("{label} 已导出（模拟）: {filename}"));
}

fn format_status(status: &AceCycleStatus) -> &'static str {
    match status {
        AceCycleStatus::Pending => "待执行",
        AceCycleStatus::Running => "运行中",
        AceCycleStatus::Completed => "已完成",
        AceCycleStatus::Failed => "失败",
        AceCycleStatus::Cancelled => "已取消",
    }
}
