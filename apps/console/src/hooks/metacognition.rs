//! 元认知分析 Hooks
//!
//! 提供元认知分析相关的 hooks

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::AnalysisResultResponse;
use crate::state::use_app_state;
use crate::{API_CLIENT, APP_CONFIG};

/// 元认知分析状态
#[derive(Clone, Debug, Default)]
pub struct MetacognitionState {
    pub loading: bool,
    pub error: Option<String>,
    pub analysis: Option<AnalysisResultResponse>,
}

/// 元认知分析 Hook
pub fn use_metacognition_analysis() -> Signal<MetacognitionState> {
    let state_store = use_app_state();
    let mut state = use_signal(MetacognitionState::default);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    let session_id = snapshot.session_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id, session_id)| {
        async move {
            TimeoutFuture::new(0).await;

            let tenant = tenant_id.clone().or_else(|| {
                APP_CONFIG
                    .get()
                    .and_then(|cfg| cfg.default_tenant_id.clone())
            });

            let Some(tenant) = tenant else {
                state.write().error = Some("请先选择租户".into());
                return;
            };

            let Some(client) = API_CLIENT.get().cloned() else {
                state.write().error = Some("API 客户端未初始化".into());
                return;
            };

            state.write().loading = true;
            state.write().error = None;

            // 简化查询 - 不传递额外参数
            match client
                .get_metacognition_analysis::<(), AnalysisResultResponse>(&tenant, &())
                .await
            {
                Ok(env) => {
                    state.write().analysis = env.data;
                }
                Err(err) => {
                    tracing::error!("元认知分析加载失败: {err}");
                    state.write().error = Some(format!("加载失败: {err}"));
                }
            }

            state.write().loading = false;
        }
    }));

    state
}
