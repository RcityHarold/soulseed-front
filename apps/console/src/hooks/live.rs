use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_live_event;
use crate::state::{use_app_actions, use_app_state};

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
            tracing::info!("live stream watcher: tenant={:?}, session={:?}", tenant, session);
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
