use serde_json::json;
use soulseed_agi_core_models::{
    AIId, AccessClass, AwarenessAnchor, AwarenessDegradationReason, AwarenessEvent,
    AwarenessEventType, AwarenessFork, AwarenessCycleId, ClarifyLimits, ClarifyPlan, ClarifyQuestion,
    ConversationScenario, CorrelationId, DecisionBudgetEstimate, DecisionExplain,
    DecisionPath, DecisionPlan, DecisionRationale, DialogueEvent, DialogueEventType, EnvelopeHead,
    EventId, HumanId, MessageId, MessagePointer, SessionId, Snapshot, Subject, SubjectRef,
    TenantId, ToolInvocation, ToolPlan, ToolPlanBarrier, ToolPlanEdge, ToolPlanNode, ToolResult,
    TraceId,
};

use crate::models::{
    AceBudget, AceCycleStatus, AceCycleSummary, AceExplainSection, AceLane, BundleBudget,
    BundleItem, BundleSegment, ContextAnchor, ContextBundleView, DfrExplainSection, ExplainBundle,
    ExplainIndices, ExplainSection,
};

use time::OffsetDateTime;
use uuid::Uuid;

/// 返回演示用的时间线和 Awareness 数据，便于在未接入后端时预览 UI。
pub fn sample_timeline_data() -> (Vec<DialogueEvent>, Vec<AwarenessEvent>) {
    let baseline = OffsetDateTime::now_utc().unix_timestamp() * 1_000;

    let events = vec![
        build_message_event(baseline),
        build_tool_call_event(baseline + 600),
        build_tool_result_event(baseline + 1_200),
    ];
    let awareness = vec![build_decision_awareness(baseline + 800)];

    (events, awareness)
}

/// 返回演示用的 ContextBundle 与 Explain 指纹。
pub fn sample_context_bundle() -> (ContextBundleView, ExplainIndices) {
    let anchor = ContextAnchor {
        tenant_id: 1,
        envelope_id: Uuid::new_v4().to_string(),
        config_snapshot_hash: "cfg-demo".to_string(),
        config_snapshot_version: 1,
        session_id: Some(501),
        sequence_number: Some(2),
        access_class: Some(AccessClass::Internal),
        provenance: None,
        schema_v: 1,
        scenario: Some(ConversationScenario::HumanToAi),
    };

    let bundle = ContextBundleView {
        anchor,
        segments: vec![
            BundleSegment {
                partition: "P1TaskFacts".to_string(),
                items: vec![BundleItem {
                    ci_id: "task-clarify-01".to_string(),
                    summary_level: Some("L1".to_string()),
                    tokens: 240,
                }],
            },
            BundleSegment {
                partition: "P3WorkingDelta".to_string(),
                items: vec![BundleItem {
                    ci_id: "delta-clarify-answer".to_string(),
                    summary_level: Some("L2".to_string()),
                    tokens: 180,
                }],
            },
        ],
        explain: ExplainBundle {
            reasons: vec!["Clarify 历史上下文".into(), "近期工具结果引用".into()],
            degradation_reason: None,
            indices_used: vec!["context_manifest_digest".into()],
            query_hash: Some("context:demo:v1".into()),
        },
        budget: Some(BundleBudget {
            target_tokens: 1_024,
            projected_tokens: 620,
        }),
    };

    let indices = ExplainIndices {
        graph: ExplainSection {
            indices_used: Vec::new(),
            query_hash: Some("timeline:sample".into()),
            degradation_reason: Some("index_miss".into()),
        },
        context: ExplainSection {
            indices_used: vec!["context_manifest_digest".into()],
            query_hash: Some("context:demo:v1".into()),
            degradation_reason: Some("manifest_stale".into()),
        },
        dfr: DfrExplainSection {
            router_digest: Some("sha256:dfr-demo".into()),
            degradation_reason: Some("budget_tokens".into()),
        },
        ace: AceExplainSection {
            sync_point: Some(crate::models::SyncPointKind::ClarifyAnswered),
            degradation_reason: Some("clarify_exhausted".into()),
        },
    };

    (bundle, indices)
}

