//! 自主延续控制面板
//!
//! 展示和控制 AI 自主延续会话

use dioxus::prelude::*;
use std::rc::Rc;

use crate::hooks::autonomous::{use_autonomous_control, use_autonomous_session, AutonomousControlResult};
use crate::models::{AgendaItemInput, AutonomousConfig, AutonomousSessionSummary, AutonomousStatus, StartAutonomousRequest};

/// 启动表单组件 - 独立组件避免轮询导致的焦点丢失
/// 注意：agenda_input 状态从父组件传入，确保轮询时不会丢失
#[component]
fn StartSessionForm(
    starting: bool,
    agenda_input: Signal<String>,
    on_submit: EventHandler<StartAutonomousRequest>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut agenda_input = agenda_input;

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 space-y-4",
            h3 { class: "text-sm font-semibold text-slate-800", "启动自主延续会话" }
            div {
                label { class: "block text-xs font-medium text-slate-600 mb-1", "议程项目 (每行一个)" }
                textarea {
                    class: "w-full h-24 px-3 py-2 text-sm border border-slate-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "输入要执行的任务...\n例如:\n分析当前对话上下文\n生成解决方案建议",
                    value: "{agenda_input}",
                    oninput: move |evt| agenda_input.set(evt.value().clone())
                }
            }
            div { class: "flex gap-3 mt-4",
                button {
                    r#type: "button",
                    class: "rounded bg-slate-900 px-4 py-2 text-xs font-semibold text-white hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: starting,
                    onclick: move |_| {
                        let agenda_text = agenda_input.read().clone();
                        let items: Vec<AgendaItemInput> = agenda_text
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .enumerate()
                            .map(|(i, line)| AgendaItemInput {
                                description: line.trim().to_string(),
                                priority: (5 - i.min(4)) as u8,
                            })
                            .collect();

                        let timestamp = web_sys::js_sys::Date::now() as u64;
                        let request = StartAutonomousRequest {
                            session_id: format!("auto_{}", timestamp),
                            agenda_items: if items.is_empty() {
                                vec![AgendaItemInput {
                                    description: "默认任务: 自主探索与分析".to_string(),
                                    priority: 5,
                                }]
                            } else {
                                items
                            },
                            config: AutonomousConfig::default(),
                        };

                        on_submit.call(request);
                    },
                    {if starting { "启动中..." } else { "确认启动" }}
                }
                button {
                    r#type: "button",
                    class: "rounded border border-slate-300 px-4 py-2 text-xs text-slate-600 hover:bg-slate-100",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
            }
        }
    }
}

