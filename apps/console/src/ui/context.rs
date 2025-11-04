use crate::hooks::context::use_context_bundle;
use crate::models::{
    AceExplainSection, BundleSegment, ContextBundleView, DfrExplainSection, ExplainIndices,
    ExplainSection, ManifestDigestRecord,
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
        let history = context_state.manifest_history.clone();
        let active_digest = context_state.active_manifest_digest.clone();
        let active_record = active_digest.as_deref().and_then(|digest| {
            history
                .iter()
                .find(|record| record.manifest_digest == digest)
        });
        rsx! {
            div { class: "space-y-4",
                if let Some(overview) = render_manifest_overview(bundle, active_record) {
                    {overview}
                }
                {render_anchor(bundle)}
                if let Some(explain_card) = render_manifest_explain(bundle) {
                    {explain_card}
                }
                {render_segments(bundle)}
                if let Some(ref explain_indices) = context_state.explain_indices {
                    {render_explain_indices(explain_indices)}
                }
                if let Some(history_view) = render_manifest_history(&history, active_digest.as_deref()) {
                    {history_view}
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
fn render_manifest_overview(
    bundle: &ContextBundleView,
    record: Option<&ManifestDigestRecord>,
) -> Option<Element> {
    let digest = bundle.manifest_digest.as_deref().unwrap_or("-").to_string();
    let seen_at = record
        .and_then(|entry| entry.seen_at.clone())
        .unwrap_or_else(|| "未知时间".into());
    let cycles = record
        .map(|entry| entry.cycle_ids.clone())
        .unwrap_or_default();
    let cycle_label = if cycles.is_empty() {
        "无关联周期".to_string()
    } else {
        cycles.join(", ")
    };
    let manifest_version = bundle
        .version
        .map(|version| format!("v{}", version))
        .unwrap_or_else(|| "未知版本".into());
    let generation_label = bundle
        .working_generation
        .map(|generation| format!("工作集 #{generation}"))
        .unwrap_or_else(|| "未标注".into());
    let indices = if bundle.explain.indices_used.is_empty() {
        vec!["无".to_string()]
    } else {
        bundle.explain.indices_used.clone()
    };
    let query_hash = bundle
        .explain
        .query_hash
        .as_ref()
        .map(|hash| hash.as_str())
        .unwrap_or("-");

    Some(rsx! {
        div { class: "rounded-lg border border-slate-200 bg-slate-50 p-4 shadow-sm text-xs text-slate-600 space-y-2",
            header { class: "flex flex-wrap items-center justify-between gap-2",
                h3 { class: "text-sm font-semibold text-slate-800", "Manifest 概览" }
                span { class: "text-[11px] text-slate-500", "最近更新: {seen_at}" }
            }
            div { class: "flex flex-wrap items-center gap-2 text-[11px] text-slate-500",
                span { class: "font-semibold text-slate-700", "Digest" }
                span { class: "font-mono break-all text-slate-600", "{digest}" }
                span { class: "rounded bg-slate-200 px-2 py-0.5 text-[11px] text-slate-700", "{manifest_version}" }
                span { class: "rounded bg-slate-100 px-2 py-0.5 text-[11px] text-slate-600", "{generation_label}" }
            }
            div { class: "flex flex-wrap gap-2 text-[11px] text-slate-500",
                span { class: "font-semibold text-slate-700", "关联周期" }
                span { class: "rounded bg-white px-2 py-0.5 text-slate-600 shadow-inner", "{cycle_label}" }
            }
            div { class: "space-y-1",
                span { class: "text-[11px] font-semibold text-slate-700", "Indices" }
                div { class: "flex flex-wrap gap-1",
                    for idx in indices.iter() {
                        span { class: "rounded bg-white px-2 py-0.5 text-[11px] text-slate-700 shadow-inner", "{idx}" }
                    }
                }
            }
            div { class: "flex flex-wrap gap-2 text-[11px] text-slate-500",
                span { class: "font-semibold text-slate-700", "Query Hash" }
                span { class: "font-mono break-all text-slate-600", "{query_hash}" }
            }
        }
    })
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
    let manifest_digest = bundle.manifest_digest.as_deref().unwrap_or("未提供");
    let degrade = bundle
        .degradation_reason
        .as_ref()
        .or_else(|| bundle.explain.degradation_reason.as_ref())
        .map(|code| format_degradation_label(code))
        .unwrap_or_else(|| "正常".into());

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            h3 { class: "text-sm font-semibold text-slate-800", "Anchor" }
            ul { class: "mt-2 space-y-1 text-xs text-slate-600",
                li { "Tenant: {tenant_label}" }
                li { "Session: {session_label}" }
                li { "Envelope: {envelope_label}" }
                li { "{config_info}" }
                li { "Scenario: {scenario_label}" }
                li { "Manifest: {manifest_digest}" }
                li { "降级状态: {degrade}" }
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

fn render_manifest_explain(bundle: &ContextBundleView) -> Option<Element> {
    let explain = &bundle.explain;
    let has_indices = !explain.indices_used.is_empty();
    let has_query = explain.query_hash.is_some();
    let has_reasons = !explain.reasons.is_empty();
    let degrade = bundle
        .degradation_reason
        .as_ref()
        .or_else(|| explain.degradation_reason.as_ref())
        .map(|code| format_degradation_label(code));

    if !has_indices && !has_query && !has_reasons && degrade.is_none() {
        return None;
    }

    Some(rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-2",
            h3 { class: "text-sm font-semibold text-slate-800", "Manifest Explain" }
            if let Some(status) = degrade {
                div { class: "text-xs text-slate-600",
                    span { class: "font-semibold text-slate-700", "降级状态: " }
                    span { "{status}" }
                }
            }
            if has_indices {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold text-slate-500", "Indices" }
                    div { class: "flex flex-wrap gap-1",
                        for idx in explain.indices_used.iter() {
                            span { class: "rounded bg-slate-100 px-2 py-0.5 text-[11px] text-slate-700", "{idx}" }
                        }
                    }
                }
            }
            if let Some(query) = explain.query_hash.as_ref() {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold text-slate-500", "Query Hash" }
                    span { class: "font-mono text-[11px] text-slate-600 break-all", "{query}" }
                }
            }
            if has_reasons {
                div { class: "space-y-1",
                    span { class: "text-[11px] font-semibold text-slate-500", "Explain 理由" }
                    ul { class: "list-disc pl-4 text-[11px] text-slate-600 space-y-1",
                        for reason in explain.reasons.iter() {
                            li { "{reason}" }
                        }
                    }
                }
            }
        }
    })
}

fn render_manifest_history(
    history: &[ManifestDigestRecord],
    active_digest: Option<&str>,
) -> Option<Element> {
    if history.is_empty() {
        return None;
    }

    Some(rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-3",
            div { class: "flex items-center justify-between",
                h3 { class: "text-sm font-semibold text-slate-800", "Manifest 历史" }
                span { class: "text-[11px] text-slate-500", "共 {history.len()} 条" }
            }
            ul { class: "space-y-2",
                for record in history.iter().take(8) {
                    {render_manifest_history_item(record, active_digest)}
                }
            }
        }
    })
}

