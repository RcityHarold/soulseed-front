use std::str::FromStr;

use serde_json::json;
use soulseed_agi_core_models::legacy::dialogue_event::{DialogueEvent, MessagePointer};
use soulseed_agi_core_models::{
    AccessClass, ConversationScenario, CorrelationId, DialogueEventType, EnvelopeHead, EventId,
    IdError, MessageId, Provenance, SessionId, Snapshot, Subject, SubjectRef, TenantId, TraceId,
};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DialogueBuildError {
    #[error("tenant_id 缺失")]
    MissingTenantId,
    #[error("tenant_id 无法解析: {0}")]
    InvalidTenantId(String),
    #[error("session_id 缺失")]
    MissingSessionId,
    #[error("session_id 无法解析: {0}")]
    InvalidSessionId(String),
    #[error("sequence_number 无效，应 >= 1")]
    InvalidSequence,
}

#[derive(Clone)]
pub struct MessageEventDraft<'a> {
    pub tenant_id: &'a str,
    pub session_id: &'a str,
    pub scenario: ConversationScenario,
    pub subject: Subject,
    pub participants: Vec<SubjectRef>,
    pub text: &'a str,
    pub sequence_number: u64,
    pub channel: Option<&'a str>,
    pub access_class: AccessClass,
    pub provenance: Option<Provenance>,
    pub config_snapshot_hash: Option<&'a str>,
    pub config_snapshot_version: Option<u32>,
    pub timestamp_override_ms: Option<i64>,
}

pub fn build_message_event(
    draft: MessageEventDraft<'_>,
) -> Result<DialogueEvent, DialogueBuildError> {
    let tenant = parse_tenant_id(draft.tenant_id)?;
    let session = parse_session_id(draft.session_id)?;

    if draft.sequence_number == 0 {
        return Err(DialogueBuildError::InvalidSequence);
    }

    let event_id = EventId::generate();
    let message_id = MessageId::generate();
    let now = OffsetDateTime::now_utc();
    let timestamp_ms = draft
        .timestamp_override_ms
        .unwrap_or_else(|| (now.unix_timestamp_nanos() / 1_000_000) as i64);
    let created_at = if let Some(override_ms) = draft.timestamp_override_ms {
        OffsetDateTime::from_unix_timestamp_nanos((override_ms as i128) * 1_000_000).unwrap_or(now)
    } else {
        now
    };

    let head = EnvelopeHead {
        envelope_id: Uuid::new_v4(),
        trace_id: TraceId(format!("trace-{}", Uuid::new_v4())),
        correlation_id: CorrelationId(format!("corr-{}", Uuid::new_v4())),
        config_snapshot_hash: draft
            .config_snapshot_hash
            .map(|s| s.to_string())
            .unwrap_or_else(|| "frontend:default".to_string()),
        config_snapshot_version: draft.config_snapshot_version.unwrap_or(1),
    };

    let provenance = draft.provenance.unwrap_or_else(|| Provenance {
        source: "soulseed-console".to_string(),
        method: "interaction_panel".to_string(),
        model: None,
        content_digest_sha256: None,
    });

    let metadata = json!({
        "text": draft.text,
        "channel": draft.channel,
        "submitted_at": created_at,
        "origin": "soulseed-console",
    });

    Ok(DialogueEvent {
        tenant_id: tenant,
        event_id,
        session_id: session,
        subject: draft.subject,
        participants: draft.participants,
        head,
        snapshot: Snapshot {
            schema_v: 1,
            created_at,
        },
        timestamp_ms,
        scenario: draft.scenario,
        event_type: DialogueEventType::Message,
        time_window: None,
        access_class: draft.access_class,
        provenance: Some(provenance),
        sequence_number: draft.sequence_number,
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
        message_ref: Some(MessagePointer { message_id }),
        tool_invocation: None,
        tool_result: None,
        self_reflection: None,
        metadata,
    })
}

fn parse_tenant_id(raw: &str) -> Result<TenantId, DialogueBuildError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DialogueBuildError::MissingTenantId);
    }
    TenantId::from_str(value)
        .or_else(|_| parse_numeric_id(value, TenantId::from_raw))
        .map_err(|err| DialogueBuildError::InvalidTenantId(format!("{value} ({err:?})")))
}

fn parse_session_id(raw: &str) -> Result<SessionId, DialogueBuildError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DialogueBuildError::MissingSessionId);
    }
    SessionId::from_str(value)
        .or_else(|_| parse_numeric_id(value, SessionId::from_raw))
        .map_err(|err| DialogueBuildError::InvalidSessionId(format!("{value} ({err:?})")))
}

fn parse_numeric_id<T, F>(raw: &str, ctor: F) -> Result<T, IdError>
where
    F: Fn(u64) -> Result<T, IdError>,
{
    let numeric = raw.parse::<u64>().map_err(|_| IdError::InvalidBase36)?;
    ctor(numeric)
}
