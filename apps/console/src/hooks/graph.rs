use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::graph::sample_graph_bundle;
use crate::state::{use_app_actions, use_app_state};

pub fn use_graph_insights() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    let root = snapshot.graph.query.root_event_id;
    drop(snapshot);

    use_future(use_reactive!(|(tenant, session, root)| {
        let actions = actions.clone();
        async move {
            tracing::info!(
                "graph loader triggered: tenant={:?}, session={:?}, root={:?}",
                tenant,
                session,
                root
            );
            if tenant.is_none() || session.is_none() {
                actions.set_graph_data(None, Vec::new());
                return;
            }

            let Some(root_id) = root else {
                actions.set_graph_data(None, Vec::new());
                return;
            };

            TimeoutFuture::new(0).await;

            actions.set_graph_loading(true);
            actions.set_graph_error(None);

            TimeoutFuture::new(120).await;

            let sample = sample_graph_bundle(root_id);
            actions.set_graph_data(Some(sample.causal), sample.recall);
        }
    }));
}
