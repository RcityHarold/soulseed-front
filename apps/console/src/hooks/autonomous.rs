//! 自主延续 Hooks
//!
//! 提供自主延续会话管理的 hooks，包括启动、监控、终止等

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::models::{
    AutonomousSessionListResponse, AutonomousSessionResponse, AutonomousSessionSummary,
    AutonomousStatus, ScenarioStackState, StartAutonomousRequest, TerminationResult,
};
use crate::state::use_app_state;
use crate::{API_CLIENT, APP_CONFIG};

/// 自主延续会话状态
#[derive(Clone, Debug, Default)]
pub struct AutonomousHookState {
    pub loading: bool,
    pub error: Option<String>,
    pub is_running: bool,
    /// 会话列表
    pub sessions: Vec<AutonomousSessionSummary>,
}

/// 自主延续会话 Hook
pub fn use_autonomous_session() -> Signal<AutonomousHookState> {
    let state_store = use_app_state();
    let mut state = use_signal(AutonomousHookState::default);

    let snapshot = state_store.read();
    let tenant_id = snapshot.tenant_id.clone();
    drop(snapshot);

    // 使用单个 use_future 同时处理初始加载和轮询
    use_future(move || {
        let tenant_id = tenant_id.clone();
        async move {
            loop {
                let tenant = tenant_id.clone().or_else(|| {
                    APP_CONFIG
                        .get()
                        .and_then(|cfg| cfg.default_tenant_id.clone())
                });

                let Some(tenant) = tenant else {
                    state.write().error = Some("请先选择租户".into());
                    TimeoutFuture::new(3000).await;
                    continue;
                };

                let Some(client) = API_CLIENT.get().cloned() else {
                    state.write().error = Some("API 客户端未初始化".into());
                    TimeoutFuture::new(3000).await;
                    continue;
                };

                // 首次加载时显示 loading
                if state.read().sessions.is_empty() {
                    state.write().loading = true;
                }

                // 获取会话列表
                match client
                    .get_autonomous_sessions::<AutonomousSessionListResponse>(&tenant, None, Some(20))
                    .await
                {
                    Ok(env) => {
                        if let Some(data) = env.data {
                            let has_running = data.sessions.iter().any(|s| {
                                s.status == AutonomousStatus::Running
                                    || s.status == AutonomousStatus::Starting
                            });
                            state.write().sessions = data.sessions;
                            state.write().is_running = has_running;
                            state.write().error = None;
                        }
                    }
                    Err(err) => {
                        tracing::error!("会话列表加载失败: {err}");
                        state.write().error = Some(format!("加载失败: {err}"));
                    }
                }

                state.write().loading = false;

                // 等待 3 秒后再次轮询
                TimeoutFuture::new(3000).await;
            }
        }
    });

    state
}

/// 自主延续控制结果
#[derive(Clone, Debug)]
pub enum AutonomousControlResult {
    Started(String),
    Terminated(TerminationResult),
    Error(String),
}

/// 自主延续控制 Hook 返回结构体
pub struct AutonomousControl {
    pub starting: Signal<bool>,
    pub terminating: Signal<bool>,
    pub last_result: Signal<Option<AutonomousControlResult>>,
    pub start: Box<dyn Fn(StartAutonomousRequest) + 'static>,
    pub terminate: Box<dyn Fn(String, String) + 'static>,
}

