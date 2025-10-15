use crate::hooks::context::use_context_bundle;
use crate::models::{
    AceExplainSection, BundleSegment, ContextBundleView, DfrExplainSection, ExplainIndices,
    ExplainSection,
};
use crate::state::use_app_state;
use dioxus::prelude::*;

#[component]
pub fn ContextPanel() -> Element {
    use_context_bundle();

    let context_state = use_app_state().read().context.clone();

    let body = if context_state.is_loading {
        rsx! { p { class: "text-xs text-slate-500", "正在加载上下文包..." } }
    } else if let Some(ref err) = context_state.error {
        rsx! { p { class: "text-xs text-red-500", "上下文加载失败: {err}" } }
    } else if let Some(ref bundle) = context_state.bundle {
        rsx! {
            div { class: "space-y-4",
                {render_anchor(bundle)}
                {render_segments(bundle)}
                if let Some(ref explain_indices) = context_state.explain_indices {
                    {render_explain_indices(explain_indices)}
                }
            }
        }
    } else {
        rsx! { p { class: "text-xs text-slate-500 italic", "暂无上下文数据" } }
    };

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "上下文与 Explain" }
                p { class: "text-xs text-slate-500", "展示最新 ContextBundle、预算与 Explain 指纹" }
            }
            {body}
        }
    }
}

fn render_anchor(bundle: &ContextBundleView) -> Element {
    let anchor = &bundle.anchor;
    let tenant_label = anchor.tenant_id.clone();
    let session_label = anchor
        .session_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "无".into());
    let envelope_label = anchor.envelope_id.to_string();
    let config_info = format!(
        "Schema v{} · Config {}@{}",
        anchor.schema_v, anchor.config_snapshot_hash, anchor.config_snapshot_version
    );
    let scenario_label = anchor
        .scenario
        .as_ref()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "未标注场景".into());

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            h3 { class: "text-sm font-semibold text-slate-800", "Anchor" }
            ul { class: "mt-2 space-y-1 text-xs text-slate-600",
                li { "Tenant: {tenant_label}" }
                li { "Session: {session_label}" }
                li { "Envelope: {envelope_label}" }
                li { "{config_info}" }
                li { "Scenario: {scenario_label}" }
            }
        }
    }
}

fn render_segments(bundle: &ContextBundleView) -> Element {
    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            h3 { class: "text-sm font-semibold text-slate-800", "Segments" }
            if bundle.segments.is_empty() {
                p { class: "text-xs text-slate-500", "暂无分段" }
            } else {
                for segment in bundle.segments.iter() {
                    {render_segment(segment)}
                }
            }
            if let Some(budget) = bundle.budget.as_ref() {
                div { class: "mt-3 flex flex-wrap gap-2 text-[11px]",
                    span { class: "rounded bg-emerald-100 px-2 py-0.5 text-emerald-700", "目标 Token {budget.target_tokens}" }
                    span { class: "rounded bg-emerald-50 px-2 py-0.5 text-emerald-700", "预计使用 {budget.projected_tokens}" }
                }
            }
        }
    }
}

fn render_segment(segment: &BundleSegment) -> Element {
    rsx! {
        div { class: "mt-3 rounded border border-slate-100 bg-slate-50 p-3",
            h4 { class: "text-xs font-semibold text-slate-700", "分区 {segment.partition}" }
            if segment.items.is_empty() {
                p { class: "text-xs text-slate-500", "无条目" }
            } else {
                ul { class: "mt-2 space-y-1 text-xs text-slate-600",
                    for item in segment.items.iter() {
                        li {
                            span { class: "font-medium", "{item.ci_id}" }
                            span { class: "ml-2", "Tokens: {item.tokens}" }
                            if let Some(level) = item.summary_level.as_ref() {
                                span { class: "ml-2", "汇总层级: {level}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_explain_indices(indices: &ExplainIndices) -> Element {
    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            h3 { class: "text-sm font-semibold text-slate-800", "Explain 指纹" }
            div { class: "mt-2 grid gap-3 md:grid-cols-2",
                {render_explain_section("Graph", &indices.graph)}
                {render_explain_section("Context", &indices.context)}
                {render_dfr_section(&indices.dfr)}
                {render_ace_section(&indices.ace)}
            }
        }
    }
}

fn render_explain_section(title: &str, section: &ExplainSection) -> Element {
    let indices_text = if section.indices_used.is_empty() {
        "-".to_string()
    } else {
        section.indices_used.join(", ")
    };
    let degradation = section
        .degradation_reason
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("无");
    let query_hash = section
        .query_hash
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("-");

    rsx! {
        div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-600",
            h4 { class: "text-xs font-semibold text-slate-700", "{title}" }
            p { "Indices: {indices_text}" }
            p { "Query Hash: {query_hash}" }
            p { "Degradation: {degradation}" }
        }
    }
}

fn render_dfr_section(section: &DfrExplainSection) -> Element {
    let router = section
        .router_digest
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("-");
    let degradation = section
        .degradation_reason
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("无");

    rsx! {
        div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-600",
            h4 { class: "text-xs font-semibold text-slate-700", "DFR" }
            p { "Router Digest: {router}" }
            p { "Degradation: {degradation}" }
        }
    }
}

fn render_ace_section(section: &AceExplainSection) -> Element {
    let sync_point = section
        .sync_point
        .as_ref()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "无".to_string());
    let degradation = section
        .degradation_reason
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("无");

    rsx! {
        div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-600",
            h4 { class: "text-xs font-semibold text-slate-700", "ACE" }
            p { "SyncPoint: {sync_point}" }
            p { "Degradation: {degradation}" }
        }
    }
}