/// 自主延续控制面板组件
#[component]
pub fn AutonomousPanel() -> Element {
    let session_state = use_autonomous_session();
    let control = use_autonomous_control();
    let mut show_start_form = use_signal(|| false);
    // 将 agenda_input 状态提升到父组件，避免轮询时丢失
    let mut agenda_input = use_signal(|| String::new());

    let state = session_state.read();
    let starting = *control.starting.read();
    let terminating = *control.terminating.read();
    let show_form_val = *show_start_form.read();
    let start_fn = control.start;

    // 显示控制结果通知
    let result_notification = {
        let result = control.last_result.read();
        match result.as_ref() {
            Some(AutonomousControlResult::Started(id)) => {
                rsx! {
                    div { class: "rounded-lg border border-green-200 bg-green-50 p-3 mb-3",
                        p { class: "text-sm text-green-700", "会话已启动: {id}" }
                    }
                }
            }
            Some(AutonomousControlResult::Terminated(result)) => {
                rsx! {
                    div { class: "rounded-lg border border-blue-200 bg-blue-50 p-3 mb-3",
                        p { class: "text-sm text-blue-700", "会话已终止: {result.orchestration_id}" }
                    }
                }
            }
            Some(AutonomousControlResult::Error(err)) => {
                rsx! {
                    div { class: "rounded-lg border border-red-200 bg-red-50 p-3 mb-3",
                        p { class: "text-sm text-red-600", "操作失败: {err}" }
                    }
                }
            }
            None => rsx! {}
        }
    };

    // 主体内容
    if state.loading && state.sessions.is_empty() {
        return rsx! {
            section { class: "space-y-3",
                header { class: "flex flex-col gap-1",
                    h2 { class: "text-lg font-semibold text-slate-900", "自主延续" }
                    p { class: "text-xs text-slate-500", "AI 自主延续会话管理与监控" }
                }
                {result_notification}
                div { class: "flex items-center justify-center p-6",
                    div { class: "animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" }
                    p { class: "ml-3 text-sm text-slate-500", "正在加载会话状态..." }
                }
            }
        };
    }

    if let Some(ref err) = state.error {
        let err_msg = err.clone();
        return rsx! {
            section { class: "space-y-3",
                header { class: "flex flex-col gap-1",
                    h2 { class: "text-lg font-semibold text-slate-900", "自主延续" }
                    p { class: "text-xs text-slate-500", "AI 自主延续会话管理与监控" }
                }
                {result_notification}
                div { class: "rounded-lg border border-red-200 bg-red-50 p-4",
                    p { class: "text-sm text-red-600", "加载失败: {err_msg}" }
                }
            }
        };
    }

    let sessions = state.sessions.clone();
    let terminate_fn = Rc::new(control.terminate);

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "自主延续" }
                p { class: "text-xs text-slate-500", "AI 自主延续会话管理与监控" }
            }
            {result_notification}

            div { class: "space-y-4",
                // 会话列表
                if !sessions.is_empty() {
                    div { class: "space-y-3",
                        for session in sessions.iter() {
                            {render_session_card(session, terminating, terminate_fn.clone())}
                        }
                    }
                }

                // 启动表单 - 使用独立组件，状态从父组件传入
                // 使用 key 确保组件身份稳定，避免轮询时重建
                if show_form_val {
                    StartSessionForm {
                        key: "start-session-form",
                        starting: starting,
                        agenda_input: agenda_input,
                        on_submit: move |request: StartAutonomousRequest| {
                            start_fn(request);
                            show_start_form.set(false);
                            agenda_input.set(String::new());
                        },
                        on_cancel: move |_| {
                            show_start_form.set(false);
                        }
                    }
                } else {
                    // 启动按钮
                    div { class: "flex justify-center",
                        button {
                            r#type: "button",
                            class: "rounded bg-slate-900 px-4 py-2 text-xs font-semibold text-white hover:bg-slate-800",
                            onclick: move |_| show_start_form.set(true),
                            "+ 启动新会话"
                        }
                    }
                }

                // 空状态提示
                if sessions.is_empty() && !show_form_val {
                    div { class: "rounded-lg border border-slate-200 bg-slate-50 p-6 text-center",
                        div { class: "text-slate-400 mb-4",
                            svg {
                                class: "w-12 h-12 mx-auto",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "1.5",
                                    d: "M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                                }
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "1.5",
                                    d: "M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                                }
                            }
                        }
                        p { class: "text-sm text-slate-600 mb-1", "暂无自主延续会话" }
                        p { class: "text-xs text-slate-400", "自主延续模式允许 AI 持续执行任务议程" }
                    }
                }
            }
        }
    }
}

