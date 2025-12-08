//! 图谱增强面板
//!
//! 展示图谱节点详情、边关系等增强功能

use dioxus::prelude::*;

use crate::hooks::version_chain::use_graph_node;
use crate::models::{GraphEdgeRef, GraphNodeDetail};

/// 图谱增强面板组件
#[component]
pub fn GraphEnhancedPanel() -> Element {
    let mut node_id = use_signal(|| String::new());

    let node_state = use_graph_node(node_id.read().clone());

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "图谱增强" }
                p { class: "text-xs text-slate-500", "查看图谱节点详情与关系" }
            }
            // 搜索面板
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                div { class: "flex gap-2",
                    input {
                        r#type: "text",
                        class: "flex-1 px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                        placeholder: "输入节点 ID...",
                        value: "{node_id}",
                        oninput: move |evt| node_id.set(evt.value().clone())
                    }
                }
            }
            // 节点详情
            {
                let state = node_state.read();
                if state.loading {
                    rsx! { p { class: "text-xs text-slate-500", "正在加载节点详情..." } }
                } else if let Some(ref err) = state.error {
                    rsx! { p { class: "text-xs text-red-500", "加载失败: {err}" } }
                } else if let Some(ref node) = state.node {
                    rsx! {
                        div { class: "space-y-4",
                            {render_node_detail(node)}
                        }
                    }
                } else {
                    rsx! {
                        div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                            p { class: "text-xs text-slate-500 italic", "输入节点 ID 以查看详情" }
                        }
                    }
                }
            }
        }
    }
}

/// 渲染节点详情
fn render_node_detail(node: &GraphNodeDetail) -> Element {
    let type_color = match node.node_type.as_str() {
        "dialogue" => "bg-blue-100 text-blue-700 border-blue-200",
        "awareness" => "bg-purple-100 text-purple-700 border-purple-200",
        "decision" => "bg-green-100 text-green-700 border-green-200",
        "context" => "bg-orange-100 text-orange-700 border-orange-200",
        _ => "bg-slate-100 text-slate-700 border-slate-200",
    };

    let in_count = node.incoming_edges.len();
    let out_count = node.outgoing_edges.len();

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            div { class: "flex items-start justify-between mb-4",
                div {
                    h3 { class: "text-sm font-semibold text-slate-800 mb-1", "节点详情" }
                    span { class: "text-xs font-mono text-slate-500", "{node.node_id}" }
                }
                span { class: format!("text-xs px-3 py-1 rounded-full border {}", type_color),
                    "{node.node_type}"
                }
            }
            // 基本信息
            div { class: "grid grid-cols-2 md:grid-cols-3 gap-4 mb-4",
                div { class: "text-center p-3 bg-slate-50 rounded",
                    p { class: "text-lg font-bold text-green-600", "{in_count}" }
                    p { class: "text-xs text-slate-500", "入边数" }
                }
                div { class: "text-center p-3 bg-slate-50 rounded",
                    p { class: "text-lg font-bold text-purple-600", "{out_count}" }
                    p { class: "text-xs text-slate-500", "出边数" }
                }
                div { class: "text-center p-3 bg-slate-50 rounded",
                    p { class: "text-lg font-bold text-blue-600", {format!("{}", node.created_at_ms)} }
                    p { class: "text-xs text-slate-500", "创建时间" }
                }
            }
            // 属性
            if !node.properties.is_empty() {
                div { class: "border-t border-slate-100 pt-4",
                    h4 { class: "text-xs font-medium text-slate-600 mb-2", "节点属性" }
                    div { class: "grid grid-cols-2 gap-2",
                        for (key, value) in node.properties.iter() {
                            div { class: "p-2 bg-slate-50 rounded",
                                p { class: "text-xs text-slate-500", "{key}" }
                                p { class: "text-sm text-slate-700 font-mono break-all",
                                    "{value}"
                                }
                            }
                        }
                    }
                }
            }
            // 入边
            if !node.incoming_edges.is_empty() {
                div { class: "border-t border-slate-100 pt-4 mt-4",
                    h4 { class: "text-xs font-medium text-slate-600 mb-2", {format!("入边 ({})", in_count)} }
                    div { class: "space-y-1",
                        for edge in node.incoming_edges.iter().take(10) {
                            {render_edge_ref(edge, "incoming")}
                        }
                    }
                }
            }
            // 出边
            if !node.outgoing_edges.is_empty() {
                div { class: "border-t border-slate-100 pt-4 mt-4",
                    h4 { class: "text-xs font-medium text-slate-600 mb-2", {format!("出边 ({})", out_count)} }
                    div { class: "space-y-1",
                        for edge in node.outgoing_edges.iter().take(10) {
                            {render_edge_ref(edge, "outgoing")}
                        }
                    }
                }
            }
        }
    }
}

fn render_edge_ref(edge: &GraphEdgeRef, direction: &str) -> Element {
    let arrow = if direction == "incoming" { "←" } else { "→" };
    let weight_str = edge.weight.map(|w| format!("{:.2}", w)).unwrap_or_default();

    rsx! {
        div { class: "flex items-center gap-2 p-2 bg-slate-50 rounded text-xs",
            span { class: "text-slate-400", "{arrow}" }
            span { class: "font-mono text-slate-600", "{edge.other_node_id}" }
            span { class: "px-2 py-0.5 bg-slate-200 text-slate-500 rounded", "{edge.edge_type}" }
            if !weight_str.is_empty() {
                span { class: "text-slate-400", "权重: {weight_str}" }
            }
        }
    }
}

/// 图谱边浏览器组件 - 简化版
#[component]
pub fn GraphEdgeBrowser() -> Element {
    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "边浏览器" }
                p { class: "text-xs text-slate-500", "按条件筛选图谱边" }
            }
            div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                p { class: "text-xs text-slate-500 italic", "边浏览功能开发中..." }
            }
        }
    }
}