/// 返回演示用的 ACE 周期列表。
pub fn sample_ace_cycles() -> Vec<AceCycleSummary> {
    let clarify_anchor = build_awareness_anchor();
    let tool_anchor = build_awareness_anchor();

    vec![
        AceCycleSummary {
            cycle_id: "cyc-clarify-001".into(),
            lane: AceLane::Clarify,
            status: AceCycleStatus::Running,
            anchor: Some(clarify_anchor.clone()),
            budget: Some(AceBudget {
                tokens_allowed: Some(6_000),
                tokens_spent: Some(1_450),
                walltime_ms_allowed: Some(30_000),
                walltime_ms_used: Some(12_500),
            }),
            latest_sync_point: None,
            pending_injections: vec![],
            decision_path: Some(build_clarify_decision_path(
                clarify_anchor,
                AwarenessCycleId::new(9_001),
            )),
            metadata: Some(json!({
                "stage": "question",
                "priority": "p1_high",
                "model": "planner-v1",
            })),
        },
        AceCycleSummary {
            cycle_id: "cyc-tool-002".into(),
            lane: AceLane::Tool,
            status: AceCycleStatus::Completed,
            anchor: Some(tool_anchor.clone()),
            budget: Some(AceBudget {
                tokens_allowed: Some(8_000),
                tokens_spent: Some(5_600),
                walltime_ms_allowed: Some(45_000),
                walltime_ms_used: Some(28_400),
            }),
            latest_sync_point: None,
            pending_injections: vec![],
            decision_path: Some(build_tool_decision_path(
                tool_anchor,
                AwarenessCycleId::new(9_002),
            )),
            metadata: Some(json!({
                "tool_plan": "doc.search -> summarizer",
                "last_tool": "doc.search",
            })),
        },
    ]
}

fn build_clarify_decision_path(
    anchor: AwarenessAnchor,
    cycle_id: AwarenessCycleId,
) -> DecisionPath {
    DecisionPath {
        anchor,
        awareness_cycle_id: cycle_id,
        inference_cycle_sequence: 1,
        fork: AwarenessFork::Clarify,
        plan: DecisionPlan::Clarify {
            plan: ClarifyPlan {
                questions: vec![ClarifyQuestion {
                    q_id: "clarify-001".into(),
                    text: "Clarify lane 的 SLA 是否触发 HITL?".into(),
                }],
                limits: ClarifyLimits {
                    max_parallel: Some(2),
                    max_rounds: Some(2),
                    wait_ms: Some(1_000),
                    total_wait_ms: Some(5_000),
                },
            },
        },
        budget_plan: DecisionBudgetEstimate {
            tokens: Some(800),
            walltime_ms: Some(3_000),
            external_cost: None,
        },
        rationale: DecisionRationale::default(),
        confidence: 0.82,
        explain: DecisionExplain {
            routing_seed: 42,
            router_digest: "clarify-router:v1".into(),
            router_config_digest: "cfg:clarify:demo".into(),
            features_snapshot: None,
        },
        degradation_reason: None,
    }
}

