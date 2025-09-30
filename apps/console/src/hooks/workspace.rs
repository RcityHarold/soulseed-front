use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::workspace::sample_workspace_profiles;
use crate::state::{use_app_actions, use_app_state};

pub fn use_workspace_overview(cx: Scope) {
    let actions = use_app_actions(cx);
    let state = use_app_state(cx);

    use_effect(cx, (), move |_| {
        let actions = actions.clone();
        let state = state.clone();
        async move {
            if !state.read().workspace.tenants.is_empty() {
                return;
            }

            actions.set_workspace_loading(true);
            actions.set_workspace_error(None);

            TimeoutFuture::new(120).await;

            let data = sample_workspace_profiles();
            actions.set_workspace_data(data.clone());

            let needs_tenant = state.read().tenant_id.is_none();
            let needs_session = state.read().session_id.is_none();

            if needs_tenant || needs_session {
                let selected_tenant = state
                    .read()
                    .tenant_id
                    .clone()
                    .or_else(|| data.first().map(|tenant| tenant.tenant_id.clone()));

                if let Some(tenant_id) = selected_tenant {
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
                }
            }
        }
    });
}
