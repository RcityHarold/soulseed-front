use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_timeline_data;
use crate::state::{use_app_actions, use_app_state};

/// 监听租户、会话以及场景筛选的变化，当前使用演示数据模拟加载效果。
pub fn use_timeline_loader(cx: Scope) {
    let actions = use_app_actions(cx);
    let state = use_app_state(cx);

    let dependencies = {
        let snapshot = state.read();
        (
            snapshot.tenant_id.clone(),
            snapshot.session_id.clone(),
            snapshot.scenario_filter.clone(),
        )
    };

    use_effect(cx, dependencies, move |(_, _, scenario_filter)| {
        let actions = actions.clone();
        let scenario_filter = scenario_filter.clone();
        async move {
            actions.reset_timeline();
            actions.set_timeline_loading(true);
            actions.set_timeline_error(None);

            TimeoutFuture::new(150).await;

            let (events, awareness) = sample_timeline_data();
            let filtered_events = match scenario_filter {
                Some(filter) => events
                    .into_iter()
                    .filter(|event| event.scenario == filter)
                    .collect(),
                None => events,
            };

            actions.append_timeline(filtered_events, awareness, None);
        }
    });
}
