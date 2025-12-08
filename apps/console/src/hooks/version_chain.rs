//! 版本链与图谱增强 Hooks
//!
//! 提供版本链查询、图谱节点/边详情等 hooks

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::{GraphEdgeDetail, GraphNodeDetail, VersionChainSummary, VersionDiff};
use crate::state::use_app_state;
use crate::{API_CLIENT, APP_CONFIG};

/// 版本链状态
#[derive(Clone, Debug, Default)]
pub struct VersionChainState {
    pub loading: bool,
    pub error: Option<String>,
    pub chain: Option<VersionChainSummary>,
    pub diff: Option<VersionDiff>,
}

/// 版本链摘要 Hook
pub fn use_version_chain(entity_type: String, entity_id: String) -> Signal<VersionChainState> {
    let state_store = use_app_state();
    let mut state = use_signal(VersionChainState::default);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
        let entity_type = entity_type.clone();
        let entity_id = entity_id.clone();
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

            if entity_id.is_empty() {
                return;
            }

            let Some(client) = API_CLIENT.get().cloned() else {
                state.write().error = Some("API 客户端未初始化".into());
                return;
            };

            state.write().loading = true;
            state.write().error = None;

            match client
                .get_version_chain_summary::<VersionChainSummary>(&tenant, &entity_type, &entity_id)
                .await
            {
                Ok(env) => {
                    state.write().chain = env.data;
                }
                Err(err) => {
                    tracing::error!("版本链加载失败: {err}");
                    state.write().error = Some(format!("加载失败: {err}"));
                }
            }

            state.write().loading = false;
        }
    }));

    state
}

/// 图谱节点详情状态
#[derive(Clone, Debug, Default)]
pub struct GraphNodeState {
    pub loading: bool,
    pub error: Option<String>,
    pub node: Option<GraphNodeDetail>,
    pub edges: Vec<GraphEdgeDetail>,
}

/// 图谱节点详情 Hook
pub fn use_graph_node(node_id: String) -> Signal<GraphNodeState> {
    let state_store = use_app_state();
    let mut state = use_signal(GraphNodeState::default);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
        let node_id = node_id.clone();
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

            if node_id.is_empty() {
                return;
            }

            let Some(client) = API_CLIENT.get().cloned() else {
                state.write().error = Some("API 客户端未初始化".into());
                return;
            };

            state.write().loading = true;
            state.write().error = None;

            match client
                .get_graph_node::<GraphNodeDetail>(&tenant, &node_id)
                .await
            {
                Ok(env) => {
                    state.write().node = env.data;
                }
                Err(err) => {
                    tracing::error!("图谱节点加载失败: {err}");
                    state.write().error = Some(format!("加载失败: {err}"));
                }
            }

            state.write().loading = false;
        }
    }));

    state
}
