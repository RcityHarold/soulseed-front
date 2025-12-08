//! 元认知分析面板
//!
//! 展示元认知分析结果

use dioxus::prelude::*;

use crate::hooks::metacognition::use_metacognition_analysis;
use crate::models::{AnalysisResultResponse, InsightResponse};

/// 元认知分析面板组件
#[component]
pub fn MetacognitionPanel() -> Element {
    let analysis_state = use_metacognition_analysis();

    let body = {
        let state = analysis_state.read();
        if state.loading {
            rsx! { p { class: "text-xs text-slate-500", "正在加载元认知分析..." } }
        } else if let Some(ref err) = state.error {
            rsx! { p { class: "text-xs text-red-500", "加载失败: {err}" } }
        } else if let Some(ref analysis) = state.analysis {
            rsx! {
                div { class: "space-y-4",
                    {render_analysis_overview(analysis)}
                    {render_insights(&analysis.insights)}
                }
            }
        } else {
            rsx! { p { class: "text-xs text-slate-500 italic", "暂无元认知分析数据" } }
        }
    };

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "元认知分析" }
                p { class: "text-xs text-slate-500", "AI 自我思考过程的深度分析与模式识别" }
            }
            {body}
        }
    }
}

/// 渲染分析概览
fn render_analysis_overview(analysis: &AnalysisResultResponse) -> Element {
    let success_color = if analysis.success { "text-green-600" } else { "text-red-600" };
    let success_text = if analysis.success { "成功" } else { "失败" };

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-slate-50 p-4 shadow-sm",
            h3 { class: "text-sm font-semibold text-slate-800 mb-3", "分析概览" }
            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                div { class: "text-center",
                    p { class: "text-lg font-bold text-blue-600", "{analysis.mode}" }
                    p { class: "text-xs text-slate-500", "分析模式" }
                }
                div { class: "text-center",
                    p { class: format!("text-lg font-bold {}", success_color), "{success_text}" }
                    p { class: "text-xs text-slate-500", "状态" }
                }
                div { class: "text-center",
                    p { class: "text-lg font-bold text-green-600",
                        {format!("{}", analysis.insights.len())}
                    }
                    p { class: "text-xs text-slate-500", "洞见数量" }
                }
                div { class: "text-center",
                    p { class: "text-lg font-bold text-purple-600",
                        {format!("{}ms", analysis.execution_time_ms)}
                    }
                    p { class: "text-xs text-slate-500", "执行时间" }
                }
            }
            // 摘要
            if let Some(ref summary) = analysis.summary {
                div { class: "mt-4 p-3 bg-white rounded border border-slate-100",
                    h4 { class: "text-xs font-medium text-slate-600 mb-1", "摘要" }
                    p { class: "text-sm text-slate-700", "{summary}" }
                }
            }
        }
    }
}

/// 渲染洞见列表
fn render_insights(insights: &[InsightResponse]) -> Element {
    if insights.is_empty() {
        return rsx! {
            div { class: "rounded-lg border border-slate-200 bg-white p-4",
                p { class: "text-xs text-slate-500 italic", "暂无洞见数据" }
            }
        };
    }

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            h3 { class: "text-sm font-semibold text-slate-800 mb-3", "洞见列表" }
            div { class: "space-y-3",
                for insight in insights.iter() {
                    {render_insight(insight)}
                }
            }
        }
    }
}

fn render_insight(insight: &InsightResponse) -> Element {
    let importance_color = if insight.importance >= 0.7 {
        "bg-red-100 text-red-700"
    } else if insight.importance >= 0.4 {
        "bg-yellow-100 text-yellow-700"
    } else {
        "bg-slate-100 text-slate-700"
    };

    rsx! {
        div { class: "p-3 bg-slate-50 rounded-lg border border-slate-100",
            div { class: "flex items-start justify-between mb-2",
                div {
                    h4 { class: "text-sm font-medium text-slate-800", "{insight.title}" }
                    span { class: "text-xs text-slate-500", "类型: {insight.insight_type}" }
                }
                div { class: "flex gap-2",
                    span { class: format!("text-xs px-2 py-0.5 rounded {}", importance_color),
                        {format!("重要性: {:.0}%", insight.importance * 100.0)}
                    }
                    span { class: "text-xs px-2 py-0.5 rounded bg-blue-100 text-blue-700",
                        {format!("置信度: {:.0}%", insight.confidence * 100.0)}
                    }
                }
            }
            p { class: "text-xs text-slate-600 mb-2", "{insight.description}" }
            // 建议操作
            if !insight.suggested_actions.is_empty() {
                div { class: "mt-2",
                    h5 { class: "text-xs font-medium text-slate-500 mb-1", "建议操作" }
                    ul { class: "space-y-1",
                        for action in insight.suggested_actions.iter() {
                            li { class: "text-xs text-slate-600 flex items-start gap-1",
                                span { class: "text-green-500", "→" }
                                span { "{action}" }
                            }
                        }
                    }
                }
            }
            // 相关实体
            if !insight.related_entities.is_empty() {
                div { class: "mt-2 flex flex-wrap gap-1",
                    for entity in insight.related_entities.iter() {
                        span { class: "text-xs px-2 py-0.5 bg-slate-200 text-slate-600 rounded",
                            "{entity}"
                        }
                    }
                }
            }
        }
    }
}