fn build_tool_decision_path(anchor: AwarenessAnchor, cycle_id: AwarenessCycleId) -> DecisionPath {
    DecisionPath {
        anchor,
        awareness_cycle_id: cycle_id,
        inference_cycle_sequence: 1,
        fork: AwarenessFork::ToolPath,
        plan: DecisionPlan::Tool {
            plan: ToolPlan {
                nodes: vec![
                    ToolPlanNode {
                        id: "doc-search".into(),
                        tool_id: "doc.search".into(),
                        version: Some("v2".into()),
                        input: json!({
                            "query": "Clarify lane SLA",
                            "filters": {"priority": "p1"},
                        }),
                        timeout_ms: Some(2_000),
                        success_criteria: Some(json!({"hit": 1})),
                        evidence_policy: Some("mandatory".into()),
                    },
                    ToolPlanNode {
                        id: "answer".into(),
                        tool_id: "answer.summarize".into(),
                        version: Some("v1".into()),
                        input: json!({"mode": "concise"}),
                        timeout_ms: Some(3_000),
                        success_criteria: None,
                        evidence_policy: None,
                    },
                ],
                edges: vec![ToolPlanEdge {
                    from: "doc-search".into(),
                    to: "answer".into(),
                }],
                barrier: ToolPlanBarrier {
                    mode: Some("serial".into()),
                    timeout_ms: Some(30_000),
                },
            },
        },
        budget_plan: DecisionBudgetEstimate {
            tokens: Some(6_000),
            walltime_ms: Some(25_000),
            external_cost: Some(0.45),
        },
        rationale: DecisionRationale::default(),
        confidence: 0.74,
        explain: DecisionExplain {
            routing_seed: 84,
            router_digest: "tool-router:v2".into(),
            router_config_digest: "cfg:tool:demo".into(),
            features_snapshot: None,
        },
        degradation_reason: Some(AwarenessDegradationReason::BudgetTokens),
    }
}

/// 返回模拟 SSE 的实时事件。
pub fn sample_live_event(seq: u64) -> (DialogueEvent, Option<AwarenessEvent>) {
    let timestamp_ms = OffsetDateTime::now_utc().unix_timestamp() * 1_000 + (seq as i64 * 450);

    let mut event = build_message_event(timestamp_ms);
    event.event_id = EventId::new(30_000 + seq);
    event.sequence_number = 100 + seq as u64;
    event.metadata = json!({
        "live": true,
        "sequence": seq,
    });

    let awareness = if seq % 2 == 0 {
        Some(build_live_awareness(timestamp_ms + 120, 40_000 + seq))
    } else {
        None
    };

    (event, awareness)
}

/// 根据用户输入生成演示用的对话事件。
pub fn make_dialogue_event_from_text(seq: u64, text: &str) -> DialogueEvent {
    let mut event = build_message_event(OffsetDateTime::now_utc().unix_timestamp() * 1_000);
    event.event_id = EventId::new(60_000 + seq);
    event.sequence_number = 200 + seq as u64;
    event.metadata = json!({
        "text": text,
        "source": "interaction_form",
    });
    event
}

/// 构建注入操作对应的元数据。
pub fn make_injection_metadata(note: &str) -> serde_json::Value {
    json!({
        "last_injection": note,
        "at": OffsetDateTime::now_utc().to_string(),
    })
}

fn build_message_event(timestamp_ms: i64) -> DialogueEvent {
    DialogueEvent {
        tenant_id: TenantId::new(1),
        event_id: EventId::new(10_000),
        session_id: SessionId::new(501),
        subject: Subject::Human(HumanId::new(42)),
        participants: vec![SubjectRef {
            kind: Subject::AI(AIId::new(7)),
            role: Some("assistant".to_string()),
        }],
        head: build_head(1),
        snapshot: Snapshot {
            schema_v: 1,
            created_at: OffsetDateTime::now_utc(),
        },
        timestamp_ms,
        scenario: ConversationScenario::HumanToAi,
        event_type: DialogueEventType::Message,
        time_window: None,
        access_class: AccessClass::Internal,
        provenance: None,
        sequence_number: 1,
        trigger_event_id: None,
        temporal_pattern_id: None,
        causal_links: Vec::new(),
        reasoning_trace: None,
        reasoning_confidence: None,
        reasoning_strategy: None,
        content_embedding: None,
        context_embedding: None,
        decision_embedding: None,
        embedding_meta: None,
        concept_vector: None,
        semantic_cluster_id: None,
        cluster_method: None,
        concept_distance_to_goal: None,
        real_time_priority: None,
        notification_targets: None,
        live_stream_id: None,
        growth_stage: None,
        processing_latency_ms: None,
        influence_score: None,
        community_impact: None,
        evidence_pointer: None,
        content_digest_sha256: None,
        blob_ref: None,
        supersedes: None,
        superseded_by: None,
        message_ref: Some(MessagePointer {
            message_id: MessageId::new(5_000_001),
        }),
        tool_invocation: None,
        tool_result: None,
        self_reflection: None,
        metadata: json!({
            "text": "你好，我们来梳理 Clarify 流程的关键问题。",
        }),
    }
}