/// 自主延续控制 Hook (用于启动和终止)
pub fn use_autonomous_control() -> AutonomousControl {
    let starting = use_signal(|| false);
    let terminating = use_signal(|| false);
    let last_result = use_signal(|| None::<AutonomousControlResult>);

    let state = use_app_state();

    let start = {
        let mut starting = starting.clone();
        let mut last_result = last_result.clone();
        let state = state.clone();

        move |request: StartAutonomousRequest| {
            let mut starting = starting.clone();
            let mut last_result = last_result.clone();
            let state = state.clone();

            spawn(async move {
                starting.set(true);

                let snapshot = state.read();
                let tenant = snapshot.tenant_id.clone().or_else(|| {
                    APP_CONFIG
                        .get()
                        .and_then(|cfg| cfg.default_tenant_id.clone())
                });
                drop(snapshot);

                let Some(tenant) = tenant else {
                    last_result.set(Some(AutonomousControlResult::Error(
                        "请先选择租户".into(),
                    )));
                    starting.set(false);
                    return;
                };

                let Some(client) = API_CLIENT.get().cloned() else {
                    last_result.set(Some(AutonomousControlResult::Error(
                        "API 客户端未初始化".into(),
                    )));
                    starting.set(false);
                    return;
                };

                match client
                    .post_autonomous_start::<StartAutonomousRequest, AutonomousSessionResponse>(
                        &tenant, &request,
                    )
                    .await
                {
                    Ok(env) => {
                        if let Some(data) = env.data {
                            last_result.set(Some(AutonomousControlResult::Started(
                                data.orchestration_id,
                            )));
                        }
                    }
                    Err(err) => {
                        tracing::error!("启动自主延续失败: {err}");
                        last_result.set(Some(AutonomousControlResult::Error(format!(
                            "启动失败: {err}"
                        ))));
                    }
                }

                starting.set(false);
            });
        }
    };

    let terminate = {
        let mut terminating = terminating.clone();
        let mut last_result = last_result.clone();
        let state = state.clone();

        move |orchestration_id: String, reason: String| {
            let mut terminating = terminating.clone();
            let mut last_result = last_result.clone();
            let state = state.clone();

            spawn(async move {
                terminating.set(true);

                let snapshot = state.read();
                let tenant = snapshot.tenant_id.clone().or_else(|| {
                    APP_CONFIG
                        .get()
                        .and_then(|cfg| cfg.default_tenant_id.clone())
                });
                drop(snapshot);

                let Some(tenant) = tenant else {
                    last_result.set(Some(AutonomousControlResult::Error(
                        "请先选择租户".into(),
                    )));
                    terminating.set(false);
                    return;
                };

                let Some(client) = API_CLIENT.get().cloned() else {
                    last_result.set(Some(AutonomousControlResult::Error(
                        "API 客户端未初始化".into(),
                    )));
                    terminating.set(false);
                    return;
                };

                #[derive(serde::Serialize)]
                struct TerminateRequest {
                    reason: String,
                }

                match client
                    .post_autonomous_terminate::<TerminateRequest, TerminationResult>(
                        &tenant,
                        &orchestration_id,
                        &TerminateRequest { reason },
                    )
                    .await
                {
                    Ok(env) => {
                        if let Some(data) = env.data {
                            last_result.set(Some(AutonomousControlResult::Terminated(data)));
                        }
                    }
                    Err(err) => {
                        tracing::error!("终止自主延续失败: {err}");
                        last_result.set(Some(AutonomousControlResult::Error(format!(
                            "终止失败: {err}"
                        ))));
                    }
                }

                terminating.set(false);
            });
        }
    };

    AutonomousControl {
        starting,
        terminating,
        last_result,
        start: Box::new(start),
        terminate: Box::new(terminate),
    }
}

/// 场景栈 Hook
pub fn use_scenario_stack() -> Signal<Option<ScenarioStackState>> {
    let state_store = use_app_state();
    let mut stack = use_signal(|| None::<ScenarioStackState>);

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

            let Some(sid) = session_id.clone() else {
                stack.set(None);
                return;
            };

            let Some(client) = API_CLIENT.get().cloned() else {
                return;
            };

            match client
                .get_autonomous_scenario_stack::<ScenarioStackState>(&tenant, &sid)
                .await
            {
                Ok(env) => {
                    stack.set(env.data);
                }
                Err(err) => {
                    tracing::error!("场景栈加载失败: {err}");
                }
            }
        }
    }));

    stack
}
