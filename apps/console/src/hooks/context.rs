use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::fixtures::timeline::sample_context_bundle;
use crate::state::{use_app_actions, use_app_state};

/// 加载最新 ContextBundle 与 Explain 指纹。
pub fn use_context_bundle(cx: Scope) {
    let actions = use_app_actions(cx);
    let state = use_app_state(cx);

    let dependencies = {
        let snapshot = state.read();
        (snapshot.tenant_id.clone(), snapshot.session_id.clone())
    };

    use_effect(cx, dependencies, move |(tenant_id, session_id)| {
        let actions = actions.clone();
        async move {
            if tenant_id.is_none() || session_id.is_none() {
                actions.set_context_bundle(None, None);
                return;
            }

            actions.set_context_loading(true);
            actions.set_context_error(None);

            // 模拟网络延迟，后续可替换为实际 API 调用
            TimeoutFuture::new(120).await;

            let (bundle, indices) = sample_context_bundle();
            actions.set_context_bundle(Some(bundle), Some(indices));
        }
    });
}
