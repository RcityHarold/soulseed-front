use dioxus::prelude::*;
use serde_json::Value;
use soulseed_agi_core_models::dialogue_event::DialogueEvent as ThinDialogueEvent;
use soulseed_agi_core_models::{AccessClass, ConversationScenario, Subject, SubjectRef};

use crate::api::ClientError;
#[cfg(target_arch = "wasm32")]
use crate::models::CycleTriggerResponse;
#[cfg(target_arch = "wasm32")]
use crate::models::{
    ContextBundleView, CycleSnapshotView, ExplainIndices, OutboxMessageView, TimelinePayload,
};
use crate::services::dialogue::{build_message_event, MessageEventDraft};
use crate::state::{use_app_actions, use_app_state, AppActions, AppSignal, OperationStageKind};
#[cfg(target_arch = "wasm32")]
use crate::API_CLIENT;
use crate::APP_CONFIG;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
use {
    crate::services::sse::{SseCallbacks, SseClient, SseConnectOptions, SseHandle, SseMessage},
    tracing::warn,
};

#[derive(Clone)]
pub struct CycleTriggerParams {
    pub scenario: ConversationScenario,
    pub subject: Subject,
    pub participants: Vec<SubjectRef>,
    pub text: String,
    pub sequence_number: u64,
    pub channel: Option<String>,
    pub access_class: AccessClass,
}

#[derive(Clone)]
pub struct CycleRunnerHandle {
    actions: AppActions,
    app_state: AppSignal,
    pub is_running: Signal<bool>,
    #[cfg(target_arch = "wasm32")]
    stream_handle: Signal<Option<SseHandle>>,
}

