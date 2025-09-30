use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use reqwest::{header, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::config::AppConfig;

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Clone)]
pub struct ThinWaistClient {
    inner: reqwest::Client,
    config: Arc<AppConfig>,
    base_url: String,
}

impl ThinWaistClient {
    pub fn new(config: AppConfig) -> ClientResult<Self> {
        let timeout = config.request_timeout;
        let base_url = normalize_base_url(&config.api_base_url);

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build reqwest client")?;

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

    pub async fn post_ace_cycle<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/ace/cycles");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
    }

    pub async fn get_ace_cycle<TRes>(
        &self,
        tenant_id: &str,
        cycle_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/ace/cycles/{cycle_id}");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    pub async fn get_ace_outbox<TRes>(
        &self,
        tenant_id: &str,
        cycle_id: &str,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/ace/cycles/{cycle_id}/outbox");
        let builder = self.request(Method::GET, &path, Some(tenant_id))?;
        self.send(builder).await
    }

    pub async fn post_ace_injection<TReq, TRes>(
        &self,
        tenant_id: &str,
        payload: &TReq,
    ) -> ClientResult<ApiEnvelope<TRes>>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let path = format!("tenants/{tenant_id}/ace/injections");
        let builder = self
            .request(Method::POST, &path, Some(tenant_id))?
            .json(payload);
        self.send(builder).await
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

    pub fn build_live_stream_request(
        &self,
        tenant_id: &str,
        session_id: &str,
    ) -> ClientResult<reqwest::Request> {
        let path = format!("tenants/{tenant_id}/live/dialogues/{session_id}");
        let builder = self
            .request(Method::GET, &path, Some(tenant_id))?
            .header(header::ACCEPT, "text/event-stream");
        builder.build().context("failed to build SSE request")
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiEnvelope<T> {
    pub success: bool,
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub error: Option<ApiErrorBody>,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
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

impl ApiErrorBody {
    fn with_status(mut self, status: StatusCode) -> Self {
        self.status = Some(status);
        self
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