fn render_manifest_history_item(
    record: &ManifestDigestRecord,
    active_digest: Option<&str>,
) -> Element {
    let is_active = active_digest
        .map(|digest| digest == record.manifest_digest)
        .unwrap_or(false);
    let card_class = if is_active {
        "border-indigo-400 bg-indigo-50"
    } else {
        "border-slate-200 bg-slate-50"
    };
    let digest_label = &record.manifest_digest;
    let seen_at = record.seen_at.clone().unwrap_or_else(|| "未知时间".into());
    let cycles = if record.cycle_ids.is_empty() {
        "-".to_string()
    } else {
        record.cycle_ids.join(", ")
    };
    let bundle = record.bundle.as_ref();
    let degrade = bundle
        .and_then(|bundle| {
            bundle
                .degradation_reason
                .as_ref()
                .or_else(|| bundle.explain.degradation_reason.as_ref())
                .map(|code| format_degradation_label(code))
        })
        .unwrap_or_else(|| "正常".into());
    let segment_count = bundle.map(|b| b.segments.len()).unwrap_or(0);
    let indices: Vec<String> = bundle
        .map(|b| b.explain.indices_used.clone())
        .unwrap_or_default();
    let query_hash = bundle
        .and_then(|b| b.explain.query_hash.clone())
        .unwrap_or_else(|| "-".into());

    rsx! {
        li { class: format!("rounded border px-3 py-2 text-[11px] text-slate-600 shadow-sm {}", card_class),
            div { class: "flex flex-wrap items-center justify-between gap-2",
                span { class: "font-semibold text-slate-800", "{digest_label}" }
                span { class: "text-[11px] text-slate-500", "{seen_at}" }
            }
            div { class: "flex flex-wrap gap-2 text-[11px] text-slate-500",
                span { "周期: {cycles}" }
                span { "Segments: {segment_count}" }
                span { "降级: {degrade}" }
            }
            div { class: "flex flex-wrap gap-1",
                if indices.is_empty() {
                    span { class: "rounded bg-slate-100 px-2 py-0.5 text-slate-500", "无索引命中" }
                } else {
                    for idx in indices.iter() {
                        span { class: "rounded bg-slate-200 px-2 py-0.5 text-slate-700", "{idx}" }
                    }
                }
            }
            div { class: "font-mono text-[10px] text-slate-500 break-all", "Query: {query_hash}" }
        }
    }
}

fn format_degradation_label(value: &str) -> String {
    value
        .split(|ch| ch == '_' || ch == '-')
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
