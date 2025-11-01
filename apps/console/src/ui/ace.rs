use crate::hooks::ace::use_ace_cycles;
use crate::models::{
    AceCycleStatus, AceCycleSummary, AceLane, AwarenessEventType, CycleSnapshotView,
    OutboxMessageView, RouterDecisionView,
};
use crate::state::{use_app_actions, use_app_state, AppActions};
use dioxus::prelude::*;
use serde_json::to_string_pretty;

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
                CycleListItem {
                    key: cycle.cycle_id.clone(),
                    cycle,
                    selected_cycle_id: props.selected_cycle_id.clone(),
                    actions: props.actions.clone(),
                    snapshot_loading: props.snapshot_loading,
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
}

impl PartialEq for CycleDetailProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for CycleDetailProps {}

#[component]
fn CycleDetail(props: CycleDetailProps) -> Element {
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
    let manifest_digest = snapshot
        .sync_point
        .context_manifest
        .get("manifest_digest")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let sync_event_count = snapshot.sync_point.events.len();
    let pending_injections = snapshot.sync_point.pending_injections.len();

    let created_at = snapshot.schedule.created_at.clone();
    let sync_point_kind = format!("{:?}", snapshot.sync_point.kind);
    let outcomes = snapshot.outcomes.clone();

    rsx! {
        div { class: "md:w-2/3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-3",
            h3 { class: "text-sm font-semibold text-slate-800", "周期详情" }
            ul { class: "space-y-1 text-xs text-slate-600",
                li { "周期 ID: {cycle.cycle_id}" }
                li { "Lane: {format_lane(&cycle.lane)}" }
                li { "状态: {format_status(&cycle.status)}" }
                li { "创建时间: {created_at}" }
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
            div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-700 space-y-1",
                h4 { class: "text-xs font-semibold text-slate-800", "同步点" }
                p { "类型: {sync_point_kind}" }
                if let Some(ref digest) = manifest_digest {
                    p { "Manifest Digest: {digest}" }
                }
                p { "事件数量: {sync_event_count}" }
                p { "待处理注入: {pending_injections}" }
                if !snapshot.sync_point.pending_injections.is_empty() {
                    ul { class: "list-disc pl-4 text-[11px] text-slate-600",
                        for item in snapshot.sync_point.pending_injections.iter() {
                            li { "优先级 {item.priority} - {item.author_role}" }
                        }
                    }
                }
            }
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
            div { class: "rounded border border-slate-100 bg-white p-3 text-xs text-slate-700 space-y-1",
                h4 { class: "text-xs font-semibold text-slate-800", "Outbox" }
                if outbox.is_empty() {
                    p { class: "text-xs text-slate-500", "暂无 Outbox 信息" }
                } else {
                    ul { class: "space-y-1",
                        for item in outbox.iter() {
                            li { class: "rounded border border-slate-200 bg-slate-50 p-2",
                                span { class: "block font-semibold", "{awareness_event_label(&item.payload.event_type)}" }
                                span { class: "block text-[11px] text-slate-500", "Event ID: {item.event_id}" }
                                span { class: "block text-[11px] text-slate-500", "Cycle ID: {item.cycle_id}" }
                            }
                        }
                    }
                }
            }
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
    let fork_label = format!("{:?}", decision.decision_path.fork);
    let confidence = format!("{:.2}", decision.decision_path.confidence);
    let plan_json = to_pretty_json(&decision.plan);
    let rejected = decision.rejected.clone();

    rsx! {
        div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-700 space-y-2",
            h4 { class: "text-xs font-semibold text-slate-800", "决策信息" }
            p { class: "text-xs", "Context Digest: {decision.context_digest}" }
            p { class: "text-xs", "Issued At: {decision.issued_at}" }
            div { class: "space-y-1",
                p { class: "text-xs", "Fork: {fork_label}" }
                p { class: "text-xs", "Confidence: {confidence}" }
            }
            if !rejected.is_empty() {
                ul { class: "text-xs text-slate-600 space-y-1",
                    li { class: "font-semibold", "Rejected:" }
                    for (code, reason) in rejected.iter() {
                        li { class: "pl-2", "- {code}: {reason}" }
                    }
                }
            }
            if !decision.plan.is_null() {
                pre { class: "overflow-x-auto rounded bg-slate-900 p-3 text-[11px] text-slate-100", "{plan_json}" }
            }
        }
    }
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
