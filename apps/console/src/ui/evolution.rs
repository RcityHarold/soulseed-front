//! 演化事件面板
//!
//! 展示群体演化、个体 AI 演化、关系演化等事件

use dioxus::prelude::*;

use crate::hooks::evolution::{
    use_ai_evolution, use_evolution_overview, use_group_evolution, use_relationship_evolution,
};
use crate::models::{
    AiEvolutionEvent, AiEvolutionListResponse, GroupEvolutionEvent, GroupEvolutionListResponse,
    GroupEvolutionEventType, RelationshipEvolutionEvent, RelationshipEvolutionListResponse,
};

/// 演化事件综合面板
#[component]
pub fn EvolutionPanel() -> Element {
    let evolution_state = use_evolution_overview();

    let body = {
        let state = evolution_state.read();
        if state.loading {
            rsx! { p { class: "text-xs text-slate-500", "正在加载演化事件..." } }
        } else if let Some(ref err) = state.error {
            rsx! { p { class: "text-xs text-red-500", "加载失败: {err}" } }
        } else {
            rsx! {
                div { class: "space-y-4",
                    // 群体演化
                    if let Some(ref group) = state.group_events {
                        {render_group_evolution(group)}
                    }
                    // 关系演化
                    if let Some(ref rel) = state.relationship_events {
                        {render_relationship_evolution(rel)}
                    }
                }
            }
        }
    };

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "演化事件" }
                p { class: "text-xs text-slate-500", "AI 群体与个体的演化历程" }
            }
            {body}
        }
    }
}

/// 群体演化面板
#[component]
pub fn GroupEvolutionPanel() -> Element {
    let events = use_group_evolution();

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "群体演化" }
                p { class: "text-xs text-slate-500", "AI 群体层面的演化事件" }
            }
            if let Some(ref data) = *events.read() {
                {render_group_evolution(data)}
            } else {
                div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                    p { class: "text-xs text-slate-500 italic", "暂无群体演化事件" }
                }
            }
        }
    }
}

fn render_group_evolution(data: &GroupEvolutionListResponse) -> Element {
    if data.events.is_empty() {
        return rsx! {
            div { class: "rounded-lg border border-slate-200 bg-white p-4",
                h3 { class: "text-sm font-semibold text-slate-800 mb-2", "群体演化" }
                p { class: "text-xs text-slate-500 italic", "暂无群体演化事件" }
            }
        };
    }

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            div { class: "flex items-center justify-between mb-3",
                h3 { class: "text-sm font-semibold text-slate-800", "群体演化" }
                span { class: "text-xs text-slate-500", {format!("共 {} 条", data.total)} }
            }
            div { class: "space-y-2 max-h-96 overflow-y-auto",
                for event in data.events.iter() {
                    {render_group_event(event)}
                }
            }
        }
    }
}

fn render_group_event(event: &GroupEvolutionEvent) -> Element {
    let event_color = match event.event_type {
        GroupEvolutionEventType::GroupCreated => "bg-green-100 text-green-700 border-green-200",
        GroupEvolutionEventType::GroupMemberJoined => "bg-blue-100 text-blue-700 border-blue-200",
        GroupEvolutionEventType::GroupMemberLeft => "bg-orange-100 text-orange-700 border-orange-200",
        GroupEvolutionEventType::GroupDisbanded => "bg-red-100 text-red-700 border-red-200",
        _ => "bg-slate-100 text-slate-700 border-slate-200",
    };

    let event_type_str = format!("{:?}", event.event_type);

    rsx! {
        div { class: "p-3 bg-slate-50 rounded-lg border border-slate-100",
            div { class: "flex items-center justify-between mb-2",
                div { class: "flex items-center gap-2",
                    span { class: format!("text-xs px-2 py-1 rounded border {}", event_color),
                        "{event_type_str}"
                    }
                }
                span { class: "text-xs text-slate-400", {format!("{}", event.occurred_at_ms)} }
            }
            // 群组信息
            div { class: "flex items-center gap-2 text-xs text-slate-600",
                span { "群组 ID: {event.group_id}" }
                if let Some(ref name) = event.group_name {
                    span { class: "text-slate-400", "|" }
                    span { "名称: {name}" }
                }
            }
            span { class: "text-xs text-slate-500", "操作者: {event.actor}" }
        }
    }
}

/// 个体 AI 演化面板
#[component]
pub fn AiEvolutionPanel() -> Element {
    let mut ai_id = use_signal(|| String::new());

    let events = use_ai_evolution(ai_id.read().clone());

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "个体演化" }
                p { class: "text-xs text-slate-500", "单个 AI 实体的演化历程" }
            }
            // 搜索
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                div { class: "flex gap-2",
                    input {
                        r#type: "text",
                        class: "flex-1 px-3 py-2 text-sm border border-slate-200 rounded-lg",
                        placeholder: "输入 AI ID...",
                        value: "{ai_id}",
                        oninput: move |evt| ai_id.set(evt.value().clone())
                    }
                }
            }
            // 事件列表
            if let Some(ref data) = *events.read() {
                {render_ai_evolution(data)}
            } else {
                div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                    p { class: "text-xs text-slate-500 italic", "输入 AI ID 以查看演化历程" }
                }
            }
        }
    }
}

