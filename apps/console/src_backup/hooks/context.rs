use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::{ContextBundleView, ExplainIndices};
use crate::state::{use_app_actions, use_app_state};
use crate::{API_CLIENT, APP_CONFIG};

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
            TimeoutFuture::new(0).await;

            actions.set_context_loading(true);
            actions.set_context_error(None);

            let tenant = tenant_id.clone().or_else(|| {
                APP_CONFIG
                    .get()
                    .and_then(|cfg| cfg.default_tenant_id.clone())
            });

            let Some(tenant) = tenant else {
                actions.set_context_error(Some("请先选择租户".into()));
                actions.set_context_loading(false);
                return;
            };

            let client = API_CLIENT.get().cloned();

            if let Some(client) = client {
                let bundle_res = client
                    .get_context_bundle::<(), ContextBundleView>(&tenant, None)
                    .await;
                let explain_res = client.get_explain_indices::<ExplainIndices>(&tenant).await;

                match (bundle_res, explain_res) {
                    (Ok(bundle_env), Ok(explain_env)) => {
                        let bundle = bundle_env.data;
                        let indices = explain_env.data;
                        if bundle.is_none() {
                            actions.set_context_error(Some("ContextBundle 为空".into()));
                            actions.set_context_loading(false);
                            return;
                        }
                        actions.set_context_bundle(bundle, indices);
                    }
                    (bundle, explain) => {
                        let mut message = String::new();
                        if let Err(err) = bundle {
                            tracing::error!("context bundle fetch failed: {err}");
                            message.push_str(&format!("上下文加载失败: {err}"));
                        }
                        if let Err(err) = explain {
                            tracing::error!("explain indices fetch failed: {err}");
                            if !message.is_empty() {
                                message.push_str("；");
                            }
                            message.push_str(&format!("Explain 指纹加载失败: {err}"));
                        }
                        actions.set_context_error(Some(message));
                        actions.set_context_loading(false);
                        return;
                    }
                }
            } else {
                actions.set_context_error(Some("Thin-Waist 客户端未初始化".into()));
                actions.set_context_loading(false);
                return;
            }

            actions.set_context_loading(false);
        }
    }));
}
