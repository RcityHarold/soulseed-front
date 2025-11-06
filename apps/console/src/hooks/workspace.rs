use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use std::collections::HashMap;

use reqwest::StatusCode;

use crate::api::ClientError;
use crate::models::{TenantWorkspace, TimelinePayload, WorkspaceSession};
use crate::state::{use_app_actions, use_app_state};
use crate::{API_CLIENT, APP_CONFIG};

pub fn use_workspace_overview() {
    let actions = use_app_actions();
    let state = use_app_state();

    use_future(move || {
        let actions = actions.clone();
        let state = state.clone();
        async move {
            if !state.read().workspace.tenants.is_empty() {
                return;
            }

            TimeoutFuture::new(0).await;

            actions.set_workspace_loading(true);
            actions.set_workspace_error(None);

            let tenant = {
                let snapshot = state.read();
                snapshot.tenant_id.clone().or_else(|| {
                    APP_CONFIG
                        .get()
                        .and_then(|cfg| cfg.default_tenant_id.clone())
                })
            };

            let Some(tenant_id) = tenant else {
                actions.set_workspace_error(Some("请在环境变量或界面中设置默认租户".into()));
                actions.set_workspace_loading(false);
                return;
            };

            let client = API_CLIENT.get().cloned();

            if let Some(client) = client {
                let mut query = crate::state::TimelineQuery::default();
                query.limit = 100;

                let mut sessions: HashMap<String, WorkspaceSession> = HashMap::new();

                match client
                    .get_timeline::<_, TimelinePayload>(&tenant_id, &query)
                    .await
                {
                    Ok(env) => {
                        if let Some(payload) = env.data {
                            for event in payload.items {
                                let session_id = event.session_id.to_string();
                                let scenario = event.scenario.clone();
                                let timestamp_ms = event.timestamp_ms;

                                let entry =
                                    sessions.entry(session_id.clone()).or_insert_with(|| {
                                        WorkspaceSession {
                                            session_id: session_id.clone(),
                                            title: None,
                                            scenario: Some(scenario.clone()),
                                            last_active_ms: Some(timestamp_ms),
                                            pinned: false,
                                            summary: None,
                                        }
                                    });

                                match entry.last_active_ms {
                                    Some(ts) if timestamp_ms <= ts => {}
                                    _ => {
                                        entry.last_active_ms = Some(timestamp_ms);
                                        entry.scenario = Some(scenario.clone());
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        let not_found = matches!(err, ClientError::EmptyResponse(status) if status == StatusCode::NOT_FOUND)
                            || matches!(err, ClientError::UnexpectedStatus { status, .. } if status == StatusCode::NOT_FOUND)
                            || err.status() == Some(StatusCode::NOT_FOUND);

                        if not_found {
                            tracing::warn!("workspace timeline unavailable: {err}");
                        } else {
                            tracing::error!("workspace timeline fetch failed: {err}");
                            actions.set_workspace_error(Some(format!(
                                "无法加载 Workspace 数据: {err}"
                            )));
                            actions.set_workspace_loading(false);
                            return;
                        }
                    }
                }

                let mut sessions_vec: Vec<WorkspaceSession> = sessions.into_values().collect();
                sessions_vec.sort_by_key(|sess| sess.last_active_ms.unwrap_or_default());
                sessions_vec.reverse();

                if let Some(first) = sessions_vec.first_mut() {
                    first.pinned = true;
                }

                let pinned_sessions = sessions_vec.iter().cloned().take(1).collect::<Vec<_>>();
                let recent_sessions = sessions_vec.clone();

                let workspace = TenantWorkspace {
                    tenant_id: tenant_id.clone(),
                    display_name: tenant_id.clone(),
                    description: None,
                    manifest_level: None,
                    clarify_strategy: None,
                    pinned_sessions,
                    recent_sessions,
                };

                actions.set_workspace_data(vec![workspace.clone()]);

                let needs_tenant_session = {
                    let snapshot = state.read();
                    (snapshot.tenant_id.is_none(), snapshot.session_id.is_none())
                };

                if needs_tenant_session.0 {
                    actions.set_tenant(Some(tenant_id.clone()));
                }

                if needs_tenant_session.1 {
                    if let Some(session) = workspace
                        .pinned_sessions
                        .first()
                        .or_else(|| workspace.recent_sessions.first())
                    {
                        actions.set_session(Some(session.session_id.clone()));
                    }
                }

                actions.set_workspace_loading(false);
                return;
            } else {
                actions.set_workspace_error(Some("Thin-Waist 客户端未初始化".into()));
            }

            actions.set_workspace_loading(false);
        }
    });
}
