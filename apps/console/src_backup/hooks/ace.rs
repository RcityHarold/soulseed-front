use std::collections::HashMap;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde::Serialize;

use crate::models::{AceCycleStatus, AceCycleSummary, AceLane, AwarenessEvent, AwarenessEventType};
use crate::state::{use_app_actions, use_app_state};
use crate::{API_CLIENT, APP_CONFIG};

pub fn use_ace_cycles() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    drop(snapshot);

    use_future(use_reactive!(|(tenant, session)| {
        let actions = actions.clone();
        async move {
            tracing::info!(
                "ACE loader triggered: tenant={:?}, session={:?}",
                tenant,
                session
            );
            if tenant.is_none() || session.is_none() {
                actions.set_ace_cycles(Vec::new());
                actions.select_ace_cycle(None);
                return;
            }

            TimeoutFuture::new(0).await;

            actions.set_ace_loading(true);
            actions.set_ace_error(None);

            #[derive(Serialize)]
            struct AwarenessQuery {
                limit: u32,
            }

            let tenant_id = tenant.clone().or_else(|| {
                APP_CONFIG
                    .get()
                    .and_then(|cfg| cfg.default_tenant_id.clone())
            });

            let Some(tenant_id) = tenant_id else {
                actions.set_ace_error(Some("请先选择租户".into()));
                actions.set_ace_loading(false);
                return;
            };

            let client = API_CLIENT.get().cloned();

            if let Some(client) = client {
                match client
                    .get_awareness_events::<_, Vec<AwarenessEvent>>(
                        &tenant_id,
                        &AwarenessQuery { limit: 200 },
                    )
                    .await
                {
                    Ok(env) => {
                        if let Some(events) = env.data {
                            let mut grouped: HashMap<String, Vec<AwarenessEvent>> = HashMap::new();
                            for event in events {
                                let cycle_id = event.awareness_cycle_id.to_string();
                                grouped.entry(cycle_id).or_default().push(event);
                            }

                            let mut summaries_with_ts: Vec<(i64, AceCycleSummary)> = grouped
                                .into_iter()
                                .map(|(cycle_id, mut items)| {
                                    items.sort_by_key(|evt| evt.occurred_at_ms);
                                    let latest_ts = items
                                        .iter()
                                        .map(|evt| evt.occurred_at_ms)
                                        .max()
                                        .unwrap_or_default();
                                    let anchor = items.first().map(|evt| evt.anchor.clone());
                                    let lane = detect_lane(&items);
                                    let status = detect_status(&items);

                                    (
                                        latest_ts,
                                        AceCycleSummary {
                                            cycle_id,
                                            lane,
                                            status,
                                            anchor,
                                            budget: None,
                                            latest_sync_point: None,
                                            pending_injections: Vec::new(),
                                            decision_path: None,
                                            metadata: None,
                                        },
                                    )
                                })
                                .collect();

                            summaries_with_ts.sort_by_key(|(ts, _)| *ts);
                            summaries_with_ts.reverse();

                            let summaries: Vec<AceCycleSummary> = summaries_with_ts
                                .into_iter()
                                .map(|(_, summary)| summary)
                                .collect();

                            if let Some(first) = summaries.first().cloned() {
                                actions.select_ace_cycle(Some(first.cycle_id.clone()));
                            } else {
                                actions.select_ace_cycle(None);
                            }

                            actions.set_ace_cycles(summaries);
                            actions.set_ace_loading(false);
                            return;
                        } else {
                            actions.set_ace_cycles(Vec::new());
                            actions.select_ace_cycle(None);
                            actions.set_ace_loading(false);
                        }
                    }
                    Err(err) => {
                        tracing::error!("awareness events fetch failed: {err}");
                        actions.set_ace_error(Some(format!("ACE 数据加载失败: {err}")));
                        actions.set_ace_loading(false);
                        return;
                    }
                }
            } else {
                actions.set_ace_error(Some("Thin-Waist 客户端未初始化".into()));
                actions.set_ace_loading(false);
                return;
            }
        }
    }));
}

fn detect_lane(events: &[AwarenessEvent]) -> AceLane {
    for event in events.iter().rev() {
        if let Some(lane) = event.payload.get("lane").and_then(|value| value.as_str()) {
            match lane {
                "tool" | "tool_lane" => return AceLane::Tool,
                "self_reason" | "self" => return AceLane::SelfReason,
                "collab" | "collaboration" => return AceLane::Collab,
                _ => return AceLane::Clarify,
            }
        }
    }
    AceLane::Clarify
}

fn detect_status(events: &[AwarenessEvent]) -> AceCycleStatus {
    if events
        .iter()
        .any(|event| matches!(event.event_type, AwarenessEventType::Finalized))
    {
        AceCycleStatus::Completed
    } else if events
        .iter()
        .any(|event| matches!(event.event_type, AwarenessEventType::Rejected))
    {
        AceCycleStatus::Failed
    } else {
        AceCycleStatus::Running
    }
}
