
use crate::hooks::ace::use_ace_cycles;
use crate::models::{AceCycleStatus, AceCycleSummary, AceLane};
use crate::state::{use_app_actions, use_app_state, AppActions};
use dioxus::prelude::*;

#[component]
pub fn AcePanel() -> Element {
    use_ace_cycles();

    let actions = use_app_actions();
    let ace_state = use_app_state().read().ace.clone();

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
                }
                CycleDetail { cycle: selected_cycle }
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

    rsx! {
        div { class: "md:w-2/3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-3",
            h3 { class: "text-sm font-semibold text-slate-800", "周期详情" }
            ul { class: "space-y-1 text-xs text-slate-600",
                li { "周期 ID: {cycle.cycle_id}" }
                li { "Lane: {format_lane(&cycle.lane)}" }
                li { "状态: {format_status(&cycle.status)}" }
            }
            if let Some(budget) = cycle.budget.as_ref() {
                div { class: "flex flex-wrap gap-2 text-[11px]",
                    span { class: "rounded bg-violet-100 px-2 py-0.5 text-violet-700 font-mono", "Tokens {budget.tokens_spent.unwrap_or(0)} / {budget.tokens_allowed.unwrap_or(0)}" }
                    if let Some(allowed) = budget.walltime_ms_allowed {
                        span { class: "rounded bg-indigo-100 px-2 py-0.5 text-indigo-700 font-mono", "Wall {budget.walltime_ms_used.unwrap_or(0)} / {allowed} ms" }
                    }
                }
            }
            if let Some(metadata) = cycle.metadata.as_ref() {
                div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-600 break-words",
                    "metadata: {metadata}"
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

fn format_status(status: &AceCycleStatus) -> &'static str {
    match status {
        AceCycleStatus::Pending => "待执行",
        AceCycleStatus::Running => "运行中",
        AceCycleStatus::Completed => "已完成",
        AceCycleStatus::Failed => "失败",
        AceCycleStatus::Cancelled => "已取消",
    }
}
