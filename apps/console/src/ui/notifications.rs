use crate::state::{use_app_actions, use_app_state};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastKind {
    fn accent_classes(self) -> (&'static str, &'static str) {
        match self {
            Self::Success => ("border-emerald-500 bg-emerald-50", "text-emerald-700"),
            Self::Error => ("border-red-500 bg-red-50", "text-red-700"),
            Self::Warning => ("border-amber-500 bg-amber-50", "text-amber-700"),
            Self::Info => ("border-slate-500 bg-slate-50", "text-slate-700"),
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
pub struct ToastProps {
    pub kind: ToastKind,
    pub title: String,
    pub message: String,
    #[props(optional)]
    pub context: Option<String>,
    #[props(default)]
    pub details: Vec<(String, String)>,
    #[props(optional)]
    pub on_close: Option<EventHandler<MouseEvent>>,
}

impl PartialEq for ToastProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for ToastProps {}

#[component]
pub fn Toast(props: ToastProps) -> Element {
    let (container_class, accent_text) = props.kind.accent_classes();

    rsx! {
        div { class: format!("pointer-events-auto rounded-lg border-l-4 p-4 shadow-lg {}", container_class),
            div { class: "flex items-start justify-between gap-4",
                div { class: "space-y-1",
                    h3 { class: format!("text-sm font-semibold {}", accent_text), "{props.title}" }
                    if let Some(ref ctx) = props.context {
                        p { class: "text-[11px] text-slate-500", "{ctx}" }
                    }
                    p { class: "text-xs text-slate-700", "{props.message}" }
                    if !props.details.is_empty() {
                        ul { class: "mt-2 space-y-1 text-[11px] text-slate-500",
                            for (label, value) in props.details.iter() {
                                li {
                                    span { class: "font-medium", "{label}: " }
                                    span { class: "font-mono break-all", "{value}" }
                                }
                            }
                        }
                    }
                }
                if let Some(handler) = props.on_close.as_ref() {
                    button {
                        class: "rounded bg-slate-200 px-2 py-1 text-[11px] text-slate-600 transition hover:bg-slate-300",
                        onclick: handler.clone(),
                        "关闭"
                    }
                }
            }
        }
    }
}

#[component]
pub fn NotificationCenter() -> Element {
    let actions = use_app_actions();
    let snapshot = use_app_state().read().clone();

    let mut toasts: Vec<Element> = Vec::new();

    if let Some(error) = snapshot.operation.error.clone() {
        let mut details = Vec::new();
        if let Some(status) = snapshot.operation.last_status {
            details.push(("HTTP 状态".to_string(), status.to_string()));
        }
        if let Some(code) = snapshot.operation.error_code.clone() {
            details.push(("错误代码".to_string(), code));
        }
        if let Some(trace) = snapshot.operation.trace_id.clone() {
            details.push(("trace_id".to_string(), trace));
        }
        if let Some(triggered) = snapshot.operation.triggered_at.clone() {
            details.push(("触发时间".to_string(), triggered));
        }
        if let Some(cycle_id) = snapshot.operation.last_cycle_id.clone() {
            details.push(("周期 ID".to_string(), cycle_id));
        }
        if let Some(outcome) = snapshot.operation.last_outcome.as_ref() {
            details.push((
                "Outcome".to_string(),
                format!(
                    "#{} {}",
                    outcome.cycle_id,
                    outcome
                        .manifest_digest
                        .clone()
                        .unwrap_or_else(|| outcome.status.clone())
                ),
            ));
        }
        let context_label = snapshot
            .operation
            .context
            .clone()
            .unwrap_or_else(|| "操作失败".to_string());
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "operation-error",
                kind: ToastKind::Error,
                title: context_label,
                message: error,
                details,
                on_close: move |_| app_actions.clone().clear_operation_status(),
            }
        });
    } else if let Some(message) = snapshot.operation.last_message.clone() {
        let mut details = Vec::new();
        if let Some(trace) = snapshot.operation.trace_id.clone() {
            details.push(("trace_id".to_string(), trace));
        }
        if let Some(triggered) = snapshot.operation.triggered_at.clone() {
            details.push(("触发时间".to_string(), triggered));
        }
        if let Some(cycle_id) = snapshot.operation.last_cycle_id.clone() {
            details.push(("周期 ID".to_string(), cycle_id));
        }
        if let Some(outcome) = snapshot.operation.last_outcome.as_ref() {
            details.push((
                "Outcome".to_string(),
                format!(
                    "#{} {}",
                    outcome.cycle_id,
                    outcome
                        .manifest_digest
                        .clone()
                        .unwrap_or_else(|| outcome.status.clone())
                ),
            ));
        }
        let title = snapshot
            .operation
            .context
            .clone()
            .unwrap_or_else(|| "操作成功".to_string());
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "operation-success",
                kind: ToastKind::Success,
                title,
                message,
                details,
                on_close: move |_| app_actions.clone().clear_operation_status(),
            }
        });
    }

    if let Some(error) = snapshot.timeline.error.clone() {
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "timeline-error",
                kind: ToastKind::Error,
                title: "时间线加载失败".to_string(),
                message: error,
                on_close: move |_| app_actions.clone().set_timeline_error(None),
            }
        });
    }

    if let Some(error) = snapshot.context.error.clone() {
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "context-error",
                kind: ToastKind::Error,
                title: "上下文拉取失败".to_string(),
                message: error,
                on_close: move |_| app_actions.clone().set_context_error(None),
            }
        });
    }

    if let Some(error) = snapshot.ace.error.clone() {
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "ace-error",
                kind: ToastKind::Error,
                title: "ACE 周期拉取失败".to_string(),
                message: error,
                on_close: move |_| app_actions.clone().set_ace_error(None),
            }
        });
    }

    if let Some(error) = snapshot.graph.error.clone() {
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "graph-error",
                kind: ToastKind::Error,
                title: "图谱加载失败".to_string(),
                message: error,
                on_close: move |_| app_actions.clone().set_graph_error(None),
            }
        });
    }

    if let Some(error) = snapshot.live_stream.error.clone() {
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "live-error",
                kind: ToastKind::Warning,
                title: "实时流中断".to_string(),
                message: error,
                on_close: move |_| app_actions.clone().set_live_error(None),
            }
        });
    }

    if let Some(error) = snapshot.workspace.error.clone() {
        let app_actions = actions.clone();
        toasts.push(rsx! {
            Toast {
                key: "workspace-error",
                kind: ToastKind::Error,
                title: "工作台初始化失败".to_string(),
                message: error,
                on_close: move |_| app_actions.clone().set_workspace_error(None),
            }
        });
    }

    if toasts.is_empty() {
        return rsx! { Fragment {} };
    }

    rsx! {
        div { class: "pointer-events-none fixed right-4 top-4 z-50 flex w-80 flex-col gap-3",
            for toast in toasts {
                {toast}
            }
        }
    }
}