fn build_tool_call_event(timestamp_ms: i64) -> DialogueEvent {
    DialogueEvent {
        tenant_id: TenantId::new(1),
        event_id: EventId::new(10_001),
        session_id: SessionId::new(501),
        subject: Subject::AI(AIId::new(7)),
        participants: vec![SubjectRef {
            kind: Subject::Human(HumanId::new(42)),
            role: Some("requester".to_string()),
        }],
        head: build_head(2),
        snapshot: Snapshot {
            schema_v: 1,
            created_at: OffsetDateTime::now_utc(),
        },
        timestamp_ms,
        scenario: ConversationScenario::HumanToAi,
        event_type: DialogueEventType::ToolCall,
        time_window: None,
        access_class: AccessClass::Internal,
        provenance: None,
        sequence_number: 2,
        trigger_event_id: Some(EventId::new(10_000)),
        temporal_pattern_id: None,
        causal_links: Vec::new(),
        reasoning_trace: Some("clarify -> gather_evidence".to_string()),
        reasoning_confidence: Some(0.8),
        reasoning_strategy: Some("plan_tool_chain".to_string()),
        content_embedding: None,
        context_embedding: None,
        decision_embedding: None,
        embedding_meta: None,
        concept_vector: None,
        semantic_cluster_id: None,
        cluster_method: None,
        concept_distance_to_goal: None,
        real_time_priority: None,
        notification_targets: None,
        live_stream_id: None,
        growth_stage: None,
        processing_latency_ms: None,
        influence_score: None,
        community_impact: None,
        evidence_pointer: None,
        content_digest_sha256: None,
        blob_ref: None,
        supersedes: None,
        superseded_by: None,
        message_ref: None,
        tool_invocation: Some(ToolInvocation {
            tool_id: "doc.search".to_string(),
            call_id: "call-01".to_string(),
            input: json!({
                "query": "Clarify lane SLA",
                "filters": {"priority": "p1"},
            }),
            strategy: Some("semantic".to_string()),
        }),
        tool_result: None,
        self_reflection: None,
        metadata: json!({
            "lane": "tool",
            "model": "planner-v1",
            "duration_ms": 120,
        }),
    }
}

