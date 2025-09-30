use std::collections::BTreeMap;

use crate::hooks::graph::use_graph_insights;
use crate::models::{CausalGraphNode, CausalGraphView, RecallResultView};
use crate::state::{use_app_actions, use_app_state};
use dioxus::prelude::*;

pub fn GraphPanel(cx: Scope) -> Element {
    use_graph_insights(cx);

    let actions = use_app_actions(cx);
    let app_state = use_app_state(cx);
    let snapshot = app_state.read();
    let graph_state = snapshot.graph.clone();
    let timeline_events = snapshot.timeline.events.clone();
    drop(snapshot);

    let default_root = graph_state.query.root_event_id.or_else(|| {
        timeline_events
            .first()
            .map(|event| event.event_id.into_inner())
    });

    let root_input = use_signal(cx, || {
        default_root.map(|id| id.to_string()).unwrap_or_default()
    });

    {
        let actions = actions.clone();
        use_effect(
            cx,
            (graph_state.query.root_event_id, default_root),
            move |(current, fallback)| {
                let actions = actions.clone();
                async move {
                    if current.is_none() {
                        if let Some(root) = fallback {
                            actions.set_graph_root(Some(root));
                        }
                    }
                }
            },
        );
    }

    {
        let root_input = root_input.clone();
        use_effect(cx, graph_state.query.root_event_id, move |current| {
            let root_input = root_input.clone();
            async move {
                match current {
                    Some(id) => root_input.set(id.to_string()),
                    None => root_input.set(String::new()),
                }
            }
        });
    }

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
        .and_then(|graph| {
            graph
                .edges
                .iter()
                .find(|edge| edge.to == graph.root_event_id)
        })
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
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let render_recall = if graph_state.recall.is_empty() {
        rsx! { p { class: "text-xs text-slate-500 italic", "暂无召回结果" } }
    } else {
        rsx! {
            ul { class: "space-y-2",
                for item in graph_state.recall.iter() {
                    li { class: "rounded border border-slate-200 bg-white p-3 text-xs text-slate-600 shadow-sm space-y-1",
                        div { class: "flex items-center justify-between",
                            span { class: "font-semibold text-slate-800", format!("事件 #{}", item.event_id) }
                            span { class: "rounded bg-slate-100 px-2 py-0.5 text-[11px] text-slate-600", format_score(item.score) }
                        }
                        if let Some(label) = item.label.as_ref() {
                            p { class: "text-[11px] text-slate-500", label }
                        }
                        if let Some(snippet) = item.snippet.as_ref() {
                            p { class: "text-slate-500", snippet }
                        }
                        if let Some(reason) = item.reason.as_ref() {
                            p { class: "text-[11px] text-slate-400", format!("来源: {reason}") }
                        }
                    }
                }
            }
        }
    };

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
                        let actions = actions.clone();
                        button {
                            class: "rounded border border-slate-300 px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-100",
                            r#type: "button",
                            onclick: move |_| actions.set_graph_root(Some(up)),
                            "上游 #{up}"
                        }
                    }
                    if !downstream.is_empty() {
                        div { class: "flex flex-wrap gap-1",
                            for down in downstream.iter() {
                                let actions = actions.clone();
                                button {
                                    key: format!("down-{down}"),
                                    class: "rounded border border-slate-300 px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-100",
                                    r#type: "button",
                                    onclick: move |_| actions.set_graph_root(Some(*down)),
                                    "下游 #{down}"
                                }
                            }
                        }
                    }
                }

                if graph_state.is_loading {
                    p { class: "text-xs text-slate-500", "正在加载图谱数据..." }
                } else if let Some(ref err) = graph_state.error {
                    p { class: "text-xs text-red-500", "加载失败: {err}" }
                } else if graph_state.causal.is_none() {
                    p { class: "text-xs text-slate-500 italic", "暂无图谱数据" }
                } else {
                    render_causal_graph(cx, graph_state.causal.as_ref().expect("causal graph"))
                }
            }

            div { class: "space-y-2",
                h3 { class: "text-sm font-semibold text-slate-800", "Top-K 历史召回" }
                render_recall
            }
        }
    }
}

fn render_causal_graph(cx: Scope, graph: &CausalGraphView) -> LazyNodes {
    let mut grouped: BTreeMap<i32, Vec<&CausalGraphNode>> = BTreeMap::new();
    for node in &graph.nodes {
        let depth = node.depth.unwrap_or(0);
        grouped.entry(depth).or_default().push(node);
    }

    cx.render(rsx! {
        div { class: "space-y-3",
            h3 { class: "text-sm font-semibold text-slate-800", format!("因果链 root #{:?}", graph.root_event_id) }
            for (depth, nodes) in grouped.iter() {
                div { class: "flex flex-col gap-2",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", format!("Depth {depth}") }
                    div { class: "flex flex-wrap gap-2",
                        for node in nodes.iter() {
                            div { class: "min-w-[180px] flex-1 rounded border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600 shadow-inner",
                                p { class: "font-semibold text-slate-800", format!("事件 #{:?}", node.event_id) }
                                if let Some(label) = node.label.as_ref() {
                                    p { class: "text-[11px] text-slate-500", label }
                                }
                                if let Some(summary) = node.summary.as_ref() {
                                    p { class: "", summary }
                                }
                                if let Some(score) = node.score {
                                    p { class: "text-[11px] text-slate-400", format!("Score: {}", format_score(score)) }
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
                            li { format!("#{} -> #{} ({})", edge.from, edge.to, edge.relation.clone().unwrap_or_else(|| "关联".into())) }
                        }
                    }
                }
            }
        }
    })
}

fn format_score(score: f32) -> String {
    format!("{:.2}", score)
}