fn render_ai_evolution(data: &AiEvolutionListResponse) -> Element {
    let events_list = data.events();
    if events_list.is_empty() {
        return rsx! {
            div { class: "rounded-lg border border-slate-200 bg-white p-4",
                p { class: "text-xs text-slate-500 italic", "该 AI 暂无演化事件" }
            }
        };
    }

    let total = data.total_events();
    let ai_name = data.ai_name();
    let current_version = data.current_version();

    rsx! {
        div { class: "space-y-4",
            // AI 概览
            div { class: "rounded-lg border border-slate-200 bg-slate-50 p-4",
                div { class: "grid grid-cols-3 gap-4 text-center",
                    div {
                        p { class: "text-xl font-bold text-blue-600", "{total}" }
                        p { class: "text-xs text-slate-500", "总事件数" }
                    }
                    div {
                        p { class: "text-xl font-bold text-green-600", "{ai_name}" }
                        p { class: "text-xs text-slate-500", "AI 名称" }
                    }
                    div {
                        p { class: "text-xl font-bold text-purple-600", "v{current_version}" }
                        p { class: "text-xs text-slate-500", "当前版本" }
                    }
                }
            }
            // 事件列表
            div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                h3 { class: "text-sm font-semibold text-slate-800 mb-3", "演化历程" }
                div { class: "space-y-2 max-h-96 overflow-y-auto",
                    for event in events_list.iter() {
                        {render_ai_event(event)}
                    }
                }
            }
        }
    }
}

fn render_ai_event(event: &AiEvolutionEvent) -> Element {
    let event_color = "bg-slate-100 text-slate-700";

    rsx! {
        div { class: "p-3 bg-slate-50 rounded-lg border border-slate-100",
            div { class: "flex items-center justify-between mb-2",
                div { class: "flex items-center gap-2",
                    span { class: format!("text-xs px-2 py-1 rounded {}", event_color),
                        "{event.event_type}"
                    }
                    span { class: "text-xs text-slate-500",
                        {format!("v{} → v{}", event.from_version, event.to_version)}
                    }
                }
                span { class: "text-xs text-slate-400", "{event.occurred_at}" }
            }
            p { class: "text-sm text-slate-700", "{event.description}" }
            // 变更详情
            if !event.changes.is_empty() {
                div { class: "mt-2 flex flex-wrap gap-1",
                    for change in event.changes.iter().take(5) {
                        span { class: "text-xs px-2 py-0.5 bg-slate-200 text-slate-600 rounded",
                            "{change}"
                        }
                    }
                }
            }
        }
    }
}

/// 关系演化面板
#[component]
pub fn RelationshipEvolutionPanel() -> Element {
    let events = use_relationship_evolution();

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "关系演化" }
                p { class: "text-xs text-slate-500", "AI 实体之间关系的演化" }
            }
            if let Some(ref data) = *events.read() {
                {render_relationship_evolution(data)}
            } else {
                div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                    p { class: "text-xs text-slate-500 italic", "暂无关系演化事件" }
                }
            }
        }
    }
}

fn render_relationship_evolution(data: &RelationshipEvolutionListResponse) -> Element {
    if data.events.is_empty() {
        return rsx! {
            div { class: "rounded-lg border border-slate-200 bg-white p-4",
                h3 { class: "text-sm font-semibold text-slate-800 mb-2", "关系演化" }
                p { class: "text-xs text-slate-500 italic", "暂无关系演化事件" }
            }
        };
    }

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            div { class: "flex items-center justify-between mb-3",
                h3 { class: "text-sm font-semibold text-slate-800", "关系演化" }
                span { class: "text-xs text-slate-500", {format!("共 {} 条", data.total)} }
            }
            div { class: "space-y-2 max-h-96 overflow-y-auto",
                for event in data.events.iter() {
                    {render_relationship_event(event)}
                }
            }
        }
    }
}

fn render_relationship_event(event: &RelationshipEvolutionEvent) -> Element {
    let event_type_str = event.event_type_str();
    let event_color = "bg-slate-100 text-slate-700";

    let from_strength = event.from_strength();
    let to_strength = event.to_strength();
    let strength_change = to_strength - from_strength;
    let change_color = if strength_change > 0.0 {
        "text-green-600"
    } else if strength_change < 0.0 {
        "text-red-600"
    } else {
        "text-slate-600"
    };

    let entity_a = event.entity_a();
    let entity_b = event.entity_b();
    let relationship_type = event.relationship_type();
    let reason = event.reason();

    rsx! {
        div { class: "p-3 bg-slate-50 rounded-lg border border-slate-100",
            div { class: "flex items-center justify-between mb-2",
                span { class: format!("text-xs px-2 py-1 rounded {}", event_color),
                    "{event_type_str}"
                }
                span { class: "text-xs text-slate-400", {format!("{}", event.occurred_at_ms)} }
            }
            // 关系双方
            div { class: "flex items-center gap-2 mb-2",
                span { class: "text-sm font-medium text-slate-700", "{entity_a}" }
                span { class: "text-slate-400", "↔" }
                span { class: "text-sm font-medium text-slate-700", "{entity_b}" }
            }
            // 关系变化
            div { class: "flex items-center gap-2 text-xs",
                span { class: "text-slate-500", "关系类型:" }
                span { class: "font-medium text-slate-700", "{relationship_type}" }
                span { class: "text-slate-400", "|" }
                span { class: "text-slate-500", "强度变化:" }
                span { class: change_color,
                    {format!("{:.2} → {:.2}", from_strength, to_strength)}
                }
            }
            // 原因
            if let Some(ref r) = reason {
                p { class: "mt-2 text-xs text-slate-600", "{r}" }
            }
        }
    }
}