/// 渲染单个会话卡片
fn render_session_card(
    session: &AutonomousSessionSummary,
    terminating: bool,
    terminate_fn: Rc<Box<dyn Fn(String, String) + 'static>>,
) -> Element {
    let status_style = match session.status {
        AutonomousStatus::Running => ("bg-green-100 text-green-700", "animate-pulse"),
        AutonomousStatus::Starting => ("bg-blue-100 text-blue-700", "animate-pulse"),
        AutonomousStatus::Paused => ("bg-yellow-100 text-yellow-700", ""),
        AutonomousStatus::Completed => ("bg-green-100 text-green-700", ""),
        AutonomousStatus::Terminated => ("bg-slate-100 text-slate-600", ""),
        AutonomousStatus::Failed => ("bg-red-100 text-red-700", ""),
        _ => ("bg-slate-100 text-slate-600", ""),
    };

    let status_text = match session.status {
        AutonomousStatus::Running => "运行中",
        AutonomousStatus::Starting => "启动中",
        AutonomousStatus::Paused => "已暂停",
        AutonomousStatus::Completed => "已完成",
        AutonomousStatus::Terminated => "已终止",
        AutonomousStatus::Failed => "失败",
        _ => "未知",
    };

    let is_active = session.status == AutonomousStatus::Running
        || session.status == AutonomousStatus::Starting;

    let orch_id = session.orchestration_id.clone();
    let orch_id_display = if orch_id.len() > 20 {
        format!("{}...", &orch_id[..20])
    } else {
        orch_id.clone()
    };

    let created_time = format_timestamp(session.created_at_ms);
    let progress = if session.max_cycles > 0 {
        (session.cycles_executed as f64 / session.max_cycles as f64 * 100.0) as u32
    } else {
        0
    };

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
            // 头部：状态和 ID
            div { class: "flex items-center justify-between mb-3",
                div { class: "flex items-center gap-2",
                    span {
                        class: format!("text-xs px-2 py-1 rounded-full {} {}", status_style.0, status_style.1),
                        "{status_text}"
                    }
                    span { class: "text-xs text-slate-500 font-mono", "{orch_id_display}" }
                }
                if is_active {
                    button {
                        r#type: "button",
                        class: "rounded bg-red-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-red-500 disabled:opacity-50",
                        disabled: terminating,
                        onclick: move |_| {
                            terminate_fn(orch_id.clone(), "用户手动终止".to_string());
                        },
                        {if terminating { "终止中..." } else { "终止" }}
                    }
                }
            }

            // 当前议程
            if let Some(ref agenda) = session.current_agenda {
                div { class: "mb-3 p-2 bg-blue-50 rounded border border-blue-200",
                    p { class: "text-xs text-blue-600 font-medium", "当前任务" }
                    p { class: "text-sm text-blue-800", "{agenda}" }
                }
            }

            // 统计信息
            div { class: "grid grid-cols-5 gap-2 mb-3",
                div { class: "text-center p-2 bg-slate-50 rounded",
                    p { class: "text-sm font-bold text-blue-600", "{session.cycles_executed}/{session.max_cycles}" }
                    p { class: "text-xs text-slate-500", "周期" }
                }
                div { class: "text-center p-2 bg-slate-50 rounded",
                    p { class: "text-sm font-bold text-green-600", "{session.agenda_count}" }
                    p { class: "text-xs text-slate-500", "议程" }
                }
                div { class: "text-center p-2 bg-slate-50 rounded",
                    p { class: "text-sm font-bold text-purple-600", "${session.total_cost:.4}" }
                    p { class: "text-xs text-slate-500", "成本" }
                }
                div { class: "text-center p-2 bg-slate-50 rounded",
                    p { class: "text-sm font-bold text-orange-600", "{session.total_tokens}" }
                    p { class: "text-xs text-slate-500", "Token" }
                }
                div { class: "text-center p-2 bg-slate-50 rounded",
                    p { class: "text-sm font-bold text-amber-600", "{session.idle_count}/{session.max_idle}" }
                    p { class: "text-xs text-slate-500", "空转" }
                }
            }

            // 进度条
            if session.max_cycles > 0 {
                div { class: "w-full bg-slate-200 rounded-full h-1.5 mb-3",
                    div {
                        class: "bg-blue-600 h-1.5 rounded-full transition-all duration-300",
                        style: format!("width: {}%", progress.min(100))
                    }
                }
            }

            // 最近执行日志
            if !session.recent_logs.is_empty() {
                div { class: "mt-3 border-t border-slate-200 pt-3",
                    p { class: "text-xs font-medium text-slate-600 mb-2", "最近执行日志" }
                    div { class: "space-y-1 max-h-32 overflow-y-auto",
                        for log in session.recent_logs.iter() {
                            {render_log_entry(log)}
                        }
                    }
                }
            }

            // 创建时间
            div { class: "mt-2 text-xs text-slate-400 text-right",
                "创建于 {created_time}"
            }
        }
    }
}

/// 渲染执行日志条目
fn render_log_entry(log: &crate::models::ExecutionLogEntry) -> Element {
    let status_color = match log.status.as_str() {
        "success" => "text-green-600",
        "failed" => "text-red-600",
        _ => "text-slate-600",
    };

    let time_str = format_timestamp(log.timestamp_ms);

    rsx! {
        div { class: "flex items-start gap-2 text-xs p-1 bg-slate-50 rounded",
            span { class: format!("font-mono {}", status_color), "#{log.cycle_id}" }
            span { class: "flex-1 text-slate-700 truncate", "{log.message}" }
            span { class: "text-slate-400", "{time_str}" }
        }
    }
}

/// 格式化时间戳
fn format_timestamp(ms: i64) -> String {
    if ms <= 0 {
        return "-".to_string();
    }

    let now = web_sys::js_sys::Date::now() as i64;
    let diff = now - ms;

    if diff < 60_000 {
        "刚刚".to_string()
    } else if diff < 3_600_000 {
        format!("{}分钟前", diff / 60_000)
    } else if diff < 86_400_000 {
        format!("{}小时前", diff / 3_600_000)
    } else {
        format!("{}天前", diff / 86_400_000)
    }
}
