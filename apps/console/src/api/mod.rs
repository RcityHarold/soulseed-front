use reqwest::{header, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

use crate::config::AppConfig;
use crate::models::CausalChainQuery;

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Serialize)]
pub struct AwarenessQuery {
    pub limit: u32,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ThinWaistClient {
    inner: reqwest::Client,
    config: Arc<AppConfig>,
    base_url: String,
}

#[allow(dead_code)]
impl ThinWaistClient {
    pub fn new(config: AppConfig) -> ClientResult<Self> {
        let base_url = normalize_base_url(&config.api_base_url);

        #[cfg(target_arch = "wasm32")]
        let builder = reqwest::Client::builder();

        #[cfg(not(target_arch = "wasm32"))]
        let builder = reqwest::Client::builder().timeout(config.request_timeout);

        let client = builder.build().map_err(ClientError::from)?;

        Ok(Self {
            inner: client,
            config: Arc::new(config),
            base_url,
        })
    }

    pub fn config(&self) -> Arc<AppConfig> {
        Arc::clone(&self.config)
    }

    pub async fn post_dialogue_event<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/dialogue-events");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    pub async fn get_dialogue_event<TRes>(
        &self,
        tenant_id: &str,
        event_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/dialogue-events/{event_id}");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    pub async fn get_timeline<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: &TQuery,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/graph/timeline");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .query(query);
        self.send(builder).await
    }

    pub async fn get_causal_graph<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: &TQuery,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/graph/causal");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .query(query);
        self.send(builder).await
    }

    pub async fn get_recall<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: &TQuery,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/graph/recall");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .query(query);
        self.send(builder).await
    }

    pub async fn get_context_bundle<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: Option<&TQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/context/bundle");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(query) = query {
            builder = builder.query(query);
        }
        self.send(builder).await
    }

    pub async fn post_context_compact<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/context/manifest/compact");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    pub async fn post_trigger_dialogue<TReq, TRes>(
        &self,
        payload: &TReq,
        tenant_override: Option<&str>,
    ) -> ClientResult<TRes>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let builder = self
            .request(Method::POST, "triggers/dialogue", tenant_override)?
            .json(payload);
        self.send_plain(builder).await
    }

    pub async fn get_cycle_snapshot<TRes>(
        &self,
        cycle_id: &str,
        tenant_override: Option<&str>,
    ) -> ClientResult<TRes>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("ace/cycles/{cycle_id}");
        let builder = self.request(Method::GET, &path, tenant_override)?;
        self.send_plain(builder).await
    }

    pub async fn get_cycle_outbox<TRes>(
        &self,
        cycle_id: &str,
        tenant_override: Option<&str>,
    ) -> ClientResult<TRes>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("ace/cycles/{cycle_id}/outbox");
        let builder = self.request(Method::GET, &path, tenant_override)?;
        self.send_plain(builder).await
    }

    pub async fn post_cycle_injection<TReq, TRes>(
        &self,
        payload: &TReq,
        tenant_override: Option<&str>,
    ) -> ClientResult<TRes>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let builder = self
            .request(Method::POST, "ace/injections", tenant_override)?
            .json(payload);
        self.send_plain(builder).await
    }

    pub async fn get_awareness_events<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: &TQuery,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/awareness/events");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .query(query);
        self.send(builder).await
    }

    pub async fn get_explain_indices<TRes>(
        &self,
        tenant_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/explain/indices");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    // ========================================================================
    // 元认知分析 API (Metacognition)
    // ========================================================================

    /// 获取元认知分析结果
    pub async fn get_metacognition_analysis<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: &TQuery,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/metacognition/analysis");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .query(query);
        self.send(builder).await
    }

    /// 获取因果推理链
    pub async fn get_metacognition_causal_chain<TRes>(
        &self,
        tenant_id: &str,
        event_id: &str,
        query: Option<&CausalChainQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/metacognition/events/{event_id}/causal-chain");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(q) = query {
            builder = builder.query(q);
        }
        self.send(builder).await
    }

    /// 获取性能画像
    pub async fn get_metacognition_performance_profile<TRes>(
        &self,
        tenant_id: &str,
        ac_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/metacognition/ac/{ac_id}/performance");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 获取决策审计记录
    pub async fn get_metacognition_decision_audit<TRes>(
        &self,
        tenant_id: &str,
        decision_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/metacognition/decisions/{decision_id}/audit");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 检测元认知模式
    pub async fn get_metacognition_patterns<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: Option<&TQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/metacognition/patterns");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(q) = query {
            builder = builder.query(q);
        }
        self.send(builder).await
    }

    // ========================================================================
    // 自主延续 API (Autonomous)
    // ========================================================================

    /// 获取自主延续会话列表
    pub async fn get_autonomous_sessions<TRes>(
        &self,
        tenant_id: &str,
        status: Option<&str>,
        limit: Option<usize>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let mut path = format!("tenants/{tenant_id}/autonomous/sessions");
        let mut params = vec![];
        if let Some(s) = status {
            params.push(format!("status={}", s));
        }
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if !params.is_empty() {
            path = format!("{}?{}", path, params.join("&"));
        }
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 启动自主延续会话
    pub async fn post_autonomous_start<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/autonomous/sessions");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    /// 获取自主延续会话状态
    pub async fn get_autonomous_session<TRes>(
        &self,
        tenant_id: &str,
        session_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/autonomous/sessions/{session_id}");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 终止自主延续会话
    pub async fn post_autonomous_terminate<TReq, TRes>(
        &self,
        tenant_id: &str,
        session_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/autonomous/sessions/{session_id}/stop");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    /// 获取场景栈状态
    pub async fn get_autonomous_scenario_stack<TRes>(
        &self,
        tenant_id: &str,
        session_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/autonomous/sessions/{session_id}/scenarios");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    // ========================================================================
    // 版本链 API (Version Chain)
    // ========================================================================

    /// 获取版本链摘要
    pub async fn get_version_chain_summary<TRes>(
        &self,
        tenant_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/version-chain/{entity_id}?entity_type={entity_type}");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 获取版本差异（暂未实现后端）
    pub async fn get_version_diff<TRes>(
        &self,
        tenant_id: &str,
        entity_type: &str,
        entity_id: &str,
        from_version: u32,
        to_version: u32,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!(
            "tenants/{tenant_id}/version-chain/{entity_id}/diff?entity_type={entity_type}&from={from_version}&to={to_version}"
        );
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 获取图谱节点详情
    pub async fn get_graph_node<TRes>(
        &self,
        tenant_id: &str,
        node_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/graph/nodes/{node_id}");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 获取图谱边详情
    pub async fn get_graph_edges<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: Option<&TQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/graph/edges");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(q) = query {
            builder = builder.query(q);
        }
        self.send(builder).await
    }

    // ========================================================================
    // DFR 决策增强 API
    // ========================================================================

    /// 获取决策详情
    pub async fn get_dfr_decision<TRes>(
        &self,
        tenant_id: &str,
        decision_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/dfr/decisions/{decision_id}");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    /// 获取决策指纹列表
    pub async fn get_dfr_fingerprints<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: Option<&TQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/dfr/fingerprints");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(q) = query {
            builder = builder.query(q);
        }
        self.send(builder).await
    }

    /// 匹配决策指纹
    pub async fn post_dfr_match_fingerprint<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/dfr/fingerprints/match");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    // ========================================================================
    // SurrealDB 原生功能 API
    // ========================================================================

    /// 向量搜索
    pub async fn post_surreal_vector_search<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/search/vector");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    /// 时序聚合查询
    pub async fn get_surreal_timeseries_aggregate<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: &TQuery,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/timeseries/aggregate");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .query(query);
        self.send(builder).await
    }

    /// 创建实时订阅
    pub async fn post_surreal_subscribe<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/realtime/subscribe");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    /// 取消实时订阅
    pub async fn delete_surreal_subscription<TRes>(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/realtime/subscriptions/{subscription_id}");
        let builder = self.request(Method::DELETE, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    // ========================================================================
    // 演化事件 API (Evolution)
    // ========================================================================

    /// 获取群体演化事件列表
    pub async fn get_evolution_group<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: Option<&TQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/evolution/groups");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(q) = query {
            builder = builder.query(q);
        }
        self.send(builder).await
    }

    /// 获取个体 AI 演化事件
    pub async fn get_evolution_ai<TQuery, TRes>(
        &self,
        tenant_id: &str,
        ai_id: &str,
        query: Option<&TQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/evolution/ai/{ai_id}");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(q) = query {
            builder = builder.query(q);
        }
        self.send(builder).await
    }

    /// 获取关系演化事件
    pub async fn get_evolution_relationships<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: Option<&TQuery>,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/evolution/relationships");
        let mut builder = self.request(Method::GET, &path, Some(tenant_id))?;
        if let Some(q) = query {
            builder = builder.query(q);
        }
        self.send(builder).await
    }

    /// 获取演化时间线
    pub async fn get_evolution_timeline<TQuery, TRes>(
        &self,
        tenant_id: &str,
        query: &TQuery,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TQuery: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/evolution/timeline");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .query(query);
        self.send(builder).await
    }

    pub fn build_live_stream_request(
        &self,
        tenant_id: &str,
        session_id: &str,
    ) -> ClientResult<reqwest::Request> {
        let path = format!("tenants/{tenant_id}/live/dialogues/{session_id}");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .header(header::ACCEPT, "text/event-stream");
        builder.build().map_err(ClientError::from)
    }

    pub fn build_cycle_stream_request(
        &self,
        cycle_id: &str,
        tenant_override: Option<&str>,
    ) -> ClientResult<reqwest::Request> {
        let builder = self
            .request(
                Method::GET,
                &format!("ace/cycles/{cycle_id}/stream"),
                tenant_override,
            )?
            .header(header::ACCEPT, "text/event-stream");
        builder.build().map_err(ClientError::from)
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        tenant_override: Option<&str>,
    ) -> ClientResult<reqwest::RequestBuilder> {
        let url = self.join_path(path);
        let mut builder = self.inner.request(method, url);

        if let Some(token) = self.config.bearer_token() {
            builder = builder.header(header::AUTHORIZATION, token);
        }

        if let Some(tenant) = self.config.tenant_header(tenant_override) {
            builder = builder.header("X-Tenant-Id", tenant);
        }

        Ok(builder)
    }

    fn join_path(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    async fn send_plain<T>(&self, builder: reqwest::RequestBuilder) -> ClientResult<T>
    where
        T: DeserializeOwned,
    {
        let response = builder.send().await.map_err(ClientError::from)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(ClientError::from)?;

        if bytes.is_empty() {
            return Err(ClientError::EmptyResponse(status));
        }

        if status.is_success() {
            serde_json::from_slice(&bytes).map_err(ClientError::from)
        } else {
            Err(map_plain_error(status, &bytes))
        }
    }

    async fn send<T>(&self, builder: reqwest::RequestBuilder) -> ClientResult<ApiEnvelope<T>>
    where
        T: DeserializeOwned,
    {
        let response = builder.send().await.map_err(ClientError::from)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(ClientError::from)?;

        if bytes.is_empty() {
            return Err(ClientError::EmptyResponse(status));
        }

        let envelope: ApiEnvelope<T> = serde_json::from_slice(&bytes).map_err(ClientError::from)?;

        if status.is_success() && envelope.success {
            Ok(envelope)
        } else if let Some(err) = envelope.error.clone() {
            Err(ClientError::Api(err.with_status(status)))
        } else {
            Err(ClientError::UnexpectedStatus {
                status,
                body: bytes.to_vec(),
            })
        }
    }
}

fn normalize_base_url(input: &str) -> String {
    input.trim_end_matches('/').to_string()
}

#[derive(Deserialize)]
struct PlainAceError {
    error: String,
}

fn map_plain_error(status: StatusCode, bytes: &[u8]) -> ClientError {
    if let Ok(body) = serde_json::from_slice::<ApiErrorBody>(bytes) {
        return ClientError::Api(body.with_status(status));
    }

    if let Ok(wrapper) = serde_json::from_slice::<PlainAceError>(bytes) {
        return ClientError::Api(ApiErrorBody {
            code: "ace_error".into(),
            message: wrapper.error,
            details: None,
            status: Some(status),
        });
    }

    ClientError::UnexpectedStatus {
        status,
        body: bytes.to_vec(),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiEnvelope<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiErrorBody>,
    pub trace_id: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<Value>,
    #[serde(skip)]
    pub status: Option<StatusCode>,
}

#[allow(dead_code)]
impl ApiErrorBody {
    fn with_status(mut self, status: StatusCode) -> Self {
        self.status = Some(status);
        self
    }
}

impl std::fmt::Display for ApiErrorBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.status {
            Some(status) => write!(f, "{} {}: {}", status, self.code, self.message),
            None => write!(f, "{}: {}", self.code, self.message),
        }
    }
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("api error: {0}")]
    Api(ApiErrorBody),
    #[error("empty response body: {0}")]
    EmptyResponse(StatusCode),
    #[error("unexpected status {status}: {body:?}")]
    UnexpectedStatus { status: StatusCode, body: Vec<u8> },
}

#[allow(dead_code)]
impl ClientError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Api(body) => body.status,
            Self::EmptyResponse(status) => Some(*status),
            Self::UnexpectedStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn trace_context(&self) -> Option<&Value> {
        match self {
            Self::Api(body) => body.details.as_ref(),
            _ => None,
        }
    }
}