impl CycleRunnerHandle {
    pub fn trigger_cycle(&self, params: CycleTriggerParams) {
        trigger_cycle_impl(
            &self.actions,
            &self.app_state,
            self.is_running.clone(),
            #[cfg(target_arch = "wasm32")]
            self.stream_handle.clone(),
            params,
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn iso_timestamp_now() -> String {
    js_sys::Date::new(&JsValue::from_f64(js_sys::Date::now()))
        .to_iso_string()
        .into()
}

pub fn use_cycle_runner() -> CycleRunnerHandle {
    let actions = use_app_actions();
    let app_state = use_app_state();
    let is_running = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    let stream_handle = use_signal(|| Option::<SseHandle>::None);

    CycleRunnerHandle {
        actions: actions.clone(),
        app_state: app_state.clone(),
        is_running,
        #[cfg(target_arch = "wasm32")]
        stream_handle,
    }
}

fn trigger_cycle_impl(
    actions: &AppActions,
    app_state: &AppSignal,
    #[allow(unused_mut)] mut is_running: Signal<bool>,
    #[cfg(target_arch = "wasm32")]
    #[allow(unused_mut)]
    mut stream_handle: Signal<Option<SseHandle>>,
    params: CycleTriggerParams,
) {
    let Some(config) = APP_CONFIG.get() else {
        actions.set_operation_error("缺少 Thin-Waist 配置".into());
        return;
    };

    let tenant_id = {
        let snapshot = app_state.read();
        snapshot
            .tenant_id
            .clone()
            .or_else(|| config.default_tenant_id.clone())
    };
    let Some(tenant_id) = tenant_id else {
        actions.set_operation_error("请先选择租户".into());
        return;
    };

    let session_id = {
        let snapshot = app_state.read();
        snapshot
            .session_id
            .clone()
            .or_else(|| config.default_session_id.clone())
    };
    let Some(session_id) = session_id else {
        actions.set_operation_error("请先选择会话".into());
        return;
    };

    actions.operation_stage_reset();
    actions.set_operation_diagnostics(Vec::new(), None);

    let draft = MessageEventDraft {
        tenant_id: tenant_id.as_str(),
        session_id: session_id.as_str(),
        scenario: params.scenario,
        subject: params.subject.clone(),
        participants: params.participants.clone(),
        text: &params.text,
        sequence_number: params.sequence_number,
        channel: params.channel.as_deref(),
        access_class: params.access_class,
        provenance: None,
        config_snapshot_hash: None,
        config_snapshot_version: None,
        timestamp_override_ms: None,
    };

    let message_event = match build_message_event(draft) {
        Ok(event) => event,
        Err(err) => {
            actions.set_operation_error(format!("构造事件失败: {err}"));
            return;
        }
    };

    let thin_event: ThinDialogueEvent = message_event.clone().into();
    actions.append_timeline(vec![message_event.clone()], Vec::new(), None);

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(existing) = stream_handle.write().take() {
            existing.close();
        }

        let tenant_id = tenant_id.clone();
        let session_label = session_id.clone();
        let stream_endpoint = config.stream_endpoint();
        let actions = actions.clone();
        let mut is_running = is_running.clone();
        let stream_handle = stream_handle.clone();
        let app_state_clone = app_state.clone();

        wasm_bindgen_futures::spawn_local(async move {
            is_running.set(true);
            actions.operation_stage_start(OperationStageKind::TriggerSubmit, "提交触发请求");
            let triggered_at = iso_timestamp_now();
            actions.set_operation_triggered(Some(triggered_at));
            actions.set_operation_trace(None);
            actions.set_operation_cycle(None);
            actions.set_operation_outcome(None);
            actions.set_operation_success("已提交觉知周期触发请求，等待响应".into());
            actions
                .set_operation_context(Some(format!("触发觉知周期 @ {tenant_id}/{session_label}")));

            let Some(client) = API_CLIENT.get().cloned() else {
                actions.set_operation_error("Thin-Waist 客户端未初始化".into());
                actions.set_operation_context(Some("触发觉知周期".into()));
                is_running.set(false);
                return;
            };

            match client
                .post_trigger_dialogue::<_, CycleTriggerResponse>(
                    &thin_event,
                    Some(tenant_id.as_str()),
                )
                .await
            {
                Ok(env) => {
                    actions.set_operation_trace(env.trace_id.clone());
                    let Some(data) = env.data else {
                        actions.set_operation_error("触发接口返回空数据".into());
                        actions.set_operation_context(Some("触发觉知周期".into()));
                        is_running.set(false);
                        return;
                    };
                    actions.operation_stage_complete(
                        OperationStageKind::TriggerSubmit,
                        Some(format!("状态 {}", data.status)),
                    );
                    actions.set_operation_diagnostics(Vec::new(), None);
                    let cycle_id = data.cycle_id;
                    let cycle_id_label = cycle_id.to_string();
                    actions.operation_stage_start(
                        OperationStageKind::StreamAwait,
                        format!("等待周期 #{cycle_id_label}"),
                    );
                    actions.set_operation_cycle(Some(cycle_id_label.clone()));
                    actions.set_operation_success(format!(
                        "周期 {cycle_id_label} 已触发，状态 {}",
                        data.status
                    ));
                    actions.set_operation_context(Some(format!("觉知周期 #{cycle_id_label}")));
                    actions.select_ace_cycle(Some(cycle_id_label.clone()));

                    start_cycle_stream(
                        cycle_id,
                        cycle_id_label,
                        stream_endpoint,
                        actions.clone(),
                        is_running.clone(),
                        stream_handle.clone(),
                        app_state_clone.clone(),
                    );
                }
                Err(err) => {
                    record_client_error(
                        &actions,
                        &err,
                        "post_trigger_dialogue",
                        "触发觉知周期失败",
                        Some(OperationStageKind::TriggerSubmit),
                    );
                    actions.set_operation_cycle(None);
                    is_running.set(false);
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = thin_event;
        actions.set_operation_error("当前运行环境不支持触发觉知周期".into());
        actions.set_operation_context(Some("触发觉知周期".into()));
        is_running.set(false);
    }
}

#[cfg(target_arch = "wasm32")]
fn start_cycle_stream(
    cycle_id: u64,
    cycle_id_label: String,
    stream_endpoint: String,
    actions: AppActions,
    mut is_running: Signal<bool>,
    mut stream_handle: Signal<Option<SseHandle>>,
    app_state: AppSignal,
) {
    let endpoint = stream_endpoint.trim_end_matches('/');
    let url = format!("{endpoint}/ace/cycles/{cycle_id}/stream");

    let actions_on_open = actions.clone();
    let open_cycle_id = cycle_id_label.clone();
    let on_open = move || {
        actions_on_open.set_operation_success(format!("周期 {open_cycle_id} 流已建立，等待进度"));
    };

    let actions_on_message = actions.clone();
    let message_cycle_id = cycle_id;
    let message_cycle_label = cycle_id_label.clone();
    let mut is_running_message = is_running.clone();
    let mut stream_handle_message = stream_handle.clone();
    let on_message = move |message: SseMessage| {
        handle_cycle_stream_message(
            &actions_on_message,
            &mut is_running_message,
            &mut stream_handle_message,
            message_cycle_id,
            &message_cycle_label,
            app_state.clone(),
            message,
        );
    };

    let actions_on_error = actions.clone();
    let mut is_running_error = is_running.clone();
    let mut stream_handle_error = stream_handle.clone();
    let error_cycle_id = cycle_id_label.clone();
    let on_error = move |err: String| {
        actions_on_error.operation_stage_fail(OperationStageKind::StreamAwait, Some(err.clone()));
        actions_on_error.set_operation_error(format!("周期 {error_cycle_id} 流错误: {err}"));
        is_running_error.set(false);
        if let Some(handle) = stream_handle_error.write().take() {
            handle.close();
        }
    };

    let callbacks = SseCallbacks::new(on_open, on_message, on_error);
    match SseClient::connect(&url, callbacks, SseConnectOptions::default()) {
        Ok(handle) => {
            let mut writer = stream_handle.write();
            if let Some(existing) = writer.take() {
                existing.close();
            }
            *writer = Some(handle);
        }
        Err(err) => {
            actions.operation_stage_fail(OperationStageKind::StreamAwait, Some(err.to_string()));
            actions.set_operation_error(format!("无法订阅周期流: {err}"));
            is_running.set(false);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn handle_cycle_stream_message(
    actions: &AppActions,
    is_running: &mut Signal<bool>,
    stream_handle: &mut Signal<Option<SseHandle>>,
    cycle_id: u64,
    cycle_label: &str,
    app_state: AppSignal,
    message: SseMessage,
) {
    match message.event.as_deref() {
        Some("pending") => {
            actions.set_operation_success(format!("周期 {cycle_label} 等待中…"));
        }
        Some("complete") => {
            let status =
                extract_schedule_status(&message.data).unwrap_or_else(|| "completed".to_string());
            actions.operation_stage_complete(
                OperationStageKind::StreamAwait,
                Some(format!("完成 {status}")),
            );
            is_running.set(false);
            if let Some(handle) = stream_handle.write().take() {
                handle.close();
            }
            let actions_refresh = actions.clone();
            let app_state_clone = app_state.clone();
            let cycle_label_string = cycle_label.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                refresh_after_cycle(actions_refresh, app_state_clone, cycle_label_string, status)
                    .await;
            });
        }
        Some("timeout") => {
            actions.operation_stage_fail(OperationStageKind::StreamAwait, Some("SSE 超时".into()));
            actions.set_operation_error(format!("周期 {cycle_label} 流超时"));
            is_running.set(false);
            if let Some(handle) = stream_handle.write().take() {
                handle.close();
            }
        }
        Some(other) => {
            warn!(
                "忽略周期 {cycle_id} 未知事件 `{other}` 数据: {}",
                message.data
            );
        }
        None => {
            actions.set_operation_success(format!("周期 {cycle_label} 事件: {}", message.data));
        }
    }
}

fn normalized_key(key: &str) -> String {
    key.trim().replace('-', "_").to_ascii_lowercase()
}

pub(crate) fn extract_indices_from_details(value: &Value) -> Vec<String> {
    fn visit(value: &Value, acc: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, entry) in map {
                    let normalized = normalized_key(key);
                    if normalized == "indices_used" || normalized == "indices" {
                        if let Some(array) = entry.as_array() {
                            for item in array {
                                if let Some(text) = item.as_str() {
                                    if !acc
                                        .iter()
                                        .any(|existing| existing.eq_ignore_ascii_case(text))
                                    {
                                        acc.push(text.to_string());
                                    }
                                }
                            }
                        } else if let Some(text) = entry.as_str() {
                            if !acc
                                .iter()
                                .any(|existing| existing.eq_ignore_ascii_case(text))
                            {
                                acc.push(text.to_string());
                            }
                        }
                    } else {
                        visit(entry, acc);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, acc);
                }
            }
            _ => {}
        }
    }

    let mut collected = Vec::new();
    visit(value, &mut collected);
    collected
}

pub(crate) fn extract_budget_hint(value: &Value) -> Option<String> {
    fn visit(value: &Value) -> Option<String> {
        match value {
            Value::Object(map) => {
                let tokens_spent = map.get("tokens_spent").and_then(|v| v.as_u64());
                let tokens_allowed = map.get("tokens_allowed").and_then(|v| v.as_u64());
                let wall_spent = map.get("walltime_ms_used").and_then(|v| v.as_u64());
                let wall_allowed = map.get("walltime_ms_allowed").and_then(|v| v.as_u64());
                if tokens_spent.is_some()
                    || tokens_allowed.is_some()
                    || wall_spent.is_some()
                    || wall_allowed.is_some()
                {
                    let mut parts = Vec::new();
                    if tokens_spent.is_some() || tokens_allowed.is_some() {
                        parts.push(format!(
                            "tokens {}/{}",
                            tokens_spent
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".into()),
                            tokens_allowed
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".into())
                        ));
                    }
                    if wall_spent.is_some() || wall_allowed.is_some() {
                        parts.push(format!(
                            "wall {}ms/{}ms",
                            wall_spent
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".into()),
                            wall_allowed
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".into())
                        ));
                    }
                    if !parts.is_empty() {
                        return Some(parts.join(" · "));
                    }
                }

                if map.keys().any(|key| normalized_key(key).contains("budget")) {
                    for entry in map.values() {
                        if let Some(result) = visit(entry) {
                            return Some(result);
                        }
                    }
                } else {
                    for entry in map.values() {
                        if let Some(result) = visit(entry) {
                            return Some(result);
                        }
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    if let Some(result) = visit(item) {
                        return Some(result);
                    }
                }
            }
            _ => {}
        }
        None
    }

    visit(value)
}

fn extract_schedule_status(payload: &str) -> Option<String> {
    serde_json::from_str::<Value>(payload)
        .ok()
        .and_then(|value| {
            value
                .pointer("/schedule/status")
                .and_then(|v| v.as_str().map(|text| text.to_string()))
        })
}

fn record_client_error(
    actions: &AppActions,
    err: &ClientError,
    context: &str,
    fallback: &str,
    stage: Option<OperationStageKind>,
) {
    if let Some(kind) = stage.clone() {
        actions.operation_stage_fail(kind, Some(err.to_string()));
    }

    if let Some(details) = err.trace_context() {
        let indices = extract_indices_from_details(details);
        let budget = extract_budget_hint(details);
        actions.set_operation_diagnostics(indices, budget);
    } else {
        actions.set_operation_diagnostics(Vec::new(), None);
    }

    if let Some(status) = err.status().map(|code| code.as_u16()) {
        let trace_id = err
            .trace_context()
            .and_then(|ctx| ctx.get("trace_id"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let error_code = match err {
            ClientError::Api(body) => Some(body.code.clone()),
            _ => None,
        };
        actions.record_http_failure(
            status,
            trace_id,
            error_code,
            context.to_string(),
            Some(err.to_string()),
        );
    } else {
        actions.set_operation_error(format!("{fallback}: {err}"));
        actions.set_operation_context(Some(context.to_string()));
    }
}

#[cfg(target_arch = "wasm32")]
async fn refresh_after_cycle(
    actions: AppActions,
    app_state: AppSignal,
    cycle_label: String,
    status: String,
) {
    let Some(client) = API_CLIENT.get().cloned() else {
        actions.set_operation_error("Thin-Waist 客户端未初始化".into());
        return;
    };

    let snapshot = app_state.read();
    let tenant_id = snapshot.tenant_id.clone().or_else(|| {
        APP_CONFIG
            .get()
            .and_then(|cfg| cfg.default_tenant_id.clone())
    });
    let session_id = snapshot.session_id.clone().or_else(|| {
        APP_CONFIG
            .get()
            .and_then(|cfg| cfg.default_session_id.clone())
    });
    let mut query = snapshot.timeline.query.clone();
    if query.limit == 0 {
        query.limit = 50;
    }
    if query.session_id.is_none() {
        query.session_id = session_id.clone();
    }
    if query.scenario.is_none() {
        query.scenario = snapshot.scenario_filter.clone();
    }
    drop(snapshot);

    let Some(tenant) = tenant_id else {
        actions.set_operation_error("请先选择租户".into());
        return;
    };

    let mut refresh_ok = true;

    actions.set_timeline_loading(true);
    actions.operation_stage_start(
        OperationStageKind::SnapshotRefresh,
        format!("刷新周期 #{cycle_label}"),
    );
    actions.set_operation_diagnostics(Vec::new(), None);

    actions.set_timeline_error(None);
    match client
        .get_timeline::<_, TimelinePayload>(&tenant, &query)
        .await
    {
        Ok(env) => {
            if let Some(payload) = env.data {
                actions.reset_timeline();
                actions.append_timeline(
                    payload.items,
                    payload.awareness,
                    payload.next_cursor.clone(),
                );
                actions.update_next_cursor(payload.next_cursor);
                actions.set_timeline_loading(false);
            } else {
                actions.reset_timeline();
                actions.set_timeline_error(Some("时间线返回空数据".into()));
                actions.set_timeline_loading(false);
                refresh_ok = false;
            }
        }
        Err(err) => {
            record_client_error(
                &actions,
                &err,
                "refresh_timeline",
                "刷新时间线失败",
                Some(OperationStageKind::SnapshotRefresh),
            );
            actions.set_timeline_loading(false);
            refresh_ok = false;
        }
    }

    actions.set_context_loading(true);
    actions.set_context_error(None);
    let bundle_res = client
        .get_context_bundle::<(), ContextBundleView>(&tenant, None)
        .await;
    let explain_res = client.get_explain_indices::<ExplainIndices>(&tenant).await;

    match (bundle_res, explain_res) {
        (Ok(bundle_env), Ok(explain_env)) => {
            if bundle_env.data.is_none() {
                actions.set_context_error(Some("ContextBundle 为空".into()));
                actions.set_context_loading(false);
                refresh_ok = false;
            } else {
                actions.set_context_bundle(bundle_env.data, explain_env.data);
            }
        }
        (bundle, explain) => {
            let mut message = String::new();
            if let Err(err) = bundle {
                record_client_error(
                    &actions,
                    &err,
                    "context_bundle",
                    "上下文加载失败",
                    Some(OperationStageKind::SnapshotRefresh),
                );
                message.push_str(&format!("上下文加载失败: {err}"));
            }
            if let Err(err) = explain {
                record_client_error(
                    &actions,
                    &err,
                    "explain_indices",
                    "Explain 指纹加载失败",
                    Some(OperationStageKind::SnapshotRefresh),
                );
                if !message.is_empty() {
                    message.push_str("；");
                }
                message.push_str(&format!("Explain 指纹加载失败: {err}"));
            }
            actions.set_context_error(Some(message));
            actions.set_context_loading(false);
            refresh_ok = false;
        }
    }

    actions.set_ace_snapshot_loading(true);
    actions.set_ace_snapshot_error(None);
    actions.operation_stage_start(
        OperationStageKind::OutboxReady,
        format!("加载 Outbox #{cycle_label}"),
    );
    let snapshot_res = client
        .get_cycle_snapshot::<CycleSnapshotView>(&cycle_label, Some(&tenant))
        .await;
    let outbox_res = client
        .get_cycle_outbox::<Vec<OutboxMessageView>>(&cycle_label, Some(&tenant))
        .await;

    match (snapshot_res, outbox_res) {
        (Ok(snapshot_env), Ok(outbox_env)) => {
            if let Some(snapshot) = snapshot_env.data {
                let outbox = outbox_env.data.unwrap_or_default();
                let outbox_count = outbox.len();
                let outcome = snapshot.outcomes.last().cloned();
                actions.store_ace_snapshot(cycle_label.clone(), snapshot, outbox);
                actions.operation_stage_complete(
                    OperationStageKind::OutboxReady,
                    Some(format!("Outbox {} 条", outbox_count)),
                );
                actions.set_operation_outcome(outcome);
            } else {
                actions.set_ace_snapshot_error(Some("周期快照为空".into()));
                actions.set_operation_outcome(None);
                refresh_ok = false;
            }
        }
        (snapshot, outbox) => {
            let mut message = String::new();
            if let Err(err) = snapshot {
                message.push_str(&format!("快照加载失败: {err}"));
                record_client_error(
                    &actions,
                    &err,
                    "cycle_snapshot",
                    "周期快照加载失败",
                    Some(OperationStageKind::SnapshotRefresh),
                );
            }
            if let Err(err) = outbox {
                if !message.is_empty() {
                    message.push_str("；");
                }
                message.push_str(&format!("Outbox 加载失败: {err}"));
                record_client_error(
                    &actions,
                    &err,
                    "cycle_outbox",
                    "Outbox 加载失败",
                    Some(OperationStageKind::OutboxReady),
                );
            }
            let combined_message = if message.is_empty() {
                "加载周期快照失败".into()
            } else {
                message
            };
            actions.set_ace_snapshot_error(Some(combined_message.clone()));
            actions.set_operation_outcome(None);
            actions.operation_stage_fail(
                OperationStageKind::OutboxReady,
                Some(combined_message.clone()),
            );
            actions
                .operation_stage_fail(OperationStageKind::SnapshotRefresh, Some(combined_message));
            refresh_ok = false;
        }
    }

    if refresh_ok {
        actions.operation_stage_complete(
            OperationStageKind::SnapshotRefresh,
            Some(format!("周期 {cycle_label} 状态 {status}")),
        );
        actions.set_operation_success(format!("周期 {cycle_label} 完成: {status}，视图已更新"));
        actions.set_operation_cycle(Some(cycle_label.clone()));
        actions.set_operation_context(Some(format!("觉知周期 #{cycle_label}")));
    }
}
