use crate::models::{
    CausalGraphEdge, CausalGraphNode, CausalGraphView, ConversationScenario, DialogueEventType,
    RecallResultView,
};

pub struct GraphInsightSample {
    pub causal: CausalGraphView,
    pub recall: Vec<RecallResultView>,
}

pub fn sample_graph_bundle(root_event_id: u64) -> GraphInsightSample {
    let nodes = build_nodes(root_event_id);
    let edges = build_edges();

    let causal = CausalGraphView {
        root_event_id,
        nodes,
        edges,
    };

    let recall = build_recall_results(root_event_id);

    GraphInsightSample { causal, recall }
}

fn build_nodes(root_event_id: u64) -> Vec<CausalGraphNode> {
    let all_events = vec![
        (
            10_000u64,
            Some(DialogueEventType::Message),
            "Clarify 提问",
            Some("Clarify"),
            Some(0),
        ),
        (
            10_001u64,
            Some(DialogueEventType::ToolCall),
            "工具调用",
            Some("Tool"),
            Some(1),
        ),
        (
            10_002u64,
            Some(DialogueEventType::ToolResult),
            "工具响应",
            Some("Tool"),
            Some(2),
        ),
        (20_001u64, None, "决策路由", Some("Awareness"), Some(3)),
    ];

    all_events
        .into_iter()
        .map(
            |(event_id, event_type, label, lane, order)| CausalGraphNode {
                event_id,
                event_type,
                scenario: Some(ConversationScenario::HumanToAi),
                label: Some(label.to_string()),
                summary: lane.map(|lane| lane.to_string()),
                timestamp_ms: Some(1_700_000_000_000 + (event_id as i64 % 1_000) * 120),
                depth: Some(relative_depth(root_event_id, event_id, order.unwrap_or(0))),
                score: Some(if event_id == root_event_id { 1.0 } else { 0.75 }),
            },
        )
        .collect()
}

fn relative_depth(root: u64, event_id: u64, fallback: i32) -> i32 {
    if root == event_id {
        0
    } else if root == 10_000 {
        fallback
    } else if root == 10_001 {
        match event_id {
            10_000 => -1,
            10_001 => 0,
            10_002 => 1,
            20_001 => 2,
            _ => fallback,
        }
    } else if root == 10_002 {
        match event_id {
            10_002 => 0,
            10_001 => -1,
            10_000 => -2,
            20_001 => 1,
            _ => fallback,
        }
    } else {
        fallback
    }
}

fn build_edges() -> Vec<CausalGraphEdge> {
    vec![
        CausalGraphEdge {
            from: 10_000,
            to: 10_001,
            relation: Some("follow_up".into()),
        },
        CausalGraphEdge {
            from: 10_001,
            to: 10_002,
            relation: Some("produces".into()),
        },
        CausalGraphEdge {
            from: 10_002,
            to: 20_001,
            relation: Some("drives".into()),
        },
    ]
}

fn build_recall_results(root_event_id: u64) -> Vec<RecallResultView> {
    let base = vec![
        RecallResultView {
            event_id: 9_900,
            score: 0.82,
            label: Some("Clarify 预热".into()),
            snippet: Some("Clarify lane SLA 需要人工确认".into()),
            reason: Some("semantic".into()),
        },
        RecallResultView {
            event_id: 9_850,
            score: 0.75,
            label: Some("DocSearch 结果".into()),
            snippet: Some("工具文档：clarify_lane_sla.md".into()),
            reason: Some("vector".into()),
        },
        RecallResultView {
            event_id: 9_700,
            score: 0.68,
            label: Some("历史 Clarify 回答".into()),
            snippet: Some("Clarify lane 优先级说明".into()),
            reason: Some("bm25".into()),
        },
    ];

    base.into_iter()
        .map(|mut item| {
            if root_event_id == 10_002 {
                item.score *= 1.05;
            } else if root_event_id == 10_000 {
                item.score *= 0.97;
            }
            item
        })
        .collect()
}
