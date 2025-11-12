use crate::state::{use_app_actions, use_app_state};
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::{API_CLIENT, APP_CONFIG};

#[cfg(target_arch = "wasm32")]
use {
    crate::api::AwarenessQuery,
    crate::models::AwarenessEvent,
    gloo_timers::future::TimeoutFuture,
    tracing::{debug, warn},
};

#[cfg(target_arch = "wasm32")]
pub fn use_live_stream() {
    let actions = use_app_actions();
    let state = use_app_state();
    let last_event_time = use_signal(|| 0i64);

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|tenant| {
        let actions = actions.clone();
        let mut last_event_time = last_event_time.clone();
        async move {
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

            let client = match API_CLIENT.get().cloned() {
                Some(client) => client,
                None => {
                    actions.set_live_connected(false);
                    actions.set_live_error(Some("Thin-Waist 客户端未初始化".into()));
                    return;
                }
            };

            actions.set_live_connected(true);
            actions.set_live_error(None);

            let mut consecutive_errors = 0;
            let base_interval_ms = 3000; // 基础轮询间隔 3 秒
            let max_interval_ms = 15000; // 最大轮询间隔 15 秒

            loop {
                // 计算轮询间隔（出错时使用指数退避）
                let interval_ms = if consecutive_errors > 0 {
                    (base_interval_ms * 2u64.pow(consecutive_errors.min(3)))
                        .min(max_interval_ms)
                } else {
                    base_interval_ms
                };

                TimeoutFuture::new(interval_ms as u32).await;

                // 获取觉知事件
                match client
                    .get_awareness_events::<_, Vec<AwarenessEvent>>(
                        &tenant_id,
                        &AwarenessQuery { limit: 50 },
                    )
                    .await
                {
                    Ok(env) => {
                        consecutive_errors = 0;
                        actions.set_live_error(None);

                        if let Some(events) = env.data {
                            // 过滤出新事件（时间戳大于上次处理的）
                            let last_time = *last_event_time.read();
                            let new_events: Vec<AwarenessEvent> = events
                                .into_iter()
                                .filter(|evt| evt.occurred_at_ms > last_time)
                                .collect();

                            if !new_events.is_empty() {
                                // 更新最后事件时间戳
                                if let Some(max_time) = new_events
                                    .iter()
                                    .map(|evt| evt.occurred_at_ms)
                                    .max()
                                {
                                    *last_event_time.write() = max_time;
                                }

                                debug!(
                                    "轮询获取到 {} 个新觉知事件",
                                    new_events.len()
                                );

                                // 将新事件添加到时间线
                                actions.append_timeline(Vec::new(), new_events, None);
                            }
                        }
                    }
                    Err(err) => {
                        consecutive_errors += 1;
                        let err_msg = format!("轮询觉知事件失败: {}", err);
                        warn!("{}", err_msg);
                        actions.set_live_error(Some(err_msg));

                        // 如果连续失败太多次，标记为未连接
                        if consecutive_errors >= 5 {
                            actions.set_live_connected(false);
                        }
                    }
                }
            }
        }
    }));
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
