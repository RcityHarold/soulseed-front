use crate::state::{use_app_actions, use_app_state};
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::APP_CONFIG;

#[cfg(target_arch = "wasm32")]
use {
    crate::models::{AwarenessEvent, DialogueEvent},
    crate::services::sse::{SseCallbacks, SseClient, SseConnectOptions, SseHandle, SseMessage},
    serde::Deserialize,
    tracing::warn,
};

#[cfg(target_arch = "wasm32")]
pub fn use_live_stream() {
    let actions = use_app_actions();
    let state = use_app_state();
    let handle_slot = use_signal(|| Option::<SseHandle>::None);

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant, session)| {
        let actions = actions.clone();
        let mut handle_slot = handle_slot.clone();
        async move {
            {
                let mut writer = handle_slot.write();
                if let Some(existing) = writer.take() {
                    existing.close();
                }
            }

            let Some(config) = APP_CONFIG.get() else {
                actions.set_live_connected(false);
                actions.set_live_error(Some("缺少 Thin-Waist 配置".into()));
                return;
            };

            let tenant_id = tenant.clone().or_else(|| config.default_tenant_id.clone());
            let Some(tenant_id) = tenant_id else {
                actions.set_live_connected(false);
                actions.set_live_error(Some("请先选择租户".into()));
                return;
            };

            let session_id = session
                .clone()
                .or_else(|| config.default_session_id.clone());
            let Some(session_id) = session_id else {
                actions.set_live_connected(false);
                actions.set_live_error(Some("请先选择会话".into()));
                return;
            };

            let base = config.stream_endpoint();
            let endpoint = base.trim_end_matches('/');
            let url = format!("{endpoint}/tenants/{tenant_id}/live/dialogues/{session_id}");

            actions.set_live_connected(false);
            actions.set_live_error(None);

            let mut options = SseConnectOptions::default();
            let heartbeat_ms = config
                .sse_timeout
                .as_millis()
                .try_into()
                .unwrap_or(options.heartbeat_timeout_ms);
            options.heartbeat_timeout_ms = heartbeat_ms.max(5_000);

            let on_open_actions = actions.clone();
            let callbacks = SseCallbacks::new(
                move || {
                    on_open_actions.set_live_connected(true);
                },
                {
                    let actions = actions.clone();
                    move |message: SseMessage| {
                        handle_stream_message(&actions, message);
                    }
                },
                {
                    let actions = actions.clone();
                    move |err| {
                        actions.set_live_error(Some(err));
                        actions.set_live_connected(false);
                    }
                },
            );

            match SseClient::connect(&url, callbacks, SseConnectOptions::default()) {
                Ok(handle) => {
                    *handle_slot.write() = Some(handle);
                }
                Err(err) => {
                    actions.set_live_connected(false);
                    actions.set_live_error(Some(err.to_string()));
                }
            }
        }
    }));
}

#[cfg(target_arch = "wasm32")]
fn handle_stream_message(actions: &crate::state::AppActions, message: SseMessage) {
    #[derive(Deserialize)]
    struct DialogueStreamPayload {
        #[serde(rename = "dialogue_event")]
        dialogue_event: DialogueEvent,
        #[serde(default, alias = "awareness_events", alias = "awareness")]
        awareness_events: Vec<AwarenessEvent>,
    }

    #[derive(Deserialize)]
    struct AwarenessStreamPayload {
        #[serde(rename = "awareness_event", alias = "event")]
        awareness_event: AwarenessEvent,
    }

    match message.event.as_deref() {
        Some("ping") => {
            // heartbeat handled by SSE client; no-op for UI
        }
        Some("awareness_event") => {
            match serde_json::from_str::<AwarenessStreamPayload>(&message.data) {
                Ok(payload) => {
                    actions.append_timeline(Vec::new(), vec![payload.awareness_event], None);
                }
                Err(err) => {
                    warn!("解析 AwarenessEvent SSE 数据失败: {err}");
                }
            }
        }
        Some("dialogue_event") | None => {
            match serde_json::from_str::<DialogueStreamPayload>(&message.data) {
                Ok(DialogueStreamPayload {
                    dialogue_event,
                    awareness_events,
                }) => {
                    let legacy_event: crate::models::DialogueEvent = dialogue_event.into();
                    let event_id = legacy_event.event_id.as_u64();
                    actions.append_timeline(vec![legacy_event], awareness_events, None);
                    actions.record_live_event(event_id);
                }
                Err(err) => {
                    warn!("解析 DialogueEvent SSE 数据失败: {err}");
                }
            }
        }
        Some(other) => {
            if let Ok(DialogueStreamPayload {
                dialogue_event,
                awareness_events,
            }) = serde_json::from_str::<DialogueStreamPayload>(&message.data)
            {
                let legacy_event: crate::models::DialogueEvent = dialogue_event.into();
                let event_id = legacy_event.event_id.as_u64();
                actions.append_timeline(vec![legacy_event], awareness_events, None);
                actions.record_live_event(event_id);
                return;
            }

            if let Ok(payload) = serde_json::from_str::<AwarenessStreamPayload>(&message.data) {
                actions.append_timeline(Vec::new(), vec![payload.awareness_event], None);
                return;
            }

            warn!("忽略未知 SSE 事件 `{other}`: {}", message.data);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use crate::fixtures::timeline::sample_live_event;
#[cfg(not(target_arch = "wasm32"))]
use gloo_timers::future::TimeoutFuture;

#[cfg(not(target_arch = "wasm32"))]
pub fn use_live_stream() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant, session)| {
        let actions = actions.clone();
        async move {
            tracing::info!(
                "live stream watcher: tenant={:?}, session={:?}",
                tenant,
                session
            );
            if tenant.is_none() || session.is_none() {
                actions.set_live_connected(false);
                actions.set_live_error(None);
                return;
            }

            TimeoutFuture::new(0).await;

            actions.set_live_connected(true);
            actions.set_live_error(None);

            for seq in 0u64..5 {
                TimeoutFuture::new(1_200).await;
                let (dialogue, awareness) = sample_live_event(seq);
                let event_id = dialogue.event_id.as_u64();
                let awareness_items = awareness.into_iter().collect::<Vec<_>>();
                actions.append_timeline(vec![dialogue], awareness_items, None);
                actions.record_live_event(event_id);
            }
        }
    }));
}
