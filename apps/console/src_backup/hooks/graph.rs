use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde::Serialize;

use crate::models::CausalGraphView;
use crate::state::{use_app_actions, use_app_state};
use crate::{API_CLIENT, APP_CONFIG};

pub fn use_graph_insights() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    let root = snapshot.graph.query.root_event_id;
    drop(snapshot);

    use_future(use_reactive!(|(tenant, session, root)| {
        let actions = actions.clone();
        async move {
            tracing::info!(
                "graph loader triggered: tenant={:?}, session={:?}, root={:?}",
                tenant,
                session,
                root
            );
            if tenant.is_none() || session.is_none() {
                actions.set_graph_data(None, Vec::new());
                return;
            }

            let Some(root_id) = root else {
                actions.set_graph_data(None, Vec::new());
                return;
            };

            TimeoutFuture::new(0).await;

            actions.set_graph_loading(true);
            actions.set_graph_error(None);

            #[derive(Serialize)]
            struct CausalQuery<'a> {
                root_event_id: u64,
                #[serde(rename = "direction")]
                direction: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                depth: Option<u8>,
                #[serde(skip_serializing_if = "Option::is_none")]
                scenario: Option<&'a soulseed_agi_core_models::ConversationScenario>,
            }

            let tenant_id = tenant.clone().or_else(|| {
                APP_CONFIG
                    .get()
                    .and_then(|cfg| cfg.default_tenant_id.clone())
            });

            let Some(tenant_id) = tenant_id else {
                actions.set_graph_error(Some("请先选择租户".into()));
                actions.set_graph_loading(false);
                return;
            };

            let (depth, scenario_filter) = {
                let snapshot = state.read();
                let depth = if snapshot.graph.query.depth == 0 {
                    Some(3)
                } else {
                    Some(snapshot.graph.query.depth)
                };
                (depth, snapshot.scenario_filter.clone())
            };

            let query = CausalQuery {
                root_event_id: root_id,
                direction: "both",
                depth,
                scenario: scenario_filter.as_ref(),
            };

            let client = API_CLIENT.get().cloned();

            if let Some(client) = client {
                match client
                    .get_causal_graph::<_, CausalGraphView>(&tenant_id, &query)
                    .await
                {
                    Ok(env) => {
                        if let Some(causal) = env.data {
                            actions.set_graph_data(Some(causal), Vec::new());
                        } else {
                            actions.set_graph_error(Some("因果链路返回空数据".into()));
                            actions.set_graph_loading(false);
                        }
                    }
                    Err(err) => {
                        tracing::error!("causal graph fetch failed: {err}");
                        actions.set_graph_error(Some(format!("因果链加载失败: {err}")));
                        actions.set_graph_loading(false);
                    }
                }
            } else {
                actions.set_graph_error(Some("Thin-Waist 客户端未初始化".into()));
                actions.set_graph_loading(false);
            }
        }
    }));
}
