use std::collections::{BTreeMap, HashSet};

use crate::models::{
    AceCycleSummary, AceLane, AwarenessDegradationReason, DecisionPlan, DialogueEvent,
    DialogueEventType,
};
use crate::state::use_app_state;
use dioxus::prelude::*;
use serde_json::Value;

pub fn ToolTracePanel(cx: Scope) -> Element {
    let app_state = use_app_state(cx);
    let snapshot = app_state.read();
    let timeline_events = snapshot.timeline.events.clone();
    let cycles = snapshot.ace.cycles.clone();
    drop(snapshot);

    let traces = collect_tool_traces(&timeline_events);
    let stats = TraceStats::from_traces(&traces);
    let lane_summaries = aggregate_lane_summaries(&cycles, &traces, &stats);
    let llm_traces = collect_llm_traces(&timeline_events);

    rsx! {
        section { class: "space-y-4",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "工具与 LLM 轨迹" }
                p { class: "text-xs text-slate-500", "聚合工具调用、ACE 决策路径与推理轨迹，辅助定位性能与降级原因。" }
                SummaryRow { stats: stats.clone() }
            }

            if !lane_summaries.is_empty() {
                div { class: "grid gap-3 md:grid-cols-2 xl:grid-cols-4",
                    for summary in lane_summaries.iter().cloned() {
                        LaneCard { summary }
                    }
                }
            }

            if traces.is_empty() {
                p { class: "text-xs text-slate-500 italic", "暂无工具调用数据" }
            } else {
                div { class: "overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm",
                    table { class: "min-w-full divide-y divide-slate-200 text-xs",
                        thead { class: "bg-slate-50 text-slate-600",
                            tr {
                                th { class: TH_CLASS, "开始" }
                                th { class: TH_CLASS, "工具" }
                                th { class: TH_CLASS, "调用 ID" }
                                th { class: TH_CLASS, "状态" }
                                th { class: TH_CLASS, "模型" }
                                th { class: TH_CLASS, "Tokens(p/c)" }
                                th { class: TH_CLASS, "耗时" }
                                th { class: TH_CLASS, "降级" }
                                th { class: TH_CLASS, "摘要" }
                            }
                        }
                        tbody { class: "divide-y divide-slate-100",
                            for trace in traces.iter() {
                                tr {
                                    td { class: TD_CLASS, format!("#{}", trace.first_event_id) }
                                    td { class: TD_CLASS, &trace.tool_id }
                                    td { class: TD_CLASS, &trace.call_id }
                                    td { class: TD_CLASS,
                                        span { class: status_class(trace.success), detail_label(trace.success) }
                                    }
                                    td { class: TD_CLASS, trace.model.clone().unwrap_or_else(|| "-".into()) }
                                    td { class: TD_CLASS, format_tokens(trace.prompt_tokens, trace.completion_tokens) }
                                    td { class: TD_CLASS, format_duration(trace.duration_ms) }
                                    td { class: TD_CLASS,
                                        trace
                                            .degradation_reason
                                            .clone()
                                            .unwrap_or_else(|| "-".into())
                                    }
                                    td { class: TD_CLASS,
                                        trace
                                            .summary
                                            .clone()
                                            .unwrap_or_else(|| "-".into())
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !llm_traces.is_empty() {
                div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-2",
                    h3 { class: "text-sm font-semibold text-slate-800", "LLM 计划 / 推理轨迹" }
                    for trace in llm_traces.iter() {
                        div { class: "rounded border border-slate-100 bg-slate-50 p-3 text-xs text-slate-600 space-y-1",
                            p { class: "font-medium text-slate-700", format!("事件 #{}", trace.event_id) }
                            if let Some(model) = trace.model.as_ref() {
                                p { "模型: {model}" }
                            }
                            if let Some(plan) = trace.plan.as_ref() {
                                p { "策略: {plan}" }
                            }
                            if let Some(reasoning) = trace.reasoning.as_ref() {
                                p { class: "text-slate-500", reasoning }
                            }
                            if trace.prompt_tokens.is_some() || trace.completion_tokens.is_some() {
                                p { format!(
                                    "Tokens(prompt/comp): {}",
                                    format_tokens(trace.prompt_tokens, trace.completion_tokens)
                                ) }
                            }
                            if let Some(conf) = trace.confidence {
                                p { format!("置信度: {}", format_confidence(conf)) }
                            }
                        }
                    }
                }
            }
        }
    }
}

const TH_CLASS: &str = "px-3 py-2 text-left text-[11px] font-semibold uppercase tracking-wide";
const TD_CLASS: &str = "px-3 py-2 align-top";

#[derive(Clone, Debug, Default)]
struct ToolTraceRow {
    first_event_id: u64,
    timestamp_ms: Option<i64>,
    tool_id: String,
    call_id: String,
    success: Option<bool>,
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    duration_ms: Option<u64>,
    model: Option<String>,
    summary: Option<String>,
    degradation_reason: Option<String>,
}

impl ToolTraceRow {
    fn new(tool_id: String, call_id: String, event: &DialogueEvent) -> Self {
        let mut row = ToolTraceRow {
            tool_id,
            call_id,
            ..Default::default()
        };
        row.first_event_id = event.event_id.into_inner();
        row.timestamp_ms = Some(event.timestamp_ms);
        row.update_from_event(event);
        row
    }

    fn update_from_event(&mut self, event: &DialogueEvent) {
        self.first_event_id = self.first_event_id.min(event.event_id.into_inner());
        self.timestamp_ms = Some(match self.timestamp_ms {
            Some(existing) => existing.min(event.timestamp_ms),
            None => event.timestamp_ms,
        });

        if self.tool_id == "unknown" {
            if let Some(tool_id) = event
                .tool_invocation
                .as_ref()
                .map(|inv| inv.tool_id.clone())
                .or_else(|| event.tool_result.as_ref().map(|res| res.tool_id.clone()))
            {
                self.tool_id = tool_id;
            }
        }

        if let Some(result) = event.tool_result.as_ref() {
            self.success = Some(result.success);
            if let Some(reason) = result.degradation_reason.as_ref() {
                self.degradation_reason = Some(normalize_tool_degradation(reason));
            }
            if self.summary.is_none() {
                self.summary = result
                    .output
                    .get("highlights")
                    .and_then(|val| val.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str())
                            .collect::<Vec<_>>()
                            .join("；")
                    });
            }
        }

        accumulate_tokens(
            &mut self.prompt_tokens,
            tokens_from_metadata(&event.metadata, "prompt"),
        );
        accumulate_tokens(
            &mut self.completion_tokens,
            tokens_from_metadata(&event.metadata, "completion"),
        );
        combine_duration(
            &mut self.duration_ms,
            duration_from_metadata(&event.metadata),
        );

        if self.model.is_none() {
            self.model = event
                .metadata
                .get("model")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
        }
    }
}

#[derive(Clone, Debug, Default)]
struct TraceStats {
    total_calls: usize,
    success_calls: usize,
    failure_calls: usize,
    total_prompt_tokens: u32,
    total_completion_tokens: u32,
    total_duration_ms: u64,
    duration_samples: usize,
}

impl TraceStats {
    fn from_traces(traces: &[ToolTraceRow]) -> Self {
        let mut stats = TraceStats::default();
        for trace in traces {
            stats.record_trace(trace);
        }
        stats
    }

    fn record_trace(&mut self, trace: &ToolTraceRow) {
        self.total_calls += 1;
        match trace.success {
            Some(true) => self.success_calls += 1,
            Some(false) => self.failure_calls += 1,
            None => {}
        }

        self.total_prompt_tokens += trace.prompt_tokens.unwrap_or(0);
        self.total_completion_tokens += trace.completion_tokens.unwrap_or(0);

        if let Some(duration) = trace.duration_ms {
            self.total_duration_ms += duration;
            self.duration_samples += 1;
        }
    }

    fn success_rate_pct(&self) -> f32 {
        let evaluated = self.success_calls + self.failure_calls;
        if evaluated == 0 {
            0.0
        } else {
            self.success_calls as f32 / evaluated as f32 * 100.0
        }
    }

    fn average_latency_ms(&self) -> Option<f32> {
        if self.duration_samples == 0 {
            None
        } else {
            Some(self.total_duration_ms as f32 / self.duration_samples as f32)
        }
    }

    fn pending_calls(&self) -> usize {
        self.total_calls
            .saturating_sub(self.success_calls + self.failure_calls)
    }

    fn prompt_tokens(&self) -> u32 {
        self.total_prompt_tokens
    }

    fn completion_tokens(&self) -> u32 {
        self.total_completion_tokens
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct SummaryRowProps {
    stats: TraceStats,
}

fn SummaryRow(cx: Scope<SummaryRowProps>) -> Element {
    let stats = cx.props.stats.clone();
    let success_rate = stats.success_rate_pct();
    let avg_latency = stats
        .average_latency_ms()
        .map(|ms| format!("{:.0}ms", ms))
        .unwrap_or_else(|| "-".into());

    cx.render(rsx! {
        div { class: "flex flex-wrap items-center gap-4 text-xs text-slate-600",
            span { format!("调用: {}", stats.total_calls) }
            span { format!("成功: {}", stats.success_calls) }
            span { format!("失败: {}", stats.failure_calls) }
            span { format!("待决: {}", stats.pending_calls()) }
            span { format!("成功率: {:.1}%", success_rate) }
            span { format!("平均耗时: {}", avg_latency) }
            span { format!(
                "Tokens(prompt/comp): {}/{}",
                stats.prompt_tokens(),
                stats.completion_tokens()
            ) }
        }
    })
}

#[derive(Clone, Debug)]
struct LaneSummary {
    lane: AceLane,
    cycle_count: usize,
    tokens_spent: u32,
    tokens_allowed: u32,
    walltime_spent_ms: u64,
    walltime_allowed_ms: u64,
    planned_tokens: Option<u32>,
    planned_walltime_ms: Option<u32>,
    confidence_avg: Option<f32>,
    degradation_reasons: Vec<String>,
    plan_highlights: Vec<String>,
    tool_metrics: Option<TraceStats>,
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct LaneCardProps {
    summary: LaneSummary,
}

fn LaneCard(cx: Scope<LaneCardProps>) -> Element {
    let summary = &cx.props.summary;
    let confidence = summary.confidence_avg.map(|value| format_confidence(value));

    cx.render(rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-2 text-xs text-slate-600",
            header { class: "flex items-start justify-between",
                div {
                    h3 { class: "text-sm font-semibold text-slate-800", format!("{} Lane", format_lane(&summary.lane)) }
                    p { class: "text-[11px] text-slate-500", format!("{} 个周期", summary.cycle_count) }
                }
                if let Some(conf) = confidence {
                    span { class: "rounded-full bg-slate-100 px-2 py-0.5 text-[11px] text-slate-700", format!("置信度 {conf}") }
                }
            }

            p { format!(
                "Token 使用: {}",
                format_budget_pair(summary.tokens_spent, summary.tokens_allowed)
            ) }

            if summary.walltime_spent_ms > 0 || summary.walltime_allowed_ms > 0 {
                p { format!(
                    "Walltime: {}",
                    format_budget_pair_u64(summary.walltime_spent_ms, summary.walltime_allowed_ms)
                ) }
            }

            if let Some(tokens) = summary.planned_tokens {
                p { format!("计划 Token: {}", tokens) }
            }

            if let Some(walltime) = summary.planned_walltime_ms {
                p { format!("计划 Walltime: {}", format_ms(walltime as u64)) }
            }

            if let Some(metrics) = summary.tool_metrics.as_ref() {
                div { class: "rounded border border-slate-100 bg-slate-50 p-2 space-y-1",
                    p { format!(
                        "调用 {}/{} 成功 (待决 {})",
                        metrics.success_calls,
                        metrics.total_calls,
                        metrics.pending_calls()
                    ) }
                    p { format!("成功率: {:.1}%", metrics.success_rate_pct()) }
                    if let Some(avg) = metrics.average_latency_ms() {
                        p { format!("平均耗时: {:.0}ms", avg) }
                    }
                    p { format!(
                        "Tokens(prompt/comp): {}/{}",
                        metrics.prompt_tokens(),
                        metrics.completion_tokens()
                    ) }
                }
            }

            if !summary.degradation_reasons.is_empty() {
                div { class: "flex flex-wrap gap-1",
                    for reason in summary.degradation_reasons.iter() {
                        span { class: "rounded bg-amber-100 px-2 py-0.5 text-[11px] text-amber-800", reason }
                    }
                }
            }

            if !summary.plan_highlights.is_empty() {
                ul { class: "list-disc space-y-1 pl-4 text-[11px] text-slate-500",
                    for highlight in summary.plan_highlights.iter() {
                        li { highlight }
                    }
                }
            }
        }
    })
}

#[derive(Clone, Debug)]
struct LlmTraceRow {
    event_id: u64,
    plan: Option<String>,
    reasoning: Option<String>,
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    confidence: Option<f32>,
    model: Option<String>,
}

fn collect_tool_traces(events: &[DialogueEvent]) -> Vec<ToolTraceRow> {
    let mut map: BTreeMap<String, ToolTraceRow> = BTreeMap::new();

    for event in events.iter().filter(|event| {
        matches!(
            event.event_type,
            DialogueEventType::ToolCall | DialogueEventType::ToolResult
        )
    }) {
        let call_id = event
            .tool_invocation
            .as_ref()
            .map(|inv| inv.call_id.clone())
            .or_else(|| event.tool_result.as_ref().map(|res| res.call_id.clone()))
            .unwrap_or_else(|| format!("evt-{}", event.event_id.into_inner()));
        let tool_id = event
            .tool_invocation
            .as_ref()
            .map(|inv| inv.tool_id.clone())
            .or_else(|| event.tool_result.as_ref().map(|res| res.tool_id.clone()))
            .unwrap_or_else(|| "unknown".to_string());

        let entry = map
            .entry(call_id.clone())
            .or_insert_with(|| ToolTraceRow::new(tool_id, call_id, event));
        entry.update_from_event(event);
    }

    let mut traces: Vec<_> = map.into_values().collect();
    traces.sort_by_key(|trace| trace.timestamp_ms.unwrap_or(0));
    traces
}

fn collect_llm_traces(events: &[DialogueEvent]) -> Vec<LlmTraceRow> {
    let mut traces: Vec<_> = events
        .iter()
        .filter(|event| event.reasoning_trace.is_some() || event.reasoning_strategy.is_some())
        .map(|event| {
            let prompt_tokens = tokens_from_metadata(&event.metadata, "prompt");
            let completion_tokens = tokens_from_metadata(&event.metadata, "completion");
            let model = event
                .metadata
                .get("model")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());

            LlmTraceRow {
                event_id: event.event_id.into_inner(),
                plan: event.reasoning_strategy.clone(),
                reasoning: event.reasoning_trace.clone(),
                prompt_tokens,
                completion_tokens,
                confidence: event.reasoning_confidence,
                model,
            }
        })
        .collect();

    traces.sort_by_key(|trace| trace.event_id);
    traces
}

fn aggregate_lane_summaries(
    cycles: &[AceCycleSummary],
    traces: &[ToolTraceRow],
    stats: &TraceStats,
) -> Vec<LaneSummary> {
    let mut summaries = Vec::new();

    for lane in [
        AceLane::Clarify,
        AceLane::Tool,
        AceLane::SelfReason,
        AceLane::Collab,
    ] {
        let relevant_cycles: Vec<&AceCycleSummary> =
            cycles.iter().filter(|cycle| cycle.lane == lane).collect();
        let is_tool_lane = matches!(lane, AceLane::Tool);
        if relevant_cycles.is_empty() && !(is_tool_lane && !traces.is_empty()) {
            continue;
        }

        let mut tokens_spent = 0u32;
        let mut tokens_allowed = 0u32;
        let mut walltime_spent_ms = 0u64;
        let mut walltime_allowed_ms = 0u64;
        let mut planned_tokens = 0u32;
        let mut planned_walltime = 0u32;
        let mut has_planned_tokens = false;
        let mut has_planned_walltime = false;
        let mut confidence_sum = 0.0f32;
        let mut confidence_samples = 0usize;
        let mut degradation: HashSet<String> = HashSet::new();
        let mut plan_highlights: HashSet<String> = HashSet::new();

        for cycle in relevant_cycles.iter() {
            if let Some(budget) = cycle.budget.as_ref() {
                tokens_spent += budget.tokens_spent.unwrap_or(0);
                tokens_allowed += budget.tokens_allowed.unwrap_or(0);
                walltime_spent_ms += budget.walltime_ms_used.unwrap_or(0);
                walltime_allowed_ms += budget.walltime_ms_allowed.unwrap_or(0);
            }

            if let Some(path) = cycle.decision_path.as_ref() {
                confidence_sum += path.confidence;
                confidence_samples += 1;

                if let Some(reason) = path.degradation_reason.as_ref() {
                    degradation.insert(format_degradation_reason(reason));
                }

                if let Some(tokens) = path.budget_plan.tokens {
                    planned_tokens += tokens;
                    has_planned_tokens = true;
                }

                if let Some(walltime) = path.budget_plan.walltime_ms {
                    planned_walltime += walltime;
                    has_planned_walltime = true;
                }

                for highlight in describe_plan(&path.plan) {
                    plan_highlights.insert(highlight);
                }
            }
        }

        if is_tool_lane {
            for trace in traces {
                if let Some(reason) = trace.degradation_reason.as_ref() {
                    degradation.insert(reason.clone());
                }
            }
        }

        let mut degradation_reasons: Vec<String> = degradation.into_iter().collect();
        degradation_reasons.sort();
        let mut plan_highlights: Vec<String> = plan_highlights.into_iter().collect();
        plan_highlights.sort();

        let confidence_avg = if confidence_samples > 0 {
            Some(confidence_sum / confidence_samples as f32)
        } else {
            None
        };

        summaries.push(LaneSummary {
            lane,
            cycle_count: relevant_cycles.len(),
            tokens_spent,
            tokens_allowed,
            walltime_spent_ms,
            walltime_allowed_ms,
            planned_tokens: if has_planned_tokens {
                Some(planned_tokens)
            } else {
                None
            },
            planned_walltime_ms: if has_planned_walltime {
                Some(planned_walltime)
            } else {
                None
            },
            confidence_avg,
            degradation_reasons,
            plan_highlights,
            tool_metrics: if is_tool_lane {
                Some(stats.clone())
            } else {
                None
            },
        });
    }

    summaries
}

fn describe_plan(plan: &DecisionPlan) -> Vec<String> {
    match plan {
        DecisionPlan::Clarify { plan } => {
            let mut highlights = Vec::new();
            highlights.push(format!("Clarify 问题数 {}", plan.questions.len()));
            if let Some(first) = plan.questions.first() {
                highlights.push(format!("首题: {}", first.text));
            }
            if let Some(parallel) = plan.limits.max_parallel {
                highlights.push(format!("并行度 {}", parallel));
            }
            highlights
        }
        DecisionPlan::Tool { plan } => {
            let chain: Vec<_> = plan
                .nodes
                .iter()
                .map(|node| node.tool_id.as_str())
                .collect();
            let mut highlights = vec![format!("工具链: {}", chain.join(" → "))];
            if let Some(timeout) = plan.barrier.timeout_ms {
                highlights.push(format!("Barrier {}ms", timeout));
            }
            highlights
        }
        DecisionPlan::SelfReason { plan } => {
            let mut highlights = vec!["Self Reason".to_string()];
            if let Some(max_ic) = plan.max_ic {
                highlights.push(format!("最大轮次 {max_ic}"));
            }
            highlights
        }
        DecisionPlan::Collab { plan } => {
            let mut highlights = vec!["协作计划".to_string()];
            if let Some(order) = plan.order.as_ref() {
                highlights.push(format!("顺序 {order}"));
            }
            if let Some(rounds) = plan.rounds {
                highlights.push(format!("轮次 {rounds}"));
            }
            highlights
        }
    }
}

fn format_degradation_reason(reason: &AwarenessDegradationReason) -> String {
    match reason {
        AwarenessDegradationReason::BudgetTokens => "Token 预算不足".into(),
        AwarenessDegradationReason::BudgetWalltime => "耗时超限".into(),
        AwarenessDegradationReason::BudgetExternalCost => "外部成本受限".into(),
        AwarenessDegradationReason::EmptyCatalog => "工具目录为空".into(),
        AwarenessDegradationReason::PrivacyBlocked => "隐私策略阻断".into(),
        AwarenessDegradationReason::InvalidPlan => "计划无效".into(),
        AwarenessDegradationReason::ClarifyExhausted => "Clarify 已耗尽".into(),
        AwarenessDegradationReason::GraphDegraded => "图谱降级".into(),
        AwarenessDegradationReason::EnvctxDegraded => "上下文降级".into(),
    }
}

fn normalize_tool_degradation(reason: &str) -> String {
    match reason {
        "budget_tokens" => "Token 预算不足".into(),
        "sla_timeout" => "SLA 超时".into(),
        other => other.replace('_', " "),
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

fn format_tokens(prompt: Option<u32>, completion: Option<u32>) -> String {
    match (prompt, completion) {
        (Some(p), Some(c)) => format!("{p}/{c}"),
        (Some(p), None) => p.to_string(),
        (None, Some(c)) => c.to_string(),
        (None, None) => "-".into(),
    }
}

fn format_duration(duration: Option<u64>) -> String {
    duration.map(format_ms).unwrap_or_else(|| "-".into())
}

fn format_ms(ms: u64) -> String {
    if ms >= 1_000 {
        format!("{:.1}s", ms as f64 / 1_000.0)
    } else {
        format!("{ms}ms")
    }
}

fn format_budget_pair(spent: u32, allowed: u32) -> String {
    if allowed > 0 {
        format!("{spent} / {allowed}")
    } else {
        spent.to_string()
    }
}

fn format_budget_pair_u64(spent: u64, allowed: u64) -> String {
    if allowed > 0 {
        format!("{} / {}", format_ms(spent), format_ms(allowed))
    } else {
        format_ms(spent)
    }
}

fn format_confidence(value: f32) -> String {
    format!("{:.0}%", value * 100.0)
}

fn accumulate_tokens(target: &mut Option<u32>, addition: Option<u32>) {
    if let Some(value) = addition {
        *target.get_or_insert(0) += value;
    }
}

fn combine_duration(target: &mut Option<u64>, addition: Option<u64>) {
    if let Some(value) = addition {
        *target.get_or_insert(0) += value;
    }
}

fn tokens_from_metadata(metadata: &Value, field: &str) -> Option<u32> {
    metadata
        .get("tokens")
        .and_then(|tokens| tokens.get(field))
        .and_then(|value| value.as_u64())
        .map(|value| value as u32)
}

fn duration_from_metadata(metadata: &Value) -> Option<u64> {
    metadata.get("duration_ms").and_then(|value| value.as_u64())
}

fn status_class(success: Option<bool>) -> &'static str {
    match success {
        Some(true) => "rounded bg-green-100 px-2 py-0.5 text-green-700",
        Some(false) => "rounded bg-red-100 px-2 py-0.5 text-red-700",
        None => "rounded bg-slate-100 px-2 py-0.5 text-slate-600",
    }
}

fn detail_label(success: Option<bool>) -> &'static str {
    match success {
        Some(true) => "成功",
        Some(false) => "失败",
        None => "-",
    }
}