fn build_tool_result_event(timestamp_ms: i64) -> DialogueEvent {
    DialogueEvent {
        tenant_id: TenantId::new(1),
        event_id: EventId::new(10_002),
        session_id: SessionId::new(501),
        subject: Subject::AI(AIId::new(7)),
        participants: vec![SubjectRef {
            kind: Subject::Human(HumanId::new(42)),
            role: Some("facilitator".to_string()),
        }],
        head: build_head(2),
        snapshot: Snapshot {
            schema_v: 1,
            created_at: OffsetDateTime::now_utc(),
        },
        timestamp_ms,
        scenario: ConversationScenario::HumanToAi,
        event_type: DialogueEventType::ToolResult,
        time_window: None,
        access_class: AccessClass::Internal,
        provenance: None,
        sequence_number: 3,
        trigger_event_id: Some(EventId::new(10_001)),
        temporal_pattern_id: None,
        causal_links: Vec::new(),
        reasoning_trace: Some("RAG > clarify > summarize".to_string()),
        reasoning_confidence: Some(0.72),
        reasoning_strategy: Some("retrieve_then_answer".to_string()),
        content_embedding: None,
        context_embedding: None,
        decision_embedding: None,
        embedding_meta: None,
        concept_vector: None,
        semantic_cluster_id: None,
        cluster_method: None,
        concept_distance_to_goal: None,
        real_time_priority: None,
        notification_targets: None,
        live_stream_id: None,
        growth_stage: None,
        processing_latency_ms: None,
        influence_score: None,
        community_impact: None,
        evidence_pointer: None,
        content_digest_sha256: None,
        blob_ref: None,
        supersedes: None,
        superseded_by: None,
        message_ref: None,
        tool_invocation: Some(ToolInvocation {
            tool_id: "doc.search".to_string(),
            call_id: "call-01".to_string(),
            input: json!({
                "query": "Clarify lane SLA",
            }),
            strategy: Some("semantic".to_string()),
        }),
        tool_result: Some(ToolResult {
            tool_id: "doc.search".to_string(),
            call_id: "call-01".to_string(),
            success: true,
            output: json!({
                "highlights": ["Clarify 默认 2 min SLA", "HITL 优先级 p1 需人工确认"],
            }),
            error: None,
            degradation_reason: Some("budget_tokens".into()),
        }),
        self_reflection: None,
        metadata: json!({
            "lane": "tool",
            "model": "gpt-4o-mini",
            "duration_ms": 420,
            "tokens": {
                "prompt": 320,
                "completion": 128,
            }
        }),
    }
}

fn build_decision_awareness(timestamp_ms: i64) -> AwarenessEvent {
    AwarenessEvent {
        anchor: build_awareness_anchor(),
        event_id: EventId::new(20_001),
        event_type: AwarenessEventType::DecisionRouted,
        occurred_at_ms: timestamp_ms,
        awareness_cycle_id: AwarenessCycleId::new(9_001),
        parent_cycle_id: None,
        collab_scope_id: None,
        barrier_id: Some("clarify_lane".to_string()),
        env_mode: Some("demo".to_string()),
        inference_cycle_sequence: 1,
        degradation_reason: Some(AwarenessDegradationReason::BudgetTokens),
        payload: json!({
            "lane": "clarify",
            "routing_seed": 42,
            "explain": {
                "indices_used": ["idx_dialogue_event_timeline"],
                "query_hash": "timeline:sample",
            }
        }),
    }
}

fn build_live_awareness(timestamp_ms: i64, event_id: u64) -> AwarenessEvent {
    AwarenessEvent {
        anchor: build_awareness_anchor(),
        event_id: EventId::new(event_id),
        event_type: AwarenessEventType::HumanInjectionReceived,
        occurred_at_ms: timestamp_ms,
        awareness_cycle_id: AwarenessCycleId::new(9_100),
        parent_cycle_id: None,
        collab_scope_id: Some("live-feed".to_string()),
        barrier_id: None,
        env_mode: Some("demo".to_string()),
        inference_cycle_sequence: 1,
        degradation_reason: None,
        payload: json!({
            "note": "接收到新的 Clarify 注入",
            "event_id": event_id,
        }),
    }
}

fn build_awareness_anchor() -> AwarenessAnchor {
    AwarenessAnchor {
        tenant_id: TenantId::new(1),
        envelope_id: Uuid::new_v4(),
        config_snapshot_hash: "cfg-demo".to_string(),
        config_snapshot_version: 1,
        session_id: Some(SessionId::new(501)),
        sequence_number: Some(2),
        access_class: AccessClass::Internal,
        provenance: None,
        schema_v: 1,
    }
}

fn build_head(seq: u64) -> EnvelopeHead {
    EnvelopeHead {
        envelope_id: Uuid::new_v4(),
        trace_id: TraceId(format!("trace-{seq:04}")),
        correlation_id: CorrelationId(format!("corr-{seq:04}")),
        config_snapshot_hash: "cfg-demo".to_string(),
        config_snapshot_version: 1,
    }
}
