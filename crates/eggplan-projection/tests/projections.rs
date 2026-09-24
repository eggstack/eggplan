use eggplan_core::{Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus};
use eggplan_projection::{
    MAX_PROJECTED_ITEMS, MAX_PROJECTED_PLANS, MAX_TEXT_CHARS, OutputEnvelope, graph_projection,
    readiness_projection, reason_code, registry_projection, summarize_plan,
};
use serde_json::{Value, json};

fn item(id: &str, position: u32, status: PlanItemStatus, dependencies: &[&str]) -> PlanItem {
    PlanItem {
        id: PlanItemId::new(format!("epi_{id}")).unwrap(),
        position,
        parent: None,
        dependencies: dependencies
            .iter()
            .map(|id| PlanItemId::new(format!("epi_{id}")).unwrap())
            .collect(),
        status,
        description: format!("description {id}"),
        criteria: vec![],
        blocker: None,
        next_action: None,
    }
}

fn plan(id: &str, items: Vec<PlanItem>) -> Plan {
    let mut plan = Plan::new(
        PlanId::new(format!("ep_{id}")).unwrap(),
        "projection fixture",
        items,
    )
    .unwrap();
    plan.status = PlanStatus::Active;
    plan
}

#[test]
fn readiness_graph_and_summary_are_stable_and_derive_from_core() {
    let mut root = item("root", 0, PlanItemStatus::Pending, &["dependency"]);
    root.parent = Some(PlanItemId::new("epi_done").unwrap());
    root.next_action = Some("n".repeat(MAX_TEXT_CHARS + 1));
    let mut blocked = item("blocked", 3, PlanItemStatus::Blocked, &[]);
    blocked.blocker = Some("b".repeat(MAX_TEXT_CHARS + 1));
    let plan = plan(
        "projection",
        vec![
            root,
            item("done", 1, PlanItemStatus::Completed, &[]),
            item("free", 2, PlanItemStatus::Actionable, &[]),
            blocked,
            item("dependency", 4, PlanItemStatus::InProgress, &[]),
        ],
    );
    let ready = readiness_projection(&plan);
    assert_eq!(ready.ready_count, 1);
    assert_eq!(ready.waiting_count, 1);
    assert_eq!(ready.not_actionable_count, 3);
    assert_eq!(
        ready.items[0].readiness.as_deref(),
        Some("waiting_on_dependencies")
    );
    assert_eq!(ready.items[0].waiting_on[0].as_str(), "epi_dependency");
    assert!(ready.items[0].next_action_truncated);
    assert!(ready.items[3].blocker_truncated);

    let graph = graph_projection(&plan);
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|node| node.item_id.as_str())
            .collect::<Vec<_>>(),
        [
            "epi_root",
            "epi_done",
            "epi_free",
            "epi_blocked",
            "epi_dependency"
        ]
    );
    assert_eq!(graph.dependency_edges.len(), 1);
    assert_eq!(graph.parent_edges.len(), 1);
    assert_eq!(graph, graph_projection(&plan));

    let summary = summarize_plan(&plan, None, None, false);
    assert_eq!(summary.item_count, 5);
    assert_eq!(summary.item_status_counts["pending"], 1);
}

#[test]
fn graph_and_registry_truncation_report_original_counts() {
    let items = (0..MAX_PROJECTED_ITEMS + 1)
        .map(|index| {
            item(
                &format!("node{index}"),
                index as u32,
                PlanItemStatus::Pending,
                &[],
            )
        })
        .collect();
    let large_plan = plan("large", items);
    let graph = graph_projection(&large_plan);
    assert_eq!(graph.nodes.len(), MAX_PROJECTED_ITEMS);
    assert_eq!(graph.total_nodes, MAX_PROJECTED_ITEMS + 1);
    assert!(graph.truncated);

    let plans = (0..MAX_PROJECTED_PLANS + 1)
        .map(|index| (plan(&format!("r{index}"), vec![]), false, None))
        .collect::<Vec<_>>();
    let registry = registry_projection("epr_projection", plans);
    assert_eq!(registry.plan_count, MAX_PROJECTED_PLANS + 1);
    assert_eq!(registry.projected_plan_count, MAX_PROJECTED_PLANS);
    assert!(registry.plans_truncated);
}

#[test]
fn output_envelope_is_versioned_and_warnings_are_bounded() {
    let expected: Value =
        serde_json::from_slice(include_bytes!("fixtures/status-envelope.json")).unwrap();
    let envelope = OutputEnvelope::success("status", json!({"status":"active"}), vec![]);
    assert_eq!(serde_json::to_value(envelope).unwrap(), expected);
    let bounded = OutputEnvelope::<Value>::success(
        "status",
        json!({}),
        vec!["w".repeat(MAX_TEXT_CHARS + 20); 40],
    );
    assert_eq!(bounded.warnings.len(), 32);
    assert!(
        bounded
            .warnings
            .iter()
            .all(|warning| warning.chars().count() == MAX_TEXT_CHARS)
    );
}

#[test]
fn reason_codes_are_stable_and_do_not_depend_on_rendered_messages() {
    assert_eq!(
        reason_code(&eggplan_core::AssessmentReason::PlanBlocked),
        "plan_blocked"
    );
    assert_eq!(
        reason_code(&eggplan_core::AssessmentReason::UntrustedProvider(
            eggplan_core::EvidenceProviderId::new("epp_fixture").unwrap()
        )),
        "untrusted_provider"
    );
}
