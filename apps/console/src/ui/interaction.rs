use crate::fixtures::timeline::{make_dialogue_event_from_text, make_injection_metadata};
use crate::state::{use_app_actions, use_app_state, AppActions};
use dioxus::prelude::*;

pub fn InteractionPanel(cx: Scope) -> Element {
    let actions = use_app_actions(cx);
    let app_state = use_app_state(cx);
    let operation_state = app_state.read().operation.clone();
    drop(app_state);

    let message_input = use_signal(cx, || String::new());
    let message_seq = use_signal(cx, || 1u64);

    let injection_input = use_signal(cx, || String::new());
    let injection_seq = use_signal(cx, || 1u64);

    let on_submit_message = {
        let actions = actions.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            let text = message_input.read().trim().to_string();
            if text.is_empty() {
                actions.set_operation_error("请输入对话内容".to_string());
                return;
            }

            let seq = *message_seq.read();
            let event = make_dialogue_event_from_text(seq, &text);
            actions.append_timeline(vec![event], Vec::new(), None);
            actions.set_operation_success(format!("已提交对话事件 #{seq}"));
            message_seq.set(seq + 1);
            message_input.set(String::new());
        }
    };

    let on_submit_injection = {
        let actions = actions.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            let note = injection_input.read().trim().to_string();
            if note.is_empty() {
                actions.set_operation_error("请输入注入说明".to_string());
                return;
            }

            let seq = *injection_seq.read();
            let metadata = make_injection_metadata(&note);
            actions.update_cycle_metadata(None, metadata);
            actions.set_operation_success(format!("已提交 HITL 注入 #{seq}"));
            injection_seq.set(seq + 1);
            injection_input.set(String::new());
        }
    };

    rsx! {
        section { class: "space-y-4",
            header { class: "flex flex-col gap-1",
                h2 { class: "text-lg font-semibold text-slate-900", "人机交互入口" }
                p { class: "text-xs text-slate-500", "快速模拟对话事件与 HITL 注入，验证前后端流程。" }
                OperationStatus { status: operation_state, actions: actions.clone() }
            }

            div { class: "grid gap-4 md:grid-cols-2",
                form { class: "space-y-3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                    onsubmit: on_submit_message,
                    h3 { class: "text-sm font-semibold text-slate-800", "写入对话事件" }
                    textarea {
                        class: "w-full rounded border border-slate-300 p-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                        rows: "4",
                        placeholder: "请输入对话内容，例如 Clarify 问题或 AI 回复",
                        value: "{message_input.read()}",
                        oninput: move |evt| message_input.set(evt.value().to_string()),
                    }
                    button {
                        class: "rounded bg-slate-900 px-3 py-2 text-xs font-semibold text-white hover:bg-slate-800",
                        r#type: "submit",
                        "提交对话"
                    }
                }

                form { class: "space-y-3 rounded-lg border border-slate-200 bg-white p-4 shadow-sm",
                    onsubmit: on_submit_injection,
                    h3 { class: "text-sm font-semibold text-slate-800", "提交 HITL 注入" }
                    textarea {
                        class: "w-full rounded border border-slate-300 p-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                        rows: "4",
                        placeholder: "请输入注入说明，例如 Clarify 补充信息",
                        value: "{injection_input.read()}",
                        oninput: move |evt| injection_input.set(evt.value().to_string()),
                    }
                    button {
                        class: "rounded bg-amber-500 px-3 py-2 text-xs font-semibold text-white hover:bg-amber-400",
                        r#type: "submit",
                        "提交注入"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone)]
#[props(no_eq)]
struct OperationStatusProps {
    status: crate::state::OperationState,
    actions: AppActions,
}

fn OperationStatus(cx: Scope<OperationStatusProps>) -> Element {
    let status = &cx.props.status;

    if let Some(ref err) = status.error {
        let context_label = status.context.clone();
        let status_detail = status.last_status.map(|code| format!("HTTP 状态: {code}"));
        let trace_detail = status.last_trace_id.clone();
        let actions = cx.props.actions.clone();

        return rsx! {
            div { class: "space-y-1 rounded border border-red-200 bg-red-50 p-3 text-xs text-red-700",
                div { class: "flex items-start justify-between gap-2",
                    span { class: "font-semibold", "上次操作失败" }
                    button {
                        class: "rounded bg-red-100 px-2 py-1 text-[11px] text-red-700 transition hover:bg-red-200",
                        onclick: move |_| actions.clone().clear_operation_status(),
                        "清除"
                    }
                }
                if let Some(ctx) = context_label.as_ref() {
                    p { class: "text-[11px] text-red-600", "上下文: {ctx}" }
                }
                p { class: "text-red-700", "{err}" }
                if let Some(detail) = status_detail {
                    p { class: "font-mono", "{detail}" }
                }
                if let Some(trace) = trace_detail {
                    p { class: "font-mono break-all", "trace_id: {trace}" }
                }
            }
        };
    }

    if let Some(ref msg) = status.last_message {
        let context_label = status.context.clone();
        let actions = cx.props.actions.clone();
        return rsx! {
            div { class: "flex items-start justify-between gap-2 rounded border border-emerald-200 bg-emerald-50 p-3 text-xs text-emerald-700",
                div { class: "space-y-1",
                    span { class: "font-semibold", "上次操作成功" }
                    if let Some(ctx) = context_label.as_ref() {
                        p { class: "text-[11px] text-emerald-600", "上下文: {ctx}" }
                    }
                    p { class: "text-emerald-700", "{msg}" }
                }
                button {
                    class: "rounded bg-emerald-100 px-2 py-1 text-[11px] text-emerald-700 transition hover:bg-emerald-200",
                    onclick: move |_| actions.clone().clear_operation_status(),
                    "清除"
                }
            }
        };
    }

    rsx! { p { class: "text-xs text-slate-500", "尚未执行操作" } }
}
