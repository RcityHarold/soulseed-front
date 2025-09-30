use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::graph::sample_graph_bundle;
use crate::state::{use_app_actions, use_app_state};

pub fn use_graph_insights(cx: Scope) {
    let actions = use_app_actions(cx);
    let state = use_app_state(cx);

    let dependencies = {
        let snapshot = state.read();
        (
            snapshot.tenant_id.clone(),
            snapshot.session_id.clone(),
            snapshot.graph.query.root_event_id,
        )
    };

    use_effect(cx, dependencies, move |(tenant, session, root)| {
        let actions = actions.clone();
        async move {
            if tenant.is_none() || session.is_none() {
                actions.set_graph_data(None, Vec::new());
                return;
            }

            let Some(root_id) = root else {
                actions.set_graph_data(None, Vec::new());
                return;
            };

            actions.set_graph_loading(true);
            actions.set_graph_error(None);

            TimeoutFuture::new(120).await;

            let sample = sample_graph_bundle(root_id);
            actions.set_graph_data(Some(sample.causal), sample.recall);
        }
    });
}
