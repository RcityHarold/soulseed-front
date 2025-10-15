use crate::models::{AceExplainSection, DfrExplainSection, ExplainIndices, ExplainSection};
use crate::state::{use_app_actions, use_app_state, AppActions};
use dioxus::prelude::*;

#[component]
pub fn ExplainDiagnosticPanel() -> Element {
    let actions = use_app_actions();
    let app_state = use_app_state();
    let context_state = app_state.read().context.clone();
    let body = if context_state.is_loading {
        rsx! { p { class: "text-xs text-slate-500", "正在载入 Explain 指纹..." } }
    } else if let Some(ref err) = context_state.error {
        rsx! { p { class: "text-xs text-red-500", "Explain 数据获取失败: {err}" } }
    } else if let Some(ref indices) = context_state.explain_indices {
        let summary = summarize_indices(indices);
        rsx! {
            div { class: "space-y-4",
                {render_summary(&summary)}
                div { class: "grid gap-3 xl:grid-cols-2",
                    {render_section(actions.clone(), "Graph", &indices.graph)}
                    {render_section(actions.clone(), "Context", &indices.context)}
                }
                div { class: "grid gap-3 md:grid-cols-2",
                    {render_dfr_card(&indices.dfr)}
                    {render_ace_card(&indices.ace)}
                }
            }
        }
    } else {
        rsx! { p { class: "text-xs text-slate-500 italic", "暂无 Explain 数据" } }
    };

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "Explain & SLO 诊断" }
                p { class: "text-xs text-slate-500", "检视 Graph/Context/DFR/ACE 指纹，聚焦索引命中与降级原因" }
            }
            {body}
        }
    }
}

#[derive(Default)]
struct Summary {
    missing: Vec<&'static str>,
    degraded: Vec<(&'static str, String)>,
}

fn summarize_indices(indices: &ExplainIndices) -> Summary {
    let mut summary = Summary::default();

    if indices.graph.indices_used.is_empty() {
        summary.missing.push("Graph");
    }
    if indices.context.indices_used.is_empty() {
        summary.missing.push("Context");
    }

    if let Some(reason) = indices.graph.degradation_reason.as_ref() {
        summary.degraded.push(("Graph", format_reason(reason)));
    }
    if let Some(reason) = indices.context.degradation_reason.as_ref() {
        summary.degraded.push(("Context", format_reason(reason)));
    }
    if let Some(reason) = indices.dfr.degradation_reason.as_ref() {
        summary.degraded.push(("DFR", format_reason(reason)));
    }
    if let Some(reason) = indices.ace.degradation_reason.as_ref() {
        summary.degraded.push(("ACE", format_reason(reason)));
    }

    summary
}

fn render_summary(summary: &Summary) -> Element {
    let missing_count = summary.missing.len();
    let degraded_count = summary.degraded.len();

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm text-xs text-slate-600 space-y-2",
            div { class: "flex flex-wrap items-center gap-3",
                span { class: "font-semibold text-slate-800", "SLO 总览" }
                span { class: "rounded bg-amber-100 px-2 py-0.5 text-amber-800", {format!("未命中索引 {missing_count}")} }
                span { class: "rounded bg-red-100 px-2 py-0.5 text-red-700", {format!("降级 {degraded_count}")} }
            }
            if !summary.missing.is_empty() {
                div { class: "flex flex-wrap gap-1",
                    for facet in summary.missing.iter() {
                        span { class: "rounded bg-amber-100 px-2 py-0.5 text-[11px] text-amber-800", {format!("{facet} 未命中")} }
                    }
                }
            }
            if !summary.degraded.is_empty() {
                div { class: "space-y-1",
                    for (facet, reason) in summary.degraded.iter() {
                        p { class: "flex items-center gap-2",
                            span { class: "rounded bg-red-100 px-2 py-0.5 text-[11px] text-red-700", "{facet}" }
                            span { class: "text-slate-500", "{reason}" }
                        }
                    }
                }
            }
            if summary.missing.is_empty() && summary.degraded.is_empty() {
                p { class: "text-slate-500", "所有模块命中索引，未检测到降级。" }
            }
        }
    }
}

