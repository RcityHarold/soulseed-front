
use std::collections::BTreeMap;

use crate::hooks::graph::use_graph_insights;
use crate::models::{CausalGraphNode, CausalGraphView, RecallResultView};
use crate::state::{use_app_actions, use_app_state};
use dioxus::prelude::*;

#[component]
pub fn GraphPanel() -> Element {
    use_graph_insights();

    let actions = use_app_actions();
    let snapshot = use_app_state().read().clone();
    let graph_state = snapshot.graph;
    let timeline_events = snapshot.timeline.events;

    let default_root = graph_state
        .query
        .root_event_id
        .or_else(|| timeline_events.first().map(|event| event.event_id.as_u64()));

    let mut root_input = use_signal(|| {
        graph_state
            .query
            .root_event_id
            .map(|id| id.to_string())
            .unwrap_or_default()
    });

    let current_root = graph_state.query.root_event_id;

    use_effect({
        let actions = actions.clone();
        let fallback = default_root;
        let current = current_root;
        move || {
            if current.is_none() {
                if let Some(root) = fallback {
                    actions.set_graph_root(Some(root));
                }
            }
        }
    });

    use_effect({
        let mut root_signal = root_input.clone();
        let desired_value = current_root.map(|id| id.to_string()).unwrap_or_default();
        move || {
            let current_value = root_signal.read().clone();
            if current_value != desired_value {
                root_signal.set(desired_value.clone());
            }
        }
    });

    let on_submit_root = {
        let actions = actions.clone();
        let root_input = root_input.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            let value = root_input.read();
            let trimmed = value.trim();
            if trimmed.is_empty() {
                actions.set_graph_error(Some("请输入 root_event_id".into()));
                return;
            }

            match trimmed.parse::<u64>() {
                Ok(root_id) => {
                    actions.set_graph_error(None);
                    actions.set_graph_root(Some(root_id));
                }
                Err(_) => {
                    actions.set_graph_error(Some("root_event_id 必须是数字".into()));
                }
            }
        }
    };

    let upstream = graph_state
        .causal
        .as_ref()
        .and_then(|graph| graph.edges.iter().find(|edge| edge.to == graph.root_event_id))
        .map(|edge| edge.from);

    let downstream: Vec<u64> = graph_state
        .causal
        .as_ref()
        .map(|graph| {
            graph
                .edges
                .iter()
                .filter(|edge| edge.from == graph.root_event_id)
                .map(|edge| edge.to)
                .collect()
        })
        .unwrap_or_default();

    let recall_section = render_recall_list(&graph_state.recall);

    rsx! {
        section { class: "space-y-4",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "图谱与回溯" }
                p { class: "text-xs text-slate-500", "围绕 root_event 观察因果链与 Top-K 历史召回。" }
            }

            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-3",
                form { class: "flex flex-wrap items-end gap-2", onsubmit: on_submit_root,
                    div { class: "flex flex-col gap-1",
                        label { class: "text-[11px] font-semibold text-slate-600", "root_event_id" }
                        input {
                            class: "w-40 rounded border border-slate-300 px-2 py-1 text-xs",
                            value: "{root_input.read()}",
                            oninput: move |evt| root_input.set(evt.value().to_string()),
                            placeholder: "例如 10000",
                        }
                    }
                    button {
                        class: "rounded bg-slate-900 px-3 py-1.5 text-xs font-semibold text-white hover:bg-slate-800",
                        r#type: "submit",
                        "刷新"
                    }
                    if let Some(up) = upstream {
                        button {
                            class: "rounded border border-slate-300 px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-100",
                            r#type: "button",
                            onclick: {
                                let actions = actions.clone();
                                move |_| actions.set_graph_root(Some(up))
                            },
                            {format!("上游 #{up}")}
                        }
                    }
                    if !downstream.is_empty() {
                        div { class: "flex flex-wrap gap-1",
                            for down in downstream.iter() {
                                button {
                                    key: format!("down-{down}"),
                                    class: "rounded border border-slate-300 px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-100",
                                    r#type: "button",
                                    onclick: {
                                        let actions = actions.clone();
                                        let target = *down;
                                        move |_| actions.set_graph_root(Some(target))
                                    },
                                    {format!("下游 #{down}")}
                                }
                            }
                        }
                    }
                }

                if graph_state.is_loading {
                    p { class: "text-xs text-slate-500", "正在加载图谱数据..." }
                } else if let Some(ref err) = graph_state.error {
                    p { class: "text-xs text-red-500", "加载失败: {err}" }
                } else if let Some(graph) = graph_state.causal.as_ref() {
                    {render_causal_graph(graph)}
                } else {
                    p { class: "text-xs text-slate-500 italic", "暂无图谱数据" }
                }
            }

            div { class: "space-y-2",
                h3 { class: "text-sm font-semibold text-slate-800", "Top-K 历史召回" }
                {recall_section}
            }
        }
    }
}

