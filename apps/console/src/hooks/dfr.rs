//! DFR 决策增强 Hooks
//!
//! 提供 DFR 决策相关的 hooks，包括决策详情、指纹匹配等

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::{DecisionDetail, FingerprintListResponse, FingerprintMatchResult};
use crate::state::use_app_state;
use crate::{API_CLIENT, APP_CONFIG};

/// DFR 决策状态
#[derive(Clone, Debug, Default)]
pub struct DfrState {
    pub loading: bool,
    pub error: Option<String>,
    pub decision: Option<DecisionDetail>,
    pub fingerprints: Option<FingerprintListResponse>,
    pub match_result: Option<FingerprintMatchResult>,
}

/// DFR 决策详情 Hook
pub fn use_dfr_decision(decision_id: String) -> Signal<DfrState> {
    let state_store = use_app_state();
    let mut state = use_signal(DfrState::default);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
        let decision_id = decision_id.clone();
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

            if decision_id.is_empty() {
                state.write().decision = None;
                return;
            }

            let Some(client) = API_CLIENT.get().cloned() else {
                state.write().error = Some("API 客户端未初始化".into());
                return;
            };

            state.write().loading = true;
            state.write().error = None;

            match client
                .get_dfr_decision::<DecisionDetail>(&tenant, &decision_id)
                .await
            {
                Ok(env) => {
                    state.write().decision = env.data;
                }
                Err(err) => {
                    tracing::error!("决策详情加载失败: {err}");
                    state.write().error = Some(format!("加载失败: {err}"));
                }
            }

            state.write().loading = false;
        }
    }));

    state
}

/// 指纹列表 Hook
pub fn use_fingerprint_list() -> Signal<Option<FingerprintListResponse>> {
    let state_store = use_app_state();
    let mut fingerprints = use_signal(|| None::<FingerprintListResponse>);

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
                return;
            };

            let Some(client) = API_CLIENT.get().cloned() else {
                return;
            };

            #[derive(serde::Serialize)]
            struct FingerprintQuery {
                session_id: Option<String>,
            }

            let query = FingerprintQuery {
                session_id: session_id.clone(),
            };

            match client
                .get_dfr_fingerprints::<FingerprintQuery, FingerprintListResponse>(&tenant, Some(&query))
                .await
            {
                Ok(env) => {
                    fingerprints.set(env.data);
                }
                Err(err) => {
                    tracing::error!("指纹列表加载失败: {err}");
                }
            }
        }
    }));

    fingerprints
}