fn render_section(actions: AppActions, title: &str, section: &ExplainSection) -> Element {
    let indices = if section.indices_used.is_empty() {
        rsx! { span { class: "rounded bg-amber-100 px-2 py-0.5 text-[11px] text-amber-800", "未命中" } }
    } else {
        rsx! {
            div { class: "flex flex-wrap gap-1",
                for idx in section.indices_used.iter() {
                    span { class: "rounded bg-slate-200 px-2 py-0.5 text-[11px] text-slate-700", "{idx}" }
                }
            }
        }
    };

    let degradation = section
        .degradation_reason
        .as_ref()
        .map(|reason| format_reason(reason))
        .unwrap_or_else(|| "正常".into());
    let status_class = if section.degradation_reason.is_some() {
        "rounded bg-red-100 px-2 py-0.5 text-[11px] text-red-700"
    } else {
        "rounded bg-emerald-100 px-2 py-0.5 text-[11px] text-emerald-700"
    };

    let query_hash_row = section.query_hash.as_ref().map(|hash| {
        let actions = actions.clone();
        let label = format!("{title} Query Hash");
        let hash_value = hash.clone();
        rsx! {
            div { class: "flex items-center justify-between gap-2",
                span { class: "font-mono text-[11px] text-slate-500 break-all", "{hash}" }
                button {
                    class: "rounded bg-slate-900 px-2 py-1 text-[11px] font-semibold text-white hover:bg-slate-800",
                    onclick: move |_| copy_to_clipboard(actions.clone(), label.clone(), hash_value.clone()),
                    "复制"
                }
            }
        }
    });

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-2 text-xs text-slate-600",
            header { class: "flex items-center justify-between",
                h3 { class: "text-sm font-semibold text-slate-800", "{title}" }
                span { class: status_class, "{degradation}" }
            }
            div { class: "space-y-1",
                strong { class: "text-[11px] text-slate-500", "Indices" }
                {indices}
            }
            if let Some(row) = query_hash_row {
                div { class: "space-y-1",
                    strong { class: "text-[11px] text-slate-500", "Query Hash" }
                    {row}
                }
            }
        }
    }
}

fn render_dfr_card(section: &DfrExplainSection) -> Element {
    let degradation = section
        .degradation_reason
        .as_ref()
        .map(|reason| format_reason(reason))
        .unwrap_or_else(|| "正常".into());
    let status_class = if section.degradation_reason.is_some() {
        "rounded bg-red-100 px-2 py-0.5 text-[11px] text-red-700"
    } else {
        "rounded bg-emerald-100 px-2 py-0.5 text-[11px] text-emerald-700"
    };

    let router = section
        .router_digest
        .as_ref()
        .map(|digest| digest.as_str())
        .unwrap_or("-");

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-2 text-xs text-slate-600",
            header { class: "flex items-center justify-between",
                h3 { class: "text-sm font-semibold text-slate-800", "DFR" }
                span { class: status_class, "{degradation}" }
            }
            p { "Router Digest: {router}" }
        }
    }
}

fn render_ace_card(section: &AceExplainSection) -> Element {
    let degradation = section
        .degradation_reason
        .as_ref()
        .map(|reason| format_reason(reason))
        .unwrap_or_else(|| "正常".into());
    let status_class = if section.degradation_reason.is_some() {
        "rounded bg-red-100 px-2 py-0.5 text-[11px] text-red-700"
    } else {
        "rounded bg-emerald-100 px-2 py-0.5 text-[11px] text-emerald-700"
    };

    let sync_point = section
        .sync_point
        .as_ref()
        .map(|point| format!("{:?}", point))
        .unwrap_or_else(|| "未设置".into());

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-2 text-xs text-slate-600",
            header { class: "flex items-center justify-between",
                h3 { class: "text-sm font-semibold text-slate-800", "ACE" }
                span { class: status_class, "{degradation}" }
            }
            p { "SyncPoint: {sync_point}" }
        }
    }
}

fn format_reason(reason: &str) -> String {
    reason
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn copy_to_clipboard(actions: AppActions, label: String, value: String) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen_futures::{spawn_local, JsFuture};

        spawn_local(async move {
            let result = async {
                let window = web_sys::window().ok_or(())?;
                let clipboard = window.navigator().clipboard();
                let promise = clipboard.write_text(&value);
                JsFuture::from(promise)
                    .await
                    .map(|_| ())
                    .map_err(|_| ())
            }
            .await;

            match result {
                Ok(_) => actions.set_operation_success(format!("{label} 已复制")),
                Err(_) => actions.set_operation_error(format!("{label} 复制失败")),
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = value; // silence unused in native builds
        actions.set_operation_success(format!("{label} 已复制（模拟）"));
    }
}
