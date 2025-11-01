use crate::models::{ConversationScenario, TenantWorkspace, WorkspaceSession};

/// 提供多租户 / 多会话演示数据，便于在未接入后端时验证工作台 UI。
pub fn sample_workspace_profiles() -> Vec<TenantWorkspace> {
    vec![
        TenantWorkspace {
            tenant_id: "tenant-alpha".into(),
            display_name: "Alpha Labs".into(),
            description: Some("默认 Clarify 工作流与 Manifest L2".into()),
            manifest_level: Some("L2".into()),
            clarify_strategy: Some("clarify_iterative_v1".into()),
            pinned_sessions: vec![WorkspaceSession {
                session_id: "sess-4012".into(),
                title: Some("Clarify SLA 升级".into()),
                scenario: Some(ConversationScenario::HumanToAi),
                last_active_ms: Some(1_700_001_200_000),
                pinned: true,
                summary: Some("P1 事故 HITL 处理中".into()),
            }],
            recent_sessions: vec![
                WorkspaceSession {
                    session_id: "sess-4012".into(),
                    title: Some("Clarify SLA 升级".into()),
                    scenario: Some(ConversationScenario::HumanToAi),
                    last_active_ms: Some(1_700_001_200_000),
                    pinned: true,
                    summary: Some("P1 事故 HITL 处理中".into()),
                },
                WorkspaceSession {
                    session_id: "sess-3980".into(),
                    title: Some("Multi AI 对齐".into()),
                    scenario: Some(ConversationScenario::MultiHumanToMultiAi),
                    last_active_ms: Some(1_699_998_900_000),
                    pinned: false,
                    summary: Some("回放 Clarify+Tool 组合".into()),
                },
            ],
        },
        TenantWorkspace {
            tenant_id: "tenant-beta".into(),
            display_name: "Beta Retail".into(),
            description: Some("零售问答场景，默认 Manifest L3".into()),
            manifest_level: Some("L3".into()),
            clarify_strategy: Some("clarify_minimal".into()),
            pinned_sessions: vec![WorkspaceSession {
                session_id: "sess-5201".into(),
                title: Some("新品发布问答".into()),
                scenario: Some(ConversationScenario::HumanGroup),
                last_active_ms: Some(1_700_002_450_000),
                pinned: true,
                summary: Some("多主持人协同".into()),
            }],
            recent_sessions: vec![
                WorkspaceSession {
                    session_id: "sess-5201".into(),
                    title: Some("新品发布问答".into()),
                    scenario: Some(ConversationScenario::HumanGroup),
                    last_active_ms: Some(1_700_002_450_000),
                    pinned: true,
                    summary: Some("多主持人协同".into()),
                },
                WorkspaceSession {
                    session_id: "sess-5150".into(),
                    title: Some("库存追踪".into()),
                    scenario: Some(ConversationScenario::AiToSystem),
                    last_active_ms: Some(1_699_997_100_000),
                    pinned: false,
                    summary: Some("自动化补货脚本".into()),
                },
            ],
        },
        TenantWorkspace {
            tenant_id: "tenant-gamma".into(),
            display_name: "Gamma Studio".into(),
            description: Some("实验性多 AI 协作".into()),
            manifest_level: Some("L1".into()),
            clarify_strategy: Some("clarify_auto".into()),
            pinned_sessions: Vec::new(),
            recent_sessions: vec![WorkspaceSession {
                session_id: "sess-6100".into(),
                title: Some("Self Talk 迭代".into()),
                scenario: Some(ConversationScenario::AiSelfTalk),
                last_active_ms: Some(1_700_000_050_000),
                pinned: false,
                summary: Some("自动推理回归".into()),
            }],
        },
    ]
}
