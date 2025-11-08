use std::collections::HashMap;

use dioxus::prelude::*;
use serde::Serialize;

use crate::models::{
    AceCycleStatus, AceCycleSummary, AceLane, AwarenessEvent, AwarenessEventType,
    CycleSnapshotView, OutboxMessageView,
};
use crate::state::{use_app_actions, use_app_state, AppActions, AppSignal};
use crate::{API_CLIENT, APP_CONFIG};

pub fn use_ace_cycles() {
    let actions = use_app_actions();
    let state = use_app_state();

    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone();
    let session = snapshot.session_id.clone();
    let tenant_for_details = tenant.clone();
    let selected_cycle_id = snapshot.ace.selected_cycle_id.clone();
    drop(snapshot);

    let actions_for_cycles = actions.clone();
    let state_for_cycles = state.clone();
    let cycles_loader = use_future(move || {
        let actions = actions_for_cycles.clone();
        let state = state_for_cycles.clone();
        async move {
            load_ace_cycles(actions, state).await;
        }
    });

    {
        let actions = actions.clone();
        let mut loader = cycles_loader;
        use_effect(use_reactive!(|(tenant, session)| {
            let _ = session;
            if tenant.is_some() {
                loader.restart();
            } else {
                actions.set_ace_cycles(Vec::new());
                actions.select_ace_cycle(None);
            }
        }));
    }

    let actions_for_details = actions.clone();
    let state_for_details = state.clone();
    let detail_loader = use_future(move || {
        let actions = actions_for_details.clone();
        let state = state_for_details.clone();
        async move {
            load_cycle_snapshot(actions, state).await;
        }
    });

    {
        let actions = actions.clone();
        let mut loader = detail_loader;
        use_effect(use_reactive!(|(tenant_for_details, selected_cycle_id)| {
            if tenant_for_details.is_some() && selected_cycle_id.is_some() {
                loader.restart();
            } else {
                actions.set_ace_snapshot_loading(false);
                actions.set_ace_snapshot_error(None);
            }
        }));
    }
}

#[derive(Serialize)]
struct AwarenessQuery {
    limit: u32,
}

async fn load_ace_cycles(actions: AppActions, state: AppSignal) {
    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone().or_else(|| {
        APP_CONFIG
            .get()
            .and_then(|cfg| cfg.default_tenant_id.clone())
    });
    drop(snapshot);

    let tenant = match tenant {
        Some(value) => value,
        None => {
            actions.set_ace_cycles(Vec::new());
            actions.select_ace_cycle(None);
            actions.set_ace_loading(false);
            return;
        }
    };

    actions.set_ace_loading(true);
    actions.set_ace_error(None);

    let client = match API_CLIENT.get().cloned() {
        Some(client) => client,
        None => {
            actions.set_ace_error(Some("Thin-Waist 客户端未初始化".into()));
            actions.set_ace_loading(false);
            return;
        }
    };

    match client
        .get_awareness_events::<_, Vec<AwarenessEvent>>(&tenant, &AwarenessQuery { limit: 200 })
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
            } else {
                actions.set_ace_cycles(Vec::new());
                actions.select_ace_cycle(None);
            }
            actions.set_ace_loading(false);
        }
        Err(err) => {
            tracing::error!("awareness events fetch failed: {err}");
            actions.set_ace_error(Some(format!("ACE 数据加载失败: {err}")));
            actions.set_ace_loading(false);
        }
    }
}

async fn load_cycle_snapshot(actions: AppActions, state: AppSignal) {
    let snapshot = state.read();
    let tenant = snapshot.tenant_id.clone().or_else(|| {
        APP_CONFIG
            .get()
            .and_then(|cfg| cfg.default_tenant_id.clone())
    });
    let selected_cycle_id = snapshot.ace.selected_cycle_id.clone();
    let cached = selected_cycle_id.as_ref().map(|id| {
        snapshot.ace.snapshots.contains_key(id) && snapshot.ace.outboxes.contains_key(id)
    });
    drop(snapshot);

    let selected = match selected_cycle_id {
        Some(value) => value,
        None => {
            actions.set_ace_snapshot_loading(false);
            actions.set_ace_snapshot_error(None);
            return;
        }
    };

    let tenant = match tenant {
        Some(value) => value,
        None => {
            actions.set_ace_snapshot_error(Some("请先选择租户".into()));
            actions.set_ace_snapshot_loading(false);
            return;
        }
    };

    if cached.unwrap_or(false) {
        actions.set_ace_snapshot_loading(false);
        return;
    }

    actions.set_ace_snapshot_loading(true);
    actions.set_ace_snapshot_error(None);

    let client = match API_CLIENT.get().cloned() {
        Some(client) => client,
        None => {
            actions.set_ace_snapshot_error(Some("Thin-Waist 客户端未初始化".into()));
            actions.set_ace_snapshot_loading(false);
            return;
        }
    };

    // 将 Base36 格式的 cycle_id 转换为 u64 字符串
    use soulseed_agi_core_models::AwarenessCycleId;
    use std::str::FromStr;

    let cycle_id_u64 = match AwarenessCycleId::from_str(&selected) {
        Ok(id) => id.as_u64().to_string(),
        Err(_) => {
            // 如果解析失败，可能已经是 u64 格式，直接使用
            selected.clone()
        }
    };

    let snapshot_res = client
        .get_cycle_snapshot::<CycleSnapshotView>(&cycle_id_u64, Some(&tenant))
        .await;
    let outbox_res = client
        .get_cycle_outbox::<Vec<OutboxMessageView>>(&cycle_id_u64, Some(&tenant))
        .await;

    match (snapshot_res, outbox_res) {
        (Ok(snapshot), Ok(outbox)) => {
            actions.store_ace_snapshot(selected.clone(), snapshot, outbox);
        }
        (snapshot_result, outbox_result) => {
            let mut message = String::new();
            if let Err(err) = snapshot_result {
                message.push_str(&format!("快照加载失败: {err}"));
            }
            if let Err(err) = outbox_result {
                if !message.is_empty() {
                    message.push_str("；");
                }
                message.push_str(&format!("Outbox 加载失败: {err}"));
            }
            if message.is_empty() {
                message.push_str("加载周期快照失败");
            }
            actions.set_ace_snapshot_error(Some(message));
        }
    }

    actions.set_ace_snapshot_loading(false);
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
