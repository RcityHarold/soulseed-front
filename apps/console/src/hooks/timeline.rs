use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_timeline_data;
use crate::state::{use_app_actions, use_app_state};

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
        async move {
            tracing::info!(
                "timeline loader triggered: tenant={:?}, session={:?}, scenario={:?}",
                tenant,
                session,
                scenario_filter
            );
            if tenant.is_none() || session.is_none() {
                actions.reset_timeline();
                return;
            }

            TimeoutFuture::new(0).await;

            actions.reset_timeline();
            actions.set_timeline_loading(true);
            actions.set_timeline_error(None);

            TimeoutFuture::new(150).await;

            let (events, awareness) = sample_timeline_data();
            let filtered_events = match scenario_filter.clone() {
                Some(filter) => events
                    .into_iter()
                    .filter(|event| event.scenario == filter)
                    .collect(),
                None => events,
            };

            actions.append_timeline(filtered_events, awareness, None);
        }
    }));
}
