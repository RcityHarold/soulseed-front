use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_context_bundle;
use crate::state::{use_app_actions, use_app_state};

/// 加载最新 ContextBundle 与 Explain 指纹。
pub fn use_context_bundle() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant_id = snapshot.tenant_id.clone();
    let session_id = snapshot.session_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id, session_id)| {
        let actions = actions.clone();
        async move {
            tracing::info!(
                "context loader triggered: tenant={:?}, session={:?}",
                tenant_id,
                session_id
            );
            if tenant_id.is_none() || session_id.is_none() {
                actions.set_context_bundle(None, None);
                return;
            }

            TimeoutFuture::new(0).await;

            actions.set_context_loading(true);
            actions.set_context_error(None);

            TimeoutFuture::new(120).await;

            let (bundle, indices) = sample_context_bundle();
            actions.set_context_bundle(Some(bundle), Some(indices));
        }
    }));
}
