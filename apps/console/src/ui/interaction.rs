#[cfg(target_arch = "wasm32")]
use crate::api::ClientError;
#[cfg(target_arch = "wasm32")]
use crate::hooks::cycle_runner::{extract_budget_hint, extract_indices_from_details};
use crate::hooks::cycle_runner::{use_cycle_runner, CycleTriggerParams};
#[cfg(target_arch = "wasm32")]
use crate::models::CycleSnapshotView;
use crate::state::{
    use_app_actions, use_app_state, AppActions, AuditActionKind, AuditLogEntry, OperationStageKind,
    OperationStageStatus, OperationState,
};
#[cfg(target_arch = "wasm32")]
use crate::{API_CLIENT, APP_CONFIG};
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde::Serialize;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
use soulseed_agi_core_models::{
    AIId, AccessClass, ConversationScenario, DialogueEventType, HumanId, Subject, SubjectRef,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn InteractionPanel() -> Element {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let operation_state = snapshot.operation.clone();
    let cycles_state = snapshot.ace.cycles.clone();
    let selected_cycle_state = snapshot.ace.selected_cycle_id.clone();
    let audit_entries = snapshot.audit.entries.clone();
    drop(snapshot);

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
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let injection_seq = use_signal(|| 1u64);
    let injection_pending = use_signal(|| false);

    let initial_injection_cycle = selected_cycle_state
        .clone()
        .or_else(|| cycles_state.first().map(|cycle| cycle.cycle_id.clone()));
    let injection_cycle = use_signal(move || initial_injection_cycle.clone());
    let injection_priority = use_signal(|| "p1_high".to_string());
    let injection_author_role = use_signal(|| "facilitator".to_string());

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
    let mut injection_cycle_signal = injection_cycle.clone();
    let mut injection_priority_signal = injection_priority.clone();
    let mut injection_author_signal = injection_author_role.clone();

    let scenario_value_str = scenario_value(&scenario_selected);
    let event_type_value_str = event_type_value(&event_type_selected);
    let access_value_str = access_value(access_selected);

    {
        let cycles = cycles_state.clone();
        let selected_cycle = selected_cycle_state.clone();
        let mut cycle_signal = injection_cycle.clone();
        use_effect(move || {
            cycle_signal.with_mut(|current| {
                let current_valid = current
                    .as_ref()
                    .map(|value| cycles.iter().any(|cycle| &cycle.cycle_id == value))
                    .unwrap_or(false);

                let mut fallback = selected_cycle.clone();
                if !fallback
                    .as_ref()
                    .map(|value| cycles.iter().any(|cycle| &cycle.cycle_id == value))
                    .unwrap_or(false)
                {
                    fallback = cycles.first().map(|cycle| cycle.cycle_id.clone());
                }

                let desired = if current_valid {
                    current.clone()
                } else {
                    fallback
                };

                if *current != desired {
                    *current = desired;
                }
            });
        });
    }

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

    #[cfg(target_arch = "wasm32")]
    let on_submit_injection = {
        let actions = actions.clone();
        let app_state = state.clone();
        let injection_input = injection_input.clone();
        let injection_cycle = injection_cycle.clone();
        let injection_priority = injection_priority.clone();
        let injection_author_role = injection_author_role.clone();
        let mut injection_pending = injection_pending.clone();
        let injection_seq = injection_seq.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            let note = injection_input.read().trim().to_string();
            if note.is_empty() {
                actions.set_operation_error("请输入注入说明".to_string());
                return;
            }

            let selected_cycle = injection_cycle.read();
            let Some(cycle_label) = selected_cycle.clone() else {
                actions.set_operation_error("请选择目标周期".to_string());
                return;
            };
            if cycle_label.trim().is_empty() {
                actions.set_operation_error("请选择目标周期".to_string());
                return;
            }

            let cycle_id = match cycle_label.parse::<u64>() {
                Ok(value) => value,
                Err(_) => {
                    actions.set_operation_error("当前周期 ID 无法解析，请确认选择".to_string());
                    return;
                }
            };

            let priority_value = injection_priority.read().clone();
            let author_role_value = injection_author_role.read().clone();
            let seq = *injection_seq.read();
            let context_label = format!("HITL 注入 @ 周期 {cycle_label}");

            actions.set_operation_context(Some(context_label.clone()));
            actions.set_operation_trace(None);
            actions.set_operation_success(format!(
                "正在向周期 {cycle_label} 提交注入（优先级 {priority_value}）"
            ));

            injection_pending.set(true);

            let actions_clone = actions.clone();
            let app_state_clone = app_state.clone();
            let mut injection_input_signal = injection_input.clone();
            let mut injection_seq_signal = injection_seq.clone();
            let mut injection_pending_signal = injection_pending.clone();
            let priority_for_async = priority_value.clone();
            let author_for_async = author_role_value.clone();
            let note_for_async = note.clone();
            let cycle_label_for_async = cycle_label.clone();
            let context_for_async = context_label.clone();

            spawn_local(async move {
                let tenant_id = {
                    let snapshot = app_state_clone.read();
                    snapshot.tenant_id.clone().or_else(|| {
                        APP_CONFIG
                            .get()
                            .and_then(|cfg| cfg.default_tenant_id.clone())
                    })
                };

                let Some(tenant_id) = tenant_id else {
                    actions_clone.set_operation_error("请先选择租户后再提交注入".into());
                    actions_clone.set_operation_context(Some(context_for_async.clone()));
                    injection_pending_signal.set(false);
                    return;
                };

                let Some(client) = API_CLIENT.get().cloned() else {
                    actions_clone.set_operation_error("Thin-Waist 客户端未初始化".into());
                    actions_clone.set_operation_context(Some(context_for_async.clone()));
                    injection_pending_signal.set(false);
                    return;
                };

                actions_clone.operation_stage_reset();
                actions_clone.operation_stage_start(
                    OperationStageKind::HitlSubmit,
                    format!("提交 HITL 注入至周期 {cycle_label_for_async}"),
                );
                actions_clone.set_operation_diagnostics(Vec::new(), None);

                #[derive(Serialize)]
                struct HitlInjectionRequest {
                    cycle_id: u64,
                    priority: String,
                    author_role: String,
                    payload: serde_json::Value,
                }

                let request = HitlInjectionRequest {
                    cycle_id,
                    priority: priority_for_async.clone(),
                    author_role: author_for_async.clone(),
                    payload: json!({
                        "kind": "clarify_override",
                        "note": note_for_async,
                        "sequence": seq,
                        "submitted_via": "soulseed-console"
                    }),
                };

                match client
                    .post_cycle_injection::<_, CycleSnapshotView>(
                        &request,
                        Some(tenant_id.as_str()),
                    )
                    .await
                {
                    Ok(env) => {
                        actions_clone.set_operation_trace(env.trace_id.clone());
                        if let Some(snapshot) = env.data {
                            let outbox = snapshot.outbox.clone();
                            let outcome = snapshot.outcomes.last().cloned();
                            actions_clone.store_ace_snapshot(
                                cycle_label_for_async.clone(),
                                snapshot,
                                outbox,
                            );
                            actions_clone.operation_stage_complete(
                                OperationStageKind::HitlSubmit,
                                Some(format!(
                                    "角色 {author_for_async} · 优先级 {priority_for_async}"
                                )),
                            );
                            actions_clone.set_operation_diagnostics(Vec::new(), None);
                            actions_clone.set_operation_outcome(outcome);
                            actions_clone.set_operation_success(format!(
                                "周期 {cycle_label_for_async} 已接收 HITL 注入（优先级 {priority_for_async}）"
                            ));
                            actions_clone.set_operation_cycle(Some(cycle_label_for_async.clone()));
                            actions_clone.select_ace_cycle(Some(cycle_label_for_async.clone()));
                            actions_clone.set_operation_context(Some(context_for_async.clone()));
                            injection_seq_signal.set(seq + 1);
                            injection_input_signal.set(String::new());
                        } else {
                            actions_clone.operation_stage_fail(
                                OperationStageKind::HitlSubmit,
                                Some("HITL 注入返回空快照".into()),
                            );
                            actions_clone.set_operation_error("HITL 注入返回空快照".into());
                            actions_clone.set_operation_context(Some(context_for_async.clone()));
                        }
                    }
                    Err(err) => {
                        actions_clone.operation_stage_fail(
                            OperationStageKind::HitlSubmit,
                            Some(err.to_string()),
                        );
                        if let Some(details) = err.trace_context() {
                            let indices = extract_indices_from_details(details);
                            let budget = extract_budget_hint(details);
                            actions_clone.set_operation_diagnostics(indices, budget);
                        } else {
                            actions_clone.set_operation_diagnostics(Vec::new(), None);
                        }
                        if let Some(status) = err.status().map(|code| code.as_u16()) {
                            let trace_id = err
                                .trace_context()
                                .and_then(|ctx| ctx.get("trace_id"))
                                .and_then(|value| value.as_str())
                                .map(|value| value.to_string());
                            let error_code = match &err {
                                ClientError::Api(body) => Some(body.code.clone()),
                                _ => None,
                            };
                            actions_clone.record_http_failure(
                                status,
                                trace_id,
                                error_code,
                                context_for_async.clone(),
                                Some(err.to_string()),
                            );
                        } else {
                            actions_clone.set_operation_error(format!("HITL 注入失败: {err}"));
                            actions_clone.set_operation_context(Some(context_for_async.clone()));
                        }
                    }
                }

                injection_pending_signal.set(false);
            });
        }
    };

    #[cfg(not(target_arch = "wasm32"))]
    let on_submit_injection = {
        let actions = actions.clone();
        move |evt: FormEvent| {
            evt.prevent_default();
            actions.set_operation_error("当前运行环境不支持 HITL 注入提交".to_string());
            actions.set_operation_context(Some("HITL 注入".into()));
        }
    };

    let cycle_options: Vec<(String, String)> = cycles_state
        .iter()
        .map(|cycle| {
            (
                cycle.cycle_id.clone(),
                format!("{} · {:?} · {:?}", cycle.cycle_id, cycle.lane, cycle.status),
            )
        })
        .collect();
    let selected_cycle_value = injection_cycle.read();
    let cycle_value_str = selected_cycle_value.clone().unwrap_or_default();
    let priority_value = injection_priority.read().clone();
    let author_role_value = injection_author_role.read().clone();
    let injection_busy = *injection_pending.read();
    let disable_injection = injection_busy || cycle_options.is_empty();

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
                    div { class: "space-y-2 text-xs text-slate-600",
                        label { class: "space-y-1",
                            span { class: "block font-medium", "目标周期" }
                            select {
                                class: "w-full rounded border border-slate-300 p-2 bg-white",
                                value: "{cycle_value_str}",
                                disabled: disable_injection,
                                onchange: move |evt| {
                                    let value = evt.value().trim().to_string();
                                    if value.is_empty() {
                                        injection_cycle_signal.set(None);
                                    } else {
                                        injection_cycle_signal.set(Some(value));
                                    }
                                },
                                if cycle_options.is_empty() {
                                    option { value: "", selected: true, disabled: true, "暂无可用周期" }
                                } else {
                                    option { value: "", selected: cycle_value_str.is_empty(), "请选择周期" }
                                    for (value, label) in cycle_options.iter() {
                                        option {
                                            value: "{value}",
                                            selected: cycle_value_str == *value,
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "grid grid-cols-2 gap-2",
                            label { class: "space-y-1",
                                span { class: "block font-medium", "优先级" }
                                select {
                                    class: "w-full rounded border border-slate-300 p-2 bg-white",
                                    value: "{priority_value}",
                                    disabled: injection_busy,
                                    onchange: move |evt| injection_priority_signal.set(evt.value().to_string()),
                                    for option in PRIORITY_OPTIONS.iter() {
                                        option {
                                            value: option.value,
                                            selected: priority_value == option.value,
                                            "{option.label}"
                                        }
                                    }
                                }
                            }
                            label { class: "space-y-1",
                                span { class: "block font-medium", "作者角色" }
                                select {
                                    class: "w-full rounded border border-slate-300 p-2 bg-white",
                                    value: "{author_role_value}",
                                    disabled: injection_busy,
                                    onchange: move |evt| injection_author_signal.set(evt.value().to_string()),
                                    for option in AUTHOR_ROLE_OPTIONS.iter() {
                                        option {
                                            value: option.value,
                                            selected: author_role_value == option.value,
                                            "{option.label}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    textarea {
                        class: "w-full rounded border border-slate-300 p-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                        rows: "4",
                        placeholder: "请输入注入说明，例如 Clarify 补充信息",
                        value: "{injection_input.read()}",
                        disabled: disable_injection,
                        oninput: move |evt| injection_input.set(evt.value().to_string()),
                    }
                    button {
                        class: "rounded bg-amber-500 px-3 py-2 text-xs font-semibold text-white hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-70",
                        r#type: "submit",
                        disabled: disable_injection,
                        if injection_busy { "提交中…" } else { "提交注入" }
                    }
                }
            }
            AuditLogPanel {
                entries: audit_entries.clone(),
                actions: actions.clone(),
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
    let status = props.status.clone();
    let actions = props.actions.clone();

    let mut stage_order = if !status.stages.is_empty()
        && status
            .stages
            .iter()
            .all(|stage| stage.kind == OperationStageKind::HitlSubmit)
    {
        vec![OperationStageKind::HitlSubmit]
    } else {
        vec![
            OperationStageKind::TriggerSubmit,
            OperationStageKind::StreamAwait,
            OperationStageKind::SnapshotRefresh,
            OperationStageKind::OutboxReady,
        ]
    };

    for stage in status.stages.iter() {
        if !stage_order.contains(&stage.kind) {
            stage_order.push(stage.kind.clone());
        }
    }

    let stage_views: Vec<StageView> = stage_order
        .into_iter()
        .map(|kind| {
            let data = status.stages.iter().find(|stage| stage.kind == kind);
            let label = data
                .and_then(|stage| {
                    let trimmed = stage.label.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .unwrap_or_else(|| default_stage_label(&kind).to_string());
            let detail = data.and_then(|stage| stage.detail.clone());
            let started_at = data.and_then(|stage| stage.started_at.clone());
            let finished_at = data.and_then(|stage| stage.finished_at.clone());
            let duration_ms = data.and_then(|stage| stage.duration_ms);
            let status_value = data
                .map(|stage| stage.status.clone())
                .unwrap_or_else(OperationStageStatus::default);
            let is_current = status
                .current_stage
                .as_ref()
                .map(|current| current == &kind)
                .unwrap_or(false);

            StageView {
                label,
                status: status_value,
                detail,
                started_at,
                finished_at,
                duration_ms,
                is_current,
            }
        })
        .collect();

    let message_block = if let Some(ref err) = status.error {
        Some(rsx! {
            div { class: "space-y-1 rounded border border-red-200 bg-red-50 p-3 text-xs text-red-700",
                span { class: "font-semibold", "上次操作失败" }
                p { class: "text-red-700", "{err}" }
            }
        })
    } else if let Some(ref msg) = status.last_message {
        Some(rsx! {
            div { class: "space-y-1 rounded border border-emerald-200 bg-emerald-50 p-3 text-xs text-emerald-700",
                span { class: "font-semibold", "上次操作成功" }
                p { class: "text-emerald-700", "{msg}" }
            }
        })
    } else {
        None
    };

    let mut metadata_rows: Vec<(&'static str, String)> = Vec::new();
    if let Some(ref context) = status.context {
        metadata_rows.push(("上下文", context.clone()));
    }
    if let Some(triggered) = status.triggered_at.clone() {
        metadata_rows.push(("触发时间", triggered));
    }
    if let Some(cycle) = status.last_cycle_id.clone() {
        metadata_rows.push(("周期 ID", cycle));
    }
    if let Some(outcome) = status.last_outcome.as_ref() {
        let outcome_label = outcome
            .manifest_digest
            .clone()
            .unwrap_or_else(|| outcome.status.clone());
        metadata_rows.push((
            "最新 Outcome",
            format!("#{} {outcome_label}", outcome.cycle_id),
        ));
    }
    if let Some(total) = status.total_elapsed_ms {
        metadata_rows.push(("累计耗时", humanize_duration(total)));
    }

    let mut observability_rows: Vec<(&'static str, String)> = Vec::new();
    if let Some(code) = status.last_status {
        observability_rows.push(("HTTP 状态", code.to_string()));
    }
    if let Some(ref err_code) = status.error_code {
        observability_rows.push(("错误代码", err_code.clone()));
    }
    if let Some(ref trace) = status.trace_id {
        observability_rows.push(("trace_id", trace.clone()));
    }
    if let Some(ref budget) = status.last_budget {
        observability_rows.push(("预算提示", budget.clone()));
    }

    let has_indices = !status.last_indices_used.is_empty();
    let has_observability = has_indices || !observability_rows.is_empty();

    let actions_clear = actions.clone();

    rsx! {
        div { class: "rounded-lg border border-slate-200 bg-white p-4 shadow-sm space-y-3 text-xs text-slate-600",
            header { class: "flex flex-wrap items-start justify-between gap-2",
                div { class: "space-y-1",
                    h3 { class: "text-sm font-semibold text-slate-900", "操作状态" }
                    if metadata_rows.iter().any(|(label, _)| *label == "上下文") {
                        if let Some((_, context)) = metadata_rows.iter().find(|(label, _)| *label == "上下文") {
                            p { class: "text-[11px] text-slate-500", "{context}" }
                        }
                    }
                }
                button {
                    class: "rounded border border-slate-300 px-3 py-1 text-[11px] text-slate-600 hover:bg-slate-100",
                    onclick: move |_| actions_clear.clone().clear_operation_status(),
                    "清除记录"
                }
            }

            if let Some(block) = message_block {
                {block}
            } else {
                p { class: "text-[11px] text-slate-500 italic", "尚未执行操作" }
            }

            if !stage_views.is_empty() {
                div { class: "space-y-2",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "阶段进度" }
                    div { class: "grid gap-2 md:grid-cols-2",
                        for view in stage_views.iter() {
                            { render_stage_view(view) }
                        }
                    }
                }
            }

            if has_observability {
                div { class: "space-y-2",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "诊断信息" }
                    div { class: "rounded border border-slate-200 bg-slate-50 p-3 space-y-2",
                        for (label, value) in observability_rows.iter() {
                            p { class: "flex flex-wrap items-center gap-2 text-[11px] text-slate-600",
                                span { class: "font-semibold text-slate-700", "{label}:" }
                                span { class: "font-mono break-all", "{value}" }
                            }
                        }
                        if has_indices {
                            div { class: "space-y-1",
                                span { class: "text-[11px] font-semibold text-slate-700", "命中索引" }
                                div { class: "flex flex-wrap gap-1",
                                    for idx in status.last_indices_used.iter() {
                                        span { class: "rounded bg-slate-200 px-2 py-0.5 text-[11px] text-slate-700", "{idx}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !metadata_rows.is_empty() {
                div { class: "space-y-2",
                    span { class: "text-[11px] font-semibold uppercase tracking-wide text-slate-500", "上下文信息" }
                    ul { class: "space-y-1 text-[11px] text-slate-600",
                        for (label, value) in metadata_rows.into_iter().filter(|(label, _)| *label != "上下文") {
                            li { "{label}: {value}" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
struct StageView {
    label: String,
    status: OperationStageStatus,
    detail: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    duration_ms: Option<u64>,
    is_current: bool,
}

fn render_stage_view(stage: &StageView) -> Element {
    let badge_class = stage_badge_class(&stage.status);
    let badge_label = stage_status_text(&stage.status);
    let container_class = stage_container_class(&stage.status, stage.is_current);

    let timing = match (&stage.started_at, &stage.finished_at, stage.duration_ms) {
        (Some(start), Some(end), Some(duration)) => Some(format!(
            "{} → {} · {}",
            start,
            end,
            humanize_duration(duration)
        )),
        (Some(start), None, _) => Some(format!("开始于 {start}")),
        (None, Some(end), _) => Some(format!("结束于 {end}")),
        (_, _, Some(duration)) => Some(humanize_duration(duration)),
        _ => None,
    };

    let detail = stage.detail.clone();

    rsx! {
        div { class: container_class,
            div { class: "flex items-center justify-between gap-2",
                span { class: "text-[12px] font-semibold text-slate-800", "{stage.label}" }
                span { class: format!("rounded px-2 py-0.5 text-[11px] font-medium {}", badge_class), "{badge_label}" }
            }
            if let Some(detail_text) = detail {
                p { class: "mt-1 text-[11px] text-slate-600", "{detail_text}" }
            }
            if let Some(timing_label) = timing {
                p { class: "mt-1 text-[10px] font-mono text-slate-500 break-all", "{timing_label}" }
            }
        }
    }
}

fn default_stage_label(kind: &OperationStageKind) -> &'static str {
    match kind {
        OperationStageKind::TriggerSubmit => "提交触发",
        OperationStageKind::StreamAwait => "SSE 等待",
        OperationStageKind::SnapshotRefresh => "状态刷新",
        OperationStageKind::OutboxReady => "Outbox 拉取",
        OperationStageKind::HitlSubmit => "HITL 注入",
        OperationStageKind::ContextSync => "上下文同步",
        OperationStageKind::Unknown => "其他阶段",
    }
}

fn stage_status_text(status: &OperationStageStatus) -> &'static str {
    match status {
        OperationStageStatus::Pending => "待开始",
        OperationStageStatus::Running => "进行中",
        OperationStageStatus::Completed => "已完成",
        OperationStageStatus::Failed => "已失败",
    }
}

fn stage_badge_class(status: &OperationStageStatus) -> &'static str {
    match status {
        OperationStageStatus::Pending => "bg-slate-200 text-slate-700",
        OperationStageStatus::Running => "bg-amber-200 text-amber-800",
        OperationStageStatus::Completed => "bg-emerald-200 text-emerald-700",
        OperationStageStatus::Failed => "bg-red-200 text-red-700",
    }
}

fn stage_container_class(status: &OperationStageStatus, is_current: bool) -> String {
    let base = match status {
        OperationStageStatus::Pending => "border-slate-200 bg-white",
        OperationStageStatus::Running => "border-amber-200 bg-amber-50",
        OperationStageStatus::Completed => "border-emerald-200 bg-emerald-50",
        OperationStageStatus::Failed => "border-red-200 bg-red-50",
    };

    let highlight = if is_current {
        " ring-1 ring-offset-1 ring-amber-200"
    } else {
        ""
    };

    format!(
        "rounded border px-3 py-2 shadow-sm text-[11px] text-slate-600{} {}",
        highlight, base
    )
}
#[derive(Props, Clone)]
#[props(no_eq)]
struct AuditLogPanelProps {
    entries: Vec<AuditLogEntry>,
    actions: AppActions,
}

impl PartialEq for AuditLogPanelProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for AuditLogPanelProps {}

#[component]
fn AuditLogPanel(props: AuditLogPanelProps) -> Element {
    let total = props.entries.len();
    let actions = props.actions.clone();

    rsx! {
        section { class: "space-y-3",
            header { class: "flex flex-wrap items-center justify-between gap-2",
                div { class: "space-y-1",
                    h3 { class: "text-sm font-semibold text-slate-900", "操作审计" }
                    span { class: "text-[11px] text-slate-500", "共 {total} 条记录" }
                }
                button {
                    class: "rounded border border-slate-300 px-3 py-1 text-[11px] text-slate-600 hover:bg-slate-100",
                    onclick: move |_| actions.clone().clear_audit_logs(),
                    "清空"
                }
            }
            if props.entries.is_empty() {
                p { class: "text-xs text-slate-500 italic", "暂无复制 / 导出审计记录" }
            } else {
                ul { class: "space-y-2",
                    for entry in props.entries.iter().take(20) {
                        li { class: "rounded border border-slate-200 bg-white p-3 shadow-sm text-xs text-slate-600 space-y-1",
                            div { class: "flex flex-wrap items-center justify-between gap-2",
                                div { class: "flex items-center gap-2",
                                    span { class: format!("rounded px-2 py-0.5 text-[11px] font-medium {}", audit_action_badge_class(&entry.action)), "{audit_action_badge_label(&entry.action)}" }
                                    span { class: "font-semibold text-slate-800", "{entry.label}" }
                                }
                                span { class: "text-[11px] text-slate-500", "#{entry.id} · {entry.timestamp}" }
                            }
                            if let Some(tenant) = entry.tenant_id.as_ref() {
                                p { class: "text-[11px] text-slate-500", "Tenant: {tenant}" }
                            }
                            if let Some(session) = entry.session_id.as_ref() {
                                p { class: "text-[11px] text-slate-500", "Session: {session}" }
                            }
                            p { class: "font-mono text-[11px] text-slate-500 break-all", "Target: {entry.target}" }
                        }
                    }
                }
            }
        }
    }
}

fn audit_action_badge_class(action: &AuditActionKind) -> &'static str {
    match action {
        AuditActionKind::Copy => "bg-amber-100 text-amber-800",
        AuditActionKind::Export => "bg-indigo-100 text-indigo-700",
    }
}

fn audit_action_badge_label(action: &AuditActionKind) -> &'static str {
    match action {
        AuditActionKind::Copy => "复制",
        AuditActionKind::Export => "导出",
    }
}

fn humanize_duration(duration_ms: u64) -> String {
    if duration_ms >= 60_000 {
        let minutes = duration_ms as f64 / 60_000.0;
        format!("{minutes:.1} min")
    } else if duration_ms >= 1_000 {
        let seconds = duration_ms as f64 / 1_000.0;
        format!("{seconds:.1} s")
    } else {
        format!("{duration_ms} ms")
    }
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
struct PriorityOption {
    value: &'static str,
    label: &'static str,
}

const PRIORITY_OPTIONS: &[PriorityOption] = &[
    PriorityOption {
        value: "p0_critical",
        label: "P0 致命",
    },
    PriorityOption {
        value: "p1_high",
        label: "P1 高优先级",
    },
    PriorityOption {
        value: "p2_medium",
        label: "P2 中等",
    },
    PriorityOption {
        value: "p3_low",
        label: "P3 低优先级",
    },
];

#[derive(Clone, Copy)]
struct AuthorRoleOption {
    value: &'static str,
    label: &'static str,
}

const AUTHOR_ROLE_OPTIONS: &[AuthorRoleOption] = &[
    AuthorRoleOption {
        value: "system",
        label: "System 管理员",
    },
    AuthorRoleOption {
        value: "facilitator",
        label: "Facilitator 协同者",
    },
    AuthorRoleOption {
        value: "owner",
        label: "Owner 拥有者",
    },
    AuthorRoleOption {
        value: "participant",
        label: "Participant 参与者",
    },
    AuthorRoleOption {
        value: "guest",
        label: "Guest 访客",
    },
];

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
