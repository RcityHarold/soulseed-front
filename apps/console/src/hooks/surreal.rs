//! SurrealDB 原生功能 Hooks
//!
//! 提供向量搜索、时序聚合、实时订阅等 hooks

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::{
    IndexContentRequest, IndexContentResponse, TimeSeriesAggregateResponse, VectorSearchResponse,
};
use crate::state::use_app_state;
use crate::{API_CLIENT, APP_CONFIG};

/// 向量搜索状态
#[derive(Clone, Debug, Default)]
pub struct VectorSearchState {
    pub searching: bool,
    pub error: Option<String>,
    pub result: Option<VectorSearchResponse>,
}

/// 向量搜索 Hook
pub fn use_vector_search() -> VectorSearcher {
    let searching = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let result = use_signal(|| None::<VectorSearchResponse>);

    VectorSearcher {
        searching,
        error,
        result,
    }
}

/// 向量搜索器
#[derive(Clone, Copy)]
pub struct VectorSearcher {
    pub searching: Signal<bool>,
    pub error: Signal<Option<String>>,
    pub result: Signal<Option<VectorSearchResponse>>,
}

impl VectorSearcher {
    /// 执行向量搜索
    pub async fn search(&mut self, query: String, tenant_id: Option<String>, top_k: Option<u32>) {
        if query.trim().is_empty() {
            self.error.set(Some("请输入搜索内容".into()));
            return;
        }

        let tenant = tenant_id.or_else(|| {
            APP_CONFIG
                .get()
                .and_then(|cfg| cfg.default_tenant_id.clone())
        });

        let Some(tenant) = tenant else {
            self.error.set(Some("请先选择租户".into()));
            return;
        };

        let Some(client) = API_CLIENT.get().cloned() else {
            self.error.set(Some("API 客户端未初始化".into()));
            return;
        };

        self.searching.set(true);
        self.error.set(None);

        #[derive(serde::Serialize)]
        struct VectorSearchRequest {
            query: String,
            top_k: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            filter: Option<serde_json::Value>,
        }

        let request = VectorSearchRequest {
            query,
            top_k: top_k.unwrap_or(10),
            filter: None,
        };

        match client
            .post_surreal_vector_search::<VectorSearchRequest, VectorSearchResponse>(&tenant, &request)
            .await
        {
            Ok(env) => {
                self.result.set(env.data);
            }
            Err(err) => {
                tracing::error!("向量搜索失败: {err}");
                self.error.set(Some(format!("搜索失败: {err}")));
            }
        }

        self.searching.set(false);
    }
}

/// 时序聚合状态
#[derive(Clone, Debug, Default)]
pub struct TimeSeriesState {
    pub loading: bool,
    pub error: Option<String>,
    pub data: Option<TimeSeriesAggregateResponse>,
}

/// 时序聚合 Hook
pub fn use_timeseries_aggregate(
    metric: String,
    aggregation: String,
    interval: String,
) -> Signal<TimeSeriesState> {
    let state_store = use_app_state();
    let mut state = use_signal(TimeSeriesState::default);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant_id,)| {
        let metric = metric.clone();
        let aggregation = aggregation.clone();
        let interval = interval.clone();
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

            #[derive(serde::Serialize)]
            struct TimeSeriesQuery {
                metric: String,
                aggregation: String,
                interval: String,
            }

            let query = TimeSeriesQuery {
                metric,
                aggregation,
                interval,
            };

            match client
                .get_surreal_timeseries_aggregate::<TimeSeriesQuery, TimeSeriesAggregateResponse>(
                    &tenant, &query,
                )
                .await
            {
                Ok(env) => {
                    state.write().data = env.data;
                }
                Err(err) => {
                    tracing::error!("时序聚合加载失败: {err}");
                    state.write().error = Some(format!("加载失败: {err}"));
                }
            }

            state.write().loading = false;
        }
    }));

    state
}

/// 实时订阅状态
#[derive(Clone, Debug, Default)]
pub struct LiveSubscriptionState {
    pub connected: bool,
    pub events: Vec<serde_json::Value>,
}

/// 实时订阅 Hook
pub fn use_live_subscription() -> LiveSubscriptionManager {
    let connected = use_signal(|| false);
    let events = use_signal(Vec::<serde_json::Value>::new);

    LiveSubscriptionManager { connected, events }
}

/// 实时订阅管理器
pub struct LiveSubscriptionManager {
    pub connected: Signal<bool>,
    pub events: Signal<Vec<serde_json::Value>>,
}

// ============================================================================
// 内容索引 Hook
// ============================================================================

/// 内容索引状态
#[derive(Clone, Debug, Default)]
pub struct ContentIndexState {
    pub indexing: bool,
    pub error: Option<String>,
    pub last_result: Option<IndexContentResponse>,
}

/// 内容索引 Hook
pub fn use_content_indexer() -> ContentIndexer {
    let indexing = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let last_result = use_signal(|| None::<IndexContentResponse>);

    ContentIndexer {
        indexing,
        error,
        last_result,
    }
}

/// 内容索引器
#[derive(Clone, Copy)]
pub struct ContentIndexer {
    pub indexing: Signal<bool>,
    pub error: Signal<Option<String>>,
    pub last_result: Signal<Option<IndexContentResponse>>,
}

impl ContentIndexer {
    /// 索引单个内容
    pub async fn index_content(
        &mut self,
        content: String,
        source_type: String,
        source_id: String,
        tenant_id: Option<String>,
    ) {
        if content.trim().len() < 10 {
            self.error.set(Some("内容太短，至少需要10个字符".into()));
            return;
        }

        let tenant = tenant_id.or_else(|| {
            APP_CONFIG
                .get()
                .and_then(|cfg| cfg.default_tenant_id.clone())
        });

        let Some(tenant) = tenant else {
            self.error.set(Some("请先选择租户".into()));
            return;
        };

        let Some(client) = API_CLIENT.get().cloned() else {
            self.error.set(Some("API 客户端未初始化".into()));
            return;
        };

        self.indexing.set(true);
        self.error.set(None);

        let request = IndexContentRequest {
            content,
            source_type,
            source_id,
            journey_id: None,
            session_id: None,
            metadata: None,
        };

        match client
            .post_surreal_index_content::<IndexContentRequest, IndexContentResponse>(
                &tenant, &request,
            )
            .await
        {
            Ok(env) => {
                self.last_result.set(env.data);
            }
            Err(err) => {
                tracing::error!("内容索引失败: {err}");
                self.error.set(Some(format!("索引失败: {err}")));
            }
        }

        self.indexing.set(false);
    }
}
