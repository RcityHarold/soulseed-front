use crate::fixtures::timeline::make_injection_metadata;
use crate::hooks::cycle_runner::{use_cycle_runner, CycleTriggerParams};
use crate::state::{use_app_actions, use_app_state, AppActions, OperationState};
use dioxus::prelude::*;
use soulseed_agi_core_models::{
    AIId, AccessClass, ConversationScenario, DialogueEventType, HumanId, Subject, SubjectRef,
};

#[component]
pub fn InteractionPanel() -> Element {
    let actions = use_app_actions();
    let state = use_app_state();
    let operation_state = state.read().operation.clone();

    let cycle_runner = use_cycle_runner();

    let mut message_input = use_signal(|| String::new());
    let mut message_seq = use_signal(|| 1u64);
    let scenario_select = use_signal(|| ConversationScenario::HumanToAi);
    let event_type_select = use_signal(|| DialogueEventType::Message);
    let subject_role = use_signal(|| SubjectRole::Human);
    let subject_id = use_signal(|| "42".to_string());
    let participant_role = use_signal(|| SubjectRole::AI);
    let participant_id = use_signal(|| "7".to_string());
    let participant_label = use_signal(|| "assistant".to_string());
    let channel_input = use_signal(|| "dialogue".to_string());
    let access_class = use_signal(|| AccessClass::Internal);

    let mut injection_input = use_signal(|| String::new());
    let mut injection_seq = use_signal(|| 1u64);

    let scenario_selected = scenario_select.read().clone();
    let event_type_selected = event_type_select.read().clone();
    let access_selected = access_class.read().clone();
    let subject_role_selected = *subject_role.read();
    let participant_role_selected = *participant_role.read();
    let subject_id_value = subject_id.read().clone();
    let participant_id_value = participant_id.read().clone();
    let participant_label_value = participant_label.read().clone();
    let channel_value = channel_input.read().clone();
    let message_value = message_input.read().clone();
    let runner_is_running = *cycle_runner.is_running.read();

    let mut scenario_select_signal = scenario_select.clone();
    let mut event_type_select_signal = event_type_select.clone();
    let mut subject_role_signal = subject_role.clone();
    let mut subject_id_signal = subject_id.clone();
    let mut participant_role_signal = participant_role.clone();
    let mut participant_id_signal = participant_id.clone();
    let mut participant_label_signal = participant_label.clone();
    let mut channel_signal = channel_input.clone();
    let mut access_class_signal = access_class.clone();

    let scenario_value_str = scenario_value(&scenario_selected);
    let event_type_value_str = event_type_value(&event_type_selected);
    let access_value_str = access_value(access_selected);

    let on_submit_message = {
        let actions = actions.clone();
        let runner = cycle_runner.clone();
        let scenario_select = scenario_select.clone();
        let event_type_select = event_type_select.clone();
        let subject_role = subject_role.clone();
        let subject_id = subject_id.clone();
        let participant_role = participant_role.clone();
        let participant_id = participant_id.clone();
        let participant_label = participant_label.clone();
        let channel_input = channel_input.clone();
        let access_class = access_class.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            let text = message_input.read().trim().to_string();
            if text.is_empty() {
                actions.set_operation_error("请输入对话内容".to_string());
                return;
            }

            let event_type = event_type_select.read().clone();
            if event_type != DialogueEventType::Message {
                actions.set_operation_error("目前仅支持提交消息类型事件".to_string());
                return;
            }

            let seq = *message_seq.read();
            let scenario = scenario_select.read().clone();
            let access = *access_class.read();

            let subject = match build_subject(*subject_role.read(), &subject_id.read()) {
                Ok(subject) => subject,
                Err(err) => {
                    actions.set_operation_error(err);
                    return;
                }
            };

            let participant = match build_subject(*participant_role.read(), &participant_id.read())
            {
                Ok(subject) => SubjectRef {
                    kind: subject,
                    role: match participant_label.read().trim() {
                        "" => None,
                        value => Some(value.to_string()),
                    },
                },
                Err(err) => {
                    actions.set_operation_error(err);
                    return;
                }
            };

            runner.trigger_cycle(CycleTriggerParams {
                scenario,
                subject,
                participants: vec![participant],
                text: text.clone(),
                sequence_number: seq,
                channel: match channel_input.read().trim() {
                    "" => None,
                    value => Some(value.to_string()),
                },
                access_class: access,
            });

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
                    div { class: "grid grid-cols-2 gap-2 text-xs text-slate-600",
                        label { class: "space-y-1",
                            span { class: "block font-medium", "对话场景" }
                            select {
                                class: "w-full rounded border border-slate-300 p-2 bg-white",
                                value: scenario_value_str,
                                onchange: move |evt| {
                                    if let Some(next) = parse_scenario(evt.value().as_str()) {
                                        scenario_select_signal.set(next);
                                    }
                                },
                                for option in SCENARIO_OPTIONS.iter() {
                                    option {
                                        value: option.value,
                                        selected: scenario_value_str == option.value,
                                        "{option.label}"
                                    }
                                }
                            }
                        }
                        label { class: "space-y-1",
                            span { class: "block font-medium", "事件类型" }
                            select {
                                class: "w-full rounded border border-slate-300 p-2 bg-white",
                                value: event_type_value_str,
                                onchange: move |evt| {
                                    if let Some(next) = parse_event_type(evt.value().as_str()) {
                                        event_type_select_signal.set(next);
                                    }
                                },
                                option { value: "message", selected: event_type_value_str == "message", "消息" }
                                option { value: "tool_call", disabled: true, "工具调用 (暂不支持)" }
                                option { value: "tool_result", disabled: true, "工具结果 (暂不支持)" }
                                option { value: "self_reflection", disabled: true, "自反 (暂不支持)" }
                            }
                        }
                    }
                    div { class: "grid grid-cols-2 gap-2 text-xs text-slate-600",
                        label { class: "space-y-1",
                            span { class: "block font-medium", "主体角色" }
                            select {
                                class: "w-full rounded border border-slate-300 p-2 bg-white",
                                value: subject_role_selected.as_str(),
                                onchange: move |evt| {
                                    if let Some(role) = SubjectRole::from_str(evt.value().as_str()) {
                                        subject_role_signal.set(role);
                                    }
                                },
                                option { value: "human", selected: matches!(subject_role_selected, SubjectRole::Human), "Human" }
                                option { value: "ai", selected: matches!(subject_role_selected, SubjectRole::AI), "AI" }
                            }
                        }
                        label { class: "space-y-1",
                            span { class: "block font-medium", "主体 ID" }
                            input {
                                class: "w-full rounded border border-slate-300 p-2",
                                r#type: "text",
                                value: "{subject_id_value}",
                                oninput: move |evt| subject_id_signal.set(evt.value().to_string()),
                            }
                        }
                    }
                    div { class: "grid grid-cols-3 gap-2 text-xs text-slate-600",
                        label { class: "space-y-1",
                            span { class: "block font-medium", "参与者角色" }
                            select {
                                class: "w-full rounded border border-slate-300 p-2 bg-white",
                                value: participant_role_selected.as_str(),
                                onchange: move |evt| {
                                    if let Some(role) = SubjectRole::from_str(evt.value().as_str()) {
                                        participant_role_signal.set(role);
                                    }
                                },
                                option { value: "human", selected: matches!(participant_role_selected, SubjectRole::Human), "Human" }
                                option { value: "ai", selected: matches!(participant_role_selected, SubjectRole::AI), "AI" }
                            }
                        }
                        label { class: "space-y-1",
                            span { class: "block font-medium", "参与者 ID" }
                            input {
                                class: "w-full rounded border border-slate-300 p-2",
                                value: "{participant_id_value}",
                                oninput: move |evt| participant_id_signal.set(evt.value().to_string()),
                            }
                        }
                        label { class: "space-y-1",
                            span { class: "block font-medium", "参与者标签" }
                            input {
                                class: "w-full rounded border border-slate-300 p-2",
                                placeholder: "assistant / user",
                                value: "{participant_label_value}",
                                oninput: move |evt| participant_label_signal.set(evt.value().to_string()),
                            }
                        }
                    }
                    div { class: "grid grid-cols-2 gap-2 text-xs text-slate-600",
                        label { class: "space-y-1",
                            span { class: "block font-medium", "信道" }
                            input {
                                class: "w-full rounded border border-slate-300 p-2",
                                value: "{channel_value}",
                                oninput: move |evt| channel_signal.set(evt.value().to_string()),
                            }
                        }
                        label { class: "space-y-1",
                            span { class: "block font-medium", "访问级别" }
                            select {
                                class: "w-full rounded border border-slate-300 p-2 bg-white",
                                value: access_value_str,
                                onchange: move |evt| {
                                    if let Some(access) = parse_access_class(evt.value().as_str()) {
                                        access_class_signal.set(access);
                                    }
                                },
                                option { value: "public", selected: matches!(access_selected, AccessClass::Public), "Public" }
                                option { value: "internal", selected: matches!(access_selected, AccessClass::Internal), "Internal" }
                                option { value: "restricted", selected: matches!(access_selected, AccessClass::Restricted), "Restricted" }
                            }
                        }
                    }
                    textarea {
                        class: "w-full rounded border border-slate-300 p-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                        rows: "4",
                        placeholder: "请输入对话内容，例如 Clarify 问题或 AI 回复",
                        value: "{message_value}",
                        oninput: move |evt| message_input.set(evt.value().to_string()),
                    }
                    button {
                        class: "rounded bg-slate-900 px-3 py-2 text-xs font-semibold text-white hover:bg-slate-800",
                        r#type: "submit",
                        disabled: runner_is_running,
                        if runner_is_running { "提交中…" } else { "提交对话" }
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
    status: OperationState,
    actions: AppActions,
}

impl PartialEq for OperationStatusProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for OperationStatusProps {}

#[component]
fn OperationStatus(props: OperationStatusProps) -> Element {
    if let Some(ref err) = props.status.error {
        let context_label = props.status.context.clone();
        let status_detail = props
            .status
            .last_status
            .map(|code| format!("HTTP 状态: {code}"));
        let code_detail = props
            .status
            .error_code
            .as_ref()
            .map(|code| format!("错误代码: {code}"));
        let trace_detail = props.status.trace_id.clone();
        let trigger_detail = props
            .status
            .triggered_at
            .as_ref()
            .map(|ts| format!("触发时间: {ts}"));
        let cycle_detail = props
            .status
            .last_cycle_id
            .as_ref()
            .map(|id| format!("周期 ID: {id}"));
        let outcome_detail = props.status.last_outcome.as_ref().map(|outcome| {
            format!(
                "Outcome: #{} {}",
                outcome.cycle_id,
                outcome
                    .manifest_digest
                    .clone()
                    .unwrap_or_else(|| outcome.status.clone())
            )
        });
        let actions = props.actions.clone();

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
                if let Some(detail) = code_detail {
                    p { class: "font-mono", "{detail}" }
                }
                if let Some(trace) = trace_detail {
                    p { class: "font-mono break-all", "trace_id: {trace}" }
                }
                if let Some(detail) = trigger_detail {
                    p { class: "font-mono", "{detail}" }
                }
                if let Some(detail) = cycle_detail {
                    p { class: "font-mono", "{detail}" }
                }
                if let Some(detail) = outcome_detail {
                    p { class: "font-mono", "{detail}" }
                }
            }
        };
    }

    if let Some(ref msg) = props.status.last_message {
        let context_label = props.status.context.clone();
        let trace_detail = props.status.trace_id.clone();
        let trigger_detail = props
            .status
            .triggered_at
            .as_ref()
            .map(|ts| format!("触发时间: {ts}"));
        let cycle_detail = props
            .status
            .last_cycle_id
            .as_ref()
            .map(|id| format!("周期 ID: {id}"));
        let outcome_detail = props.status.last_outcome.as_ref().map(|outcome| {
            format!(
                "Outcome: #{} {}",
                outcome.cycle_id,
                outcome
                    .manifest_digest
                    .clone()
                    .unwrap_or_else(|| outcome.status.clone())
            )
        });
        let actions = props.actions.clone();
        return rsx! {
            div { class: "flex items-start justify-between gap-2 rounded border border-emerald-200 bg-emerald-50 p-3 text-xs text-emerald-700",
                div { class: "space-y-1",
                    span { class: "font-semibold", "上次操作成功" }
                    if let Some(ctx) = context_label.as_ref() {
                        p { class: "text-[11px] text-emerald-600", "上下文: {ctx}" }
                    }
                    p { class: "text-emerald-700", "{msg}" }
                    if let Some(trace) = trace_detail {
                        p { class: "font-mono break-all text-emerald-600", "trace_id: {trace}" }
                    }
                    if let Some(detail) = trigger_detail {
                        p { class: "font-mono text-emerald-600", "{detail}" }
                    }
                    if let Some(detail) = cycle_detail {
                        p { class: "font-mono text-emerald-600", "{detail}" }
                    }
                    if let Some(detail) = outcome_detail {
                        p { class: "font-mono text-emerald-600", "{detail}" }
                    }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubjectRole {
    Human,
    AI,
}

impl SubjectRole {
    fn as_str(self) -> &'static str {
        match self {
            SubjectRole::Human => "human",
            SubjectRole::AI => "ai",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "human" => Some(SubjectRole::Human),
            "ai" => Some(SubjectRole::AI),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct ScenarioOption {
    value: &'static str,
    label: &'static str,
}

const SCENARIO_OPTIONS: &[ScenarioOption] = &[
    ScenarioOption {
        value: "human_to_human",
        label: "人类 ↔ 人类",
    },
    ScenarioOption {
        value: "human_group",
        label: "人类群组",
    },
    ScenarioOption {
        value: "human_to_ai",
        label: "人类 ↔ AI",
    },
    ScenarioOption {
        value: "ai_to_ai",
        label: "AI ↔ AI",
    },
    ScenarioOption {
        value: "ai_self_talk",
        label: "AI 自对话",
    },
    ScenarioOption {
        value: "human_to_multi_ai",
        label: "人类 ↔ 多 AI",
    },
    ScenarioOption {
        value: "multi_human_to_multi_ai",
        label: "多人 ↔ 多 AI",
    },
    ScenarioOption {
        value: "ai_group",
        label: "AI 群组",
    },
    ScenarioOption {
        value: "ai_to_system",
        label: "AI ↔ 系统",
    },
];

fn parse_scenario(value: &str) -> Option<ConversationScenario> {
    match value {
        "human_to_human" => Some(ConversationScenario::HumanToHuman),
        "human_group" => Some(ConversationScenario::HumanGroup),
        "human_to_ai" => Some(ConversationScenario::HumanToAi),
        "ai_to_ai" => Some(ConversationScenario::AiToAi),
        "ai_self_talk" => Some(ConversationScenario::AiSelfTalk),
        "human_to_multi_ai" => Some(ConversationScenario::HumanToMultiAi),
        "multi_human_to_multi_ai" => Some(ConversationScenario::MultiHumanToMultiAi),
        "ai_group" => Some(ConversationScenario::AiGroup),
        "ai_to_system" => Some(ConversationScenario::AiToSystem),
        _ => None,
    }
}

fn scenario_value(value: &ConversationScenario) -> &'static str {
    match value {
        ConversationScenario::HumanToHuman => "human_to_human",
        ConversationScenario::HumanGroup => "human_group",
        ConversationScenario::HumanToAi => "human_to_ai",
        ConversationScenario::AiToAi => "ai_to_ai",
        ConversationScenario::AiSelfTalk => "ai_self_talk",
        ConversationScenario::HumanToMultiAi => "human_to_multi_ai",
        ConversationScenario::MultiHumanToMultiAi => "multi_human_to_multi_ai",
        ConversationScenario::AiGroup => "ai_group",
        ConversationScenario::AiToSystem => "ai_to_system",
    }
}

fn parse_event_type(value: &str) -> Option<DialogueEventType> {
    match value {
        "message" => Some(DialogueEventType::Message),
        _ => None,
    }
}

fn event_type_value(value: &DialogueEventType) -> &'static str {
    match value {
        DialogueEventType::Message => "message",
        DialogueEventType::ToolCall => "tool_call",
        DialogueEventType::ToolResult => "tool_result",
        DialogueEventType::SelfReflection => "self_reflection",
        DialogueEventType::Decision => "decision",
        DialogueEventType::Lifecycle => "lifecycle",
        DialogueEventType::System => "system",
    }
}

fn parse_access_class(value: &str) -> Option<AccessClass> {
    match value {
        "public" => Some(AccessClass::Public),
        "internal" => Some(AccessClass::Internal),
        "restricted" => Some(AccessClass::Restricted),
        _ => None,
    }
}

fn access_value(access: AccessClass) -> &'static str {
    match access {
        AccessClass::Public => "public",
        AccessClass::Internal => "internal",
        AccessClass::Restricted => "restricted",
    }
}

fn build_subject(role: SubjectRole, id_raw: &str) -> Result<Subject, String> {
    let trimmed = id_raw.trim();
    if trimmed.is_empty() {
        return Err("请填写 ID".into());
    }
    let parsed = trimmed
        .parse::<u64>()
        .map_err(|_| "ID 必须为正整数".to_string())?;
    match role {
        SubjectRole::Human => Ok(Subject::Human(HumanId::new(parsed))),
        SubjectRole::AI => Ok(Subject::AI(AIId::new(parsed))),
    }
}
