use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_live_event;
use crate::state::{use_app_actions, use_app_state};

pub fn use_live_stream(cx: Scope) {
    let actions = use_app_actions(cx);
    let state = use_app_state(cx);

    let dependencies = {
        let snapshot = state.read();
        (snapshot.tenant_id.clone(), snapshot.session_id.clone())
    };

    use_effect(cx, dependencies, move |(tenant, session)| {
        let actions = actions.clone();
        async move {
            if tenant.is_none() || session.is_none() {
                actions.set_live_connected(false);
                actions.set_live_error(None);
                return;
            }

            actions.set_live_connected(true);
            actions.set_live_error(None);

            for seq in 0u64..5 {
                TimeoutFuture::new(1_200).await;
                let (dialogue, awareness) = sample_live_event(seq);
                let awareness_items = awareness.into_iter().collect::<Vec<_>>();
                actions.append_timeline(vec![dialogue.clone()], awareness_items, None);
                actions.record_live_event(dialogue.event_id.0);
            }
        }
    });
}