fn render_recall_list(items: &[RecallResultView]) -> Element {
    if items.is_empty() {
        return rsx! { p { class: "text-xs text-slate-500 italic", "暂无召回结果" } };
    }

    rsx! {
        ul { class: "space-y-2",
            for item in items.iter() {
                li { class: "rounded border border-slate-200 bg-white p-3 text-xs text-slate-600 shadow-sm space-y-1",
                    div { class: "flex items-center justify-between",
                        span { class: "font-semibold text-slate-800", {format!("事件 #{}", item.event_id)} }
                        span { class: "rounded bg-slate-100 px-2 py-0.5 text-[11px] text-slate-600", {format_score(item.score)} }
                    }
                    if let Some(label) = item.label.as_ref() {
                        p { class: "text-[11px] text-slate-500", "{label}" }
                    }
                    if let Some(snippet) = item.snippet.as_ref() {
                        p { class: "text-slate-500", "{snippet}" }
                    }
                    if let Some(reason) = item.reason.as_ref() {
                        p { class: "text-[11px] text-slate-400", {format!("来源: {reason}")} }
                    }
                }
            }
        }
    }
}

fn render_causal_graph(graph: &CausalGraphView) -> Element {
    let mut grouped: BTreeMap<i32, Vec<&CausalGraphNode>> = BTreeMap::new();
    for node in &graph.nodes {
        let depth = node.depth.unwrap_or(0);
        grouped.entry(depth).or_default().push(node);
    }

    rsx! {
        div { class: "space-y-3",
            h3 { class: "text-sm font-semibold text-slate-800", {format!("因果链 root #{:?}", graph.root_event_id)} }
            for (depth, nodes) in grouped.iter() {
                div { class: "flex flex-col gap-2",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", {format!("Depth {depth}")} }
                    div { class: "flex flex-wrap gap-2",
                        for node in nodes.iter() {
                            div { class: "min-w-[180px] flex-1 rounded border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600 shadow-inner",
                                p { class: "font-semibold text-slate-800", {format!("事件 #{:?}", node.event_id)} }
                                if let Some(label) = node.label.as_ref() {
                                    p { class: "text-[11px] text-slate-500", "{label}" }
                                }
                                if let Some(summary) = node.summary.as_ref() {
                                    p { class: "", "{summary}" }
                                }
                                if let Some(score) = node.score {
                                    p { class: "text-[11px] text-slate-400", {format!("Score: {}", format_score(score))} }
                                }
                            }
                        }
                    }
                }
            }
            if !graph.edges.is_empty() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold text-slate-500", "边关系" }
                    ul { class: "space-y-1 text-xs text-slate-500",
                        for edge in graph.edges.iter() {
                            li { {format!("#{} -> #{} ({})", edge.from, edge.to, edge.relation.clone().unwrap_or_else(|| "关联".into()))} }
                        }
                    }
                }
            }
        }
    }
}

fn format_score(score: f32) -> String {
    format!("{:.2}", score)
}
