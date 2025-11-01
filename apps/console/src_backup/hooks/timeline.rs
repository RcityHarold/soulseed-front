use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::TimelinePayload;
use crate::state::{use_app_actions, use_app_state};
use crate::{API_CLIENT, APP_CONFIG};

/// 监听租户、会话以及场景筛选的变化，当前使用演示数据模拟加载效果。
pub fn use_timeline_loader() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    let scenario_filter = snapshot.scenario_filter.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant, session, scenario_filter)| {
        let actions = actions.clone();
        let mut state = state.clone();
        async move {
            tracing::info!(
                "timeline loader triggered: tenant={:?}, session={:?}, scenario={:?}",
                tenant,
                session,
                scenario_filter
            );
            TimeoutFuture::new(0).await;

            actions.reset_timeline();
            actions.set_timeline_loading(true);
            actions.set_timeline_error(None);

            let tenant_id = tenant.clone().or_else(|| {
                APP_CONFIG
                    .get()
                    .and_then(|cfg| cfg.default_tenant_id.clone())
            });

            let Some(tenant_id) = tenant_id else {
                actions.set_timeline_loading(false);
                actions.set_timeline_error(Some("请先选择租户".into()));
                return;
            };

            let mut query = {
                let snapshot = state.read();
                snapshot.timeline.query.clone()
            };
            if query.limit == 0 {
                query.limit = 50;
            }
            query.session_id = session.clone().or_else(|| {
                APP_CONFIG
                    .get()
                    .and_then(|cfg| cfg.default_session_id.clone())
            });
            query.scenario = scenario_filter.clone();

            {
                let mut writable = state.write();
                writable.timeline.query.limit = query.limit;
                writable.timeline.query.session_id = query.session_id.clone();
                writable.timeline.query.scenario = query.scenario.clone();
            }

            let client = API_CLIENT.get().cloned();

            if let Some(client) = client {
                match client
                    .get_timeline::<_, TimelinePayload>(&tenant_id, &query)
                    .await
                {
                    Ok(env) => {
                        if let Some(payload) = env.data {
                            actions.append_timeline(
                                payload.items,
                                payload.awareness,
                                payload.next_cursor.clone(),
                            );
                            actions.update_next_cursor(payload.next_cursor);
                            actions.set_timeline_loading(false);
                        } else {
                            actions.set_timeline_loading(false);
                            actions.set_timeline_error(Some("时间线返回空数据".into()));
                        }
                    }
                    Err(err) => {
                        tracing::error!("timeline fetch failed: {err}");
                        actions.set_timeline_loading(false);
                        actions.set_timeline_error(Some(format!("时间线加载失败: {err}")));
                    }
                }
            } else {
                actions.set_timeline_loading(false);
                actions.set_timeline_error(Some("Thin-Waist 客户端未初始化".into()));
            }
        }
    }));
}
