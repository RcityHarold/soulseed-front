//! 演化事件 Hooks
//!
//! 提供群体演化、个体 AI 演化、关系演化等事件的 hooks

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::{
    AiEvolutionListResponse, EvolutionTimelineResponse, GroupEvolutionListResponse,
    RelationshipEvolutionListResponse,
};
use crate::state::use_app_state;
use crate::{API_CLIENT, APP_CONFIG};

/// 演化事件综合状态
#[derive(Clone, Debug, Default)]
pub struct EvolutionState {
    pub loading: bool,
    pub error: Option<String>,
    pub group_events: Option<GroupEvolutionListResponse>,
    pub ai_events: Option<AiEvolutionListResponse>,
    pub relationship_events: Option<RelationshipEvolutionListResponse>,
    pub timeline: Option<EvolutionTimelineResponse>,
}

/// 群体演化事件 Hook
pub fn use_group_evolution() -> Signal<Option<GroupEvolutionListResponse>> {
    let state_store = use_app_state();
    let mut events = use_signal(|| None::<GroupEvolutionListResponse>);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
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
            struct GroupQuery {
                limit: Option<u32>,
            }

            let query = GroupQuery { limit: Some(50) };

            match client
                .get_evolution_group::<GroupQuery, GroupEvolutionListResponse>(&tenant, Some(&query))
                .await
            {
                Ok(env) => {
                    events.set(env.data);
                }
                Err(err) => {
                    tracing::error!("群体演化事件加载失败: {err}");
                }
            }
        }
    }));

    events
}

/// AI 演化事件 Hook
pub fn use_ai_evolution(ai_id: String) -> Signal<Option<AiEvolutionListResponse>> {
    let state_store = use_app_state();
    let mut events = use_signal(|| None::<AiEvolutionListResponse>);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
        let ai_id = ai_id.clone();
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

            if ai_id.is_empty() {
                return;
            }

            let Some(client) = API_CLIENT.get().cloned() else {
                return;
            };

            match client
                .get_evolution_ai::<(), AiEvolutionListResponse>(&tenant, &ai_id, None)
                .await
            {
                Ok(env) => {
                    events.set(env.data);
                }
                Err(err) => {
                    tracing::error!("AI 演化事件加载失败: {err}");
                }
            }
        }
    }));

    events
}

/// 关系演化事件 Hook
pub fn use_relationship_evolution() -> Signal<Option<RelationshipEvolutionListResponse>> {
    let state_store = use_app_state();
    let mut events = use_signal(|| None::<RelationshipEvolutionListResponse>);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
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

            match client
                .get_evolution_relationships::<(), RelationshipEvolutionListResponse>(&tenant, None)
                .await
            {
                Ok(env) => {
                    events.set(env.data);
                }
                Err(err) => {
                    tracing::error!("关系演化事件加载失败: {err}");
                }
            }
        }
    }));

    events
}

/// 综合演化状态 Hook
pub fn use_evolution_overview() -> Signal<EvolutionState> {
    let state_store = use_app_state();
    let mut state = use_signal(EvolutionState::default);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
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

            // 加载群体演化事件
            #[derive(serde::Serialize)]
            struct GroupQuery {
                limit: Option<u32>,
            }

            let query = GroupQuery { limit: Some(20) };

            match client
                .get_evolution_group::<GroupQuery, GroupEvolutionListResponse>(&tenant, Some(&query))
                .await
            {
                Ok(env) => {
                    state.write().group_events = env.data;
                }
                Err(err) => {
                    tracing::warn!("群体演化事件加载失败: {err}");
                }
            }

            // 加载关系演化事件
            match client
                .get_evolution_relationships::<(), RelationshipEvolutionListResponse>(&tenant, None)
                .await
            {
                Ok(env) => {
                    state.write().relationship_events = env.data;
                }
                Err(err) => {
                    tracing::warn!("关系演化事件加载失败: {err}");
                }
            }

            state.write().loading = false;
        }
    }));

    state
}
