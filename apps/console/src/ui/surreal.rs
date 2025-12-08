//! SurrealDB 原生功能面板
//!
//! 展示向量搜索、时序聚合、实时订阅等功能

use dioxus::prelude::*;

use crate::hooks::surreal::{use_live_subscription, use_timeseries_aggregate, use_vector_search};
use crate::models::TimeSeriesAggregateResponse;

/// 向量搜索面板组件
#[component]
pub fn VectorSearchPanel() -> Element {
    let searcher = use_vector_search();

    let mut query_text = use_signal(String::new);

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "向量搜索" }
                p { class: "text-xs text-slate-500", "基于语义相似度的向量检索" }
            }
            // 搜索表单
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                div { class: "space-y-3",
                    div {
                        label { class: "block text-xs text-slate-500 mb-1", "搜索查询" }
                        textarea {
                            class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none",
                            rows: "3",
                            placeholder: "输入要搜索的文本...",
                            value: "{query_text}",
                            oninput: move |evt| query_text.set(evt.value().clone())
                        }
                    }
                    p { class: "text-xs text-slate-500", "向量搜索功能需要后端支持" }
                }
            }
            // 状态展示
            if *searcher.searching.read() {
                p { class: "text-xs text-slate-500", "搜索中..." }
            }
            if let Some(ref err) = *searcher.error.read() {
                div { class: "p-3 bg-red-50 rounded-lg border border-red-100",
                    p { class: "text-sm text-red-700", "{err}" }
                }
            }
            if let Some(ref result) = *searcher.result.read() {
                div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                    h3 { class: "text-sm font-semibold text-slate-800 mb-3", "搜索结果" }
                    p { class: "text-xs text-slate-500",
                        {format!("找到 {} 条结果，耗时 {}ms", result.results.len(), result.search_time_ms)}
                    }
                }
            }
        }
    }
}

/// 时序聚合面板组件
#[component]
pub fn TimeSeriesPanel() -> Element {
    let mut metric = use_signal(|| "cycle_duration".to_string());
    let mut aggregation = use_signal(|| "avg".to_string());
    let mut interval = use_signal(|| "1h".to_string());

    let ts_state = use_timeseries_aggregate(
        metric.read().clone(),
        aggregation.read().clone(),
        interval.read().clone(),
    );

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "时序分析" }
                p { class: "text-xs text-slate-500", "时间序列数据的聚合分析" }
            }
            // 查询设置
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                div { class: "grid grid-cols-3 gap-3",
                    div {
                        label { class: "block text-xs text-slate-500 mb-1", "指标" }
                        select {
                            class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg",
                            value: "{metric}",
                            onchange: move |evt| metric.set(evt.value().clone()),
                            option { value: "cycle_duration", "周期时长" }
                            option { value: "token_usage", "Token 使用" }
                            option { value: "decision_confidence", "决策置信度" }
                            option { value: "error_rate", "错误率" }
                        }
                    }
                    div {
                        label { class: "block text-xs text-slate-500 mb-1", "聚合方式" }
                        select {
                            class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg",
                            value: "{aggregation}",
                            onchange: move |evt| aggregation.set(evt.value().clone()),
                            option { value: "avg", "平均值" }
                            option { value: "sum", "总和" }
                            option { value: "min", "最小值" }
                            option { value: "max", "最大值" }
                            option { value: "count", "计数" }
                        }
                    }
                    div {
                        label { class: "block text-xs text-slate-500 mb-1", "时间间隔" }
                        select {
                            class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg",
                            value: "{interval}",
                            onchange: move |evt| interval.set(evt.value().clone()),
                            option { value: "1m", "1 分钟" }
                            option { value: "5m", "5 分钟" }
                            option { value: "15m", "15 分钟" }
                            option { value: "1h", "1 小时" }
                            option { value: "1d", "1 天" }
                        }
                    }
                }
            }
            // 结果展示
            {
                let state = ts_state.read();
                if state.loading {
                    rsx! { p { class: "text-xs text-slate-500", "正在加载时序数据..." } }
                } else if let Some(ref err) = state.error {
                    rsx! { p { class: "text-xs text-red-500", "加载失败: {err}" } }
                } else if let Some(ref data) = state.data {
                    rsx! { {render_timeseries_data(data)} }
                } else {
                    rsx! {
                        div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                            p { class: "text-xs text-slate-500 italic", "暂无时序数据" }
                        }
                    }
                }
            }
        }
    }
}

