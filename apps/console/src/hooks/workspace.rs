use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::graph::sample_graph_bundle;
use crate::fixtures::timeline::{sample_ace_cycles, sample_context_bundle};
use crate::fixtures::workspace::sample_workspace_profiles;
use crate::state::{use_app_actions, use_app_state};
use crate::APP_CONFIG;

pub fn use_workspace_overview() {
    let actions = use_app_actions();
    let state = use_app_state();

    use_future(move || {
        let actions = actions.clone();
        let state = state.clone();
        async move {
            if !state.read().workspace.tenants.is_empty() {
                return;
            }

            TimeoutFuture::new(0).await;

            actions.set_workspace_loading(true);
            actions.set_workspace_error(None);

            TimeoutFuture::new(120).await;

            let data = sample_workspace_profiles();
            actions.set_workspace_data(data.clone());

            let snapshot = state.read();
            let needs_tenant = snapshot.tenant_id.is_none();
            let needs_session = snapshot.session_id.is_none();
            let current_tenant = snapshot.tenant_id.clone();
            drop(snapshot);

            if !(needs_tenant || needs_session) {
                return;
            }

            let configured_tenant = APP_CONFIG
                .get()
                .and_then(|cfg| cfg.default_tenant_id.clone());

            let selected_tenant = current_tenant
                .or(configured_tenant)
                .or_else(|| data.first().map(|tenant| tenant.tenant_id.clone()));

            let Some(tenant_id) = selected_tenant else {
                return;
            };

            if let Some(tenant) = data.iter().find(|tenant| tenant.tenant_id == tenant_id) {
                if needs_tenant {
                    actions.set_tenant(Some(tenant_id.clone()));
                }

                if needs_session {
                    let default_session = tenant
                        .pinned_sessions
                        .iter()
                        .chain(tenant.recent_sessions.iter())
                        .find(|session| session.pinned)
                        .or_else(|| tenant.recent_sessions.first());
                    if let Some(session) = default_session {
                        actions.set_session(Some(session.session_id.clone()));
                    }
                }
            }

            let snapshot = state.read();
            let timeline_empty = snapshot.timeline.events.is_empty();
            let context_missing = snapshot.context.bundle.is_none();
            let ace_missing = snapshot.ace.cycles.is_empty();
            let graph_missing = snapshot.graph.causal.is_none();
            let root_candidate = snapshot
                .timeline
                .events
                .first()
                .map(|event| event.event_id.as_u64());
            drop(snapshot);

            if timeline_empty {
                actions.playback_sample_timeline();
            }

            if context_missing {
                let (bundle, indices) = sample_context_bundle();
                actions.set_context_bundle(Some(bundle), Some(indices));
            }

            if ace_missing {
                let cycles = sample_ace_cycles();
                let selected = cycles.first().map(|cycle| cycle.cycle_id.clone());
                actions.set_ace_cycles(cycles);
                actions.select_ace_cycle(selected);
            }

            if graph_missing {
                if let Some(root) = root_candidate {
                    let sample = sample_graph_bundle(root);
                    actions.set_graph_data(Some(sample.causal), sample.recall);
                }
            }
        }
    });
}
