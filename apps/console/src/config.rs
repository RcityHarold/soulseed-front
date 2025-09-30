use std::time::Duration;

use serde::{Deserialize, Serialize};

const DEFAULT_API_BASE_URL: &str = "http://localhost:8700/api/v1";
const DEFAULT_SSE_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 15;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppProfile {
    Dev,
    Prod,
}

impl AppProfile {
    pub fn from_env(value: Option<String>) -> Self {
        match value.as_deref() {
            Some("prod") | Some("production") => Self::Prod,
            _ => Self::Dev,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_base_url: String,
    pub stream_base_url: Option<String>,
    pub default_tenant_id: Option<String>,
    pub auth_token: Option<String>,
    pub profile: AppProfile,
    pub sse_timeout: Duration,
    pub request_timeout: Duration,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_base_url: DEFAULT_API_BASE_URL.to_string(),
            stream_base_url: None,
            default_tenant_id: None,
            auth_token: None,
            profile: AppProfile::Dev,
            sse_timeout: Duration::from_millis(DEFAULT_SSE_TIMEOUT_MS),
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        crate::config::load_dotenv();

        let mut config = Self::default();

        if let Some(url) = read_env("SOULSEED_API_BASE_URL") {
            config.api_base_url = url;
        }

        if let Some(stream_url) = read_env("SOULSEED_STREAM_BASE_URL") {
            config.stream_base_url = Some(stream_url);
        }

        if let Some(tenant) = read_env("SOULSEED_DEFAULT_TENANT") {
            config.default_tenant_id = Some(tenant);
        }

        if let Some(token) = read_env("SOULSEED_AUTH_TOKEN") {
            config.auth_token = Some(token);
        }

        let profile_raw = read_env("SOULSEED_PROFILE");
        config.profile = AppProfile::from_env(profile_raw);

        if let Some(ms) =
            read_env("SOULSEED_SSE_TIMEOUT_MS").and_then(|value| value.parse::<u64>().ok())
        {
            config.sse_timeout = Duration::from_millis(ms.max(1_000));
        }

        if let Some(secs) =
            read_env("SOULSEED_REQUEST_TIMEOUT_SECS").and_then(|value| value.parse::<u64>().ok())
        {
            config.request_timeout = Duration::from_secs(secs.max(1));
        }

        config
    }

    pub fn bearer_token(&self) -> Option<String> {
        self.auth_token
            .as_ref()
            .map(|token| format!("Bearer {}", token.trim()))
    }

    pub fn tenant_header<'a>(&'a self, override_tenant: Option<&'a str>) -> Option<String> {
        override_tenant
            .or_else(|| self.default_tenant_id.as_deref())
            .map(|value| value.to_string())
    }

    pub fn stream_endpoint(&self) -> String {
        self.stream_base_url
            .clone()
            .unwrap_or_else(|| self.api_base_url.clone())
    }
}

fn read_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .or_else(|| option_env_from_build(key).map(|s| s.to_string()))
}

fn option_env_from_build(key: &str) -> Option<&'static str> {
    match key {
        "SOULSEED_API_BASE_URL" => option_env!("SOULSEED_API_BASE_URL"),
        "SOULSEED_STREAM_BASE_URL" => option_env!("SOULSEED_STREAM_BASE_URL"),
        "SOULSEED_DEFAULT_TENANT" => option_env!("SOULSEED_DEFAULT_TENANT"),
        "SOULSEED_AUTH_TOKEN" => option_env!("SOULSEED_AUTH_TOKEN"),
        "SOULSEED_PROFILE" => option_env!("SOULSEED_PROFILE"),
        "SOULSEED_SSE_TIMEOUT_MS" => option_env!("SOULSEED_SSE_TIMEOUT_MS"),
        "SOULSEED_REQUEST_TIMEOUT_SECS" => option_env!("SOULSEED_REQUEST_TIMEOUT_SECS"),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_dotenv() {
    if let Err(err) = dotenvy::dotenv() {
        if !matches!(err, dotenvy::Error::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound)
        {
            tracing::warn!("failed to load .env: {err}");
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn load_dotenv() {}