fn render_timeseries_data(data: &TimeSeriesAggregateResponse) -> Element {
    rsx! {
        div { class: "space-y-4",
            // 汇总信息
            div { class: "rounded-lg border border-slate-200 bg-slate-50 p-4 shadow-sm",
                div { class: "grid grid-cols-5 gap-4 text-center",
                    div {
                        p { class: "text-xl font-bold text-blue-600", {format!("{}", data.buckets.len())} }
                        p { class: "text-xs text-slate-500", "时间桶" }
                    }
                    div {
                        p { class: "text-xl font-bold text-green-600",
                            {format!("{:.2}", data.summary.min)}
                        }
                        p { class: "text-xs text-slate-500", "最小值" }
                    }
                    div {
                        p { class: "text-xl font-bold text-purple-600",
                            {format!("{:.2}", data.summary.max)}
                        }
                        p { class: "text-xs text-slate-500", "最大值" }
                    }
                    div {
                        p { class: "text-xl font-bold text-orange-600",
                            {format!("{:.2}", data.summary.avg)}
                        }
                        p { class: "text-xs text-slate-500", "平均值" }
                    }
                    div {
                        p { class: "text-xl font-bold text-slate-600",
                            {format!("{}", data.summary.count)}
                        }
                        p { class: "text-xs text-slate-500", "数据点" }
                    }
                }
            }
            // 趋势信息（如果有）
            if let Some(ref trend) = data.trend {
                div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                    h3 { class: "text-sm font-semibold text-slate-800 mb-3", "趋势分析" }
                    div { class: "grid grid-cols-4 gap-4 text-center",
                        div {
                            p { class: "text-lg font-bold text-blue-600", "{trend.direction}" }
                            p { class: "text-xs text-slate-500", "方向" }
                        }
                        div {
                            p { class: "text-lg font-bold text-green-600",
                                {format!("{:.3}", trend.slope)}
                            }
                            p { class: "text-xs text-slate-500", "斜率" }
                        }
                        div {
                            p { class: "text-lg font-bold text-purple-600",
                                {format!("{:.2}", trend.r_squared)}
                            }
                            p { class: "text-xs text-slate-500", "R²" }
                        }
                        div {
                            p { class: "text-lg font-bold text-orange-600",
                                {format!("{:.0}%", trend.confidence * 100.0)}
                            }
                            p { class: "text-xs text-slate-500", "置信度" }
                        }
                    }
                }
            }
        }
    }
}

/// 实时订阅面板组件
#[component]
pub fn LiveSubscriptionPanel() -> Element {
    let manager = use_live_subscription();

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "实时订阅" }
                p { class: "text-xs text-slate-500", "订阅数据库表的实时变更" }
            }
            // 状态
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                div { class: "flex items-center gap-2",
                    if *manager.connected.read() {
                        span { class: "w-2 h-2 bg-green-500 rounded-full" }
                        span { class: "text-xs text-green-600", "已连接" }
                    } else {
                        span { class: "w-2 h-2 bg-slate-400 rounded-full" }
                        span { class: "text-xs text-slate-500", "未连接" }
                    }
                }
            }
            // 事件列表
            {
                let events = manager.events.read();
                if events.is_empty() {
                    rsx! {
                        div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                            p { class: "text-xs text-slate-500 italic", "暂无实时事件" }
                        }
                    }
                } else {
                    rsx! {
                        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                            h3 { class: "text-sm font-semibold text-slate-800 mb-3",
                                {format!("实时事件 ({})", events.len())}
                            }
                            div { class: "space-y-2 max-h-64 overflow-y-auto",
                                for (i, event) in events.iter().enumerate() {
                                    div { class: "p-2 bg-slate-50 rounded text-xs font-mono",
                                        key: "{i}",
                                        {event.to_string()}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
