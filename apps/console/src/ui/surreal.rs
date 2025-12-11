//! SurrealDB 原生功能面板
//!
//! 展示向量搜索、时序聚合、实时订阅等功能

use dioxus::prelude::*;

use crate::hooks::surreal::{
    use_content_indexer, use_live_subscription, use_timeseries_aggregate, use_vector_search,
};
use crate::models::TimeSeriesAggregateResponse;

/// 向量搜索面板组件
#[component]
pub fn VectorSearchPanel() -> Element {
    let mut searcher = use_vector_search();

    let mut query_text = use_signal(String::new);
    let mut top_k = use_signal(|| 10u32);

    let is_searching = *searcher.searching.read();

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
                            disabled: is_searching,
                            oninput: move |evt| query_text.set(evt.value().clone())
                        }
                    }
                    div { class: "flex items-end gap-4",
                        div { class: "flex-1",
                            label { class: "block text-xs text-slate-500 mb-1", "返回数量 (Top K)" }
                            input {
                                r#type: "number",
                                class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                min: "1",
                                max: "100",
                                value: "{top_k}",
                                disabled: is_searching,
                                oninput: move |evt| {
                                    if let Ok(v) = evt.value().parse::<u32>() {
                                        top_k.set(v.clamp(1, 100));
                                    }
                                }
                            }
                        }
                        button {
                            class: "px-6 py-2 text-sm font-medium rounded-lg transition-colors shadow",
                            style: if is_searching {
                                "background-color: #9ca3af; color: white; cursor: not-allowed;"
                            } else {
                                "background-color: #4f46e5; color: white; cursor: pointer;"
                            },
                            disabled: is_searching,
                            onclick: move |_| {
                                let query = query_text.read().clone();
                                let k = *top_k.read();
                                spawn(async move {
                                    searcher.search(query, None, Some(k)).await;
                                });
                            },
                            if is_searching {
                                "搜索中..."
                            } else {
                                "搜索"
                            }
                        }
                    }
                }
            }
            // 状态展示
            if *searcher.searching.read() {
                div { class: "flex items-center gap-2 text-blue-600",
                    span { class: "w-4 h-4 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" }
                    span { class: "text-xs", "正在搜索..." }
                }
            }
            if let Some(ref err) = *searcher.error.read() {
                div { class: "p-3 bg-red-50 rounded-lg border border-red-100",
                    p { class: "text-sm text-red-700", "{err}" }
                }
            }
            if let Some(ref result) = *searcher.result.read() {
                div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                    h3 { class: "text-sm font-semibold text-slate-800 mb-3", "搜索结果" }
                    p { class: "text-xs text-slate-500 mb-3",
                        {format!("找到 {} 条结果，耗时 {}ms", result.results.len(), result.search_time_ms)}
                    }
                    // 显示搜索结果列表
                    if result.results.is_empty() {
                        div { class: "p-4 bg-slate-50 rounded-lg text-center",
                            p { class: "text-xs text-slate-500 italic", "没有找到匹配的结果" }
                        }
                    } else {
                        div { class: "space-y-2 max-h-96 overflow-y-auto",
                            for (i, item) in result.results.iter().enumerate() {
                                div {
                                    key: "{i}",
                                    class: "p-3 bg-slate-50 rounded-lg border border-slate-100",
                                    div { class: "flex items-center justify-between mb-2",
                                        span { class: "text-xs font-medium text-slate-700",
                                            "#{i + 1} - {item.chunk_id}"
                                        }
                                        span { class: "text-xs text-blue-600 font-mono",
                                            {format!("相似度: {:.4}", item.score)}
                                        }
                                    }
                                    if let Some(ref content) = item.content {
                                        p { class: "text-sm text-slate-800 whitespace-pre-wrap break-words",
                                            {content.clone()}
                                        }
                                    }
                                    if !item.metadata.is_empty() {
                                        div { class: "mt-2 pt-2 border-t border-slate-200",
                                            p { class: "text-xs text-slate-500 font-mono overflow-x-auto",
                                                {serde_json::to_string(&item.metadata).unwrap_or_default()}
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

/// 内容索引面板组件
#[component]
pub fn ContentIndexPanel() -> Element {
    let mut indexer = use_content_indexer();

    let mut content_text = use_signal(String::new);
    let mut source_type = use_signal(|| "manual".to_string());
    // 使用 web_sys 获取时间戳，兼容 WASM 环境
    let mut source_id = use_signal(|| {
        let timestamp = web_sys::js_sys::Date::now() as u64;
        format!("doc_{}", timestamp)
    });

    let is_indexing = *indexer.indexing.read();

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "内容索引" }
                p { class: "text-xs text-slate-500", "添加文本内容到向量索引，用于语义搜索" }
            }
            // 索引表单
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                div { class: "space-y-3",
                    div {
                        label { class: "block text-xs text-slate-500 mb-1", "内容文本" }
                        textarea {
                            class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none",
                            rows: "4",
                            placeholder: "输入要索引的内容（至少10个字符）...",
                            value: "{content_text}",
                            disabled: is_indexing,
                            oninput: move |evt| content_text.set(evt.value().clone())
                        }
                    }
                    div { class: "grid grid-cols-2 gap-4",
                        div {
                            label { class: "block text-xs text-slate-500 mb-1", "来源类型" }
                            select {
                                class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                disabled: is_indexing,
                                value: "{source_type}",
                                onchange: move |evt| source_type.set(evt.value().clone()),
                                option { value: "manual", "手动输入" }
                                option { value: "document", "文档" }
                                option { value: "note", "笔记" }
                                option { value: "faq", "FAQ" }
                                option { value: "knowledge", "知识库" }
                            }
                        }
                        div {
                            label { class: "block text-xs text-slate-500 mb-1", "来源 ID" }
                            input {
                                r#type: "text",
                                class: "w-full px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                placeholder: "唯一标识符",
                                value: "{source_id}",
                                disabled: is_indexing,
                                oninput: move |evt| source_id.set(evt.value().clone())
                            }
                        }
                    }
                    div { class: "flex justify-end",
                        button {
                            class: "px-4 py-2 text-sm font-medium rounded-lg transition-colors shadow",
                            style: if is_indexing {
                                "background-color: #9ca3af; color: white; cursor: not-allowed;"
                            } else {
                                "background-color: #16a34a; color: white; cursor: pointer;"
                            },
                            disabled: is_indexing,
                            onclick: move |_| {
                                let content = content_text.read().clone();
                                let stype = source_type.read().clone();
                                let sid = source_id.read().clone();
                                spawn(async move {
                                    indexer.index_content(content, stype, sid, None).await;
                                });
                            },
                            if is_indexing {
                                "索引中..."
                            } else {
                                "添加到索引"
                            }
                        }
                    }
                }
            }
            // 状态展示
            if *indexer.indexing.read() {
                div { class: "flex items-center gap-2 text-green-600",
                    span { class: "w-4 h-4 border-2 border-green-600 border-t-transparent rounded-full animate-spin" }
                    span { class: "text-xs", "正在生成嵌入向量并索引..." }
                }
            }
            if let Some(ref err) = *indexer.error.read() {
                div { class: "p-3 bg-red-50 rounded-lg border border-red-100",
                    p { class: "text-sm text-red-700", "{err}" }
                }
            }
            if let Some(ref result) = *indexer.last_result.read() {
                div { class: "rounded-lg border border-green-200 bg-green-50 p-4 shadow-sm",
                    h3 { class: "text-sm font-semibold text-green-800 mb-2", "索引成功" }
                    div { class: "space-y-1 text-xs text-green-700",
                        p {
                            span { class: "font-medium", "Chunk ID: " }
                            span { class: "font-mono", "{result.chunk_id}" }
                        }
                        p {
                            span { class: "font-medium", "内容长度: " }
                            span { "{result.content_length} 字符" }
                        }
                        p {
                            span { class: "font-medium", "嵌入维度: " }
                            span { "{result.embedding_dim}" }
                        }
                    }
                }
            }
        }
    }
}
