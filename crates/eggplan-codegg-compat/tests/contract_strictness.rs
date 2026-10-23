//! Pins the recorded open/closed contract decision for the bridge's outward
//! projection types.
//!
//! `architecture/codegg-compat.md` records the decision that
//! `CompatProjection` and `CompatItemSummary` stay **open**
//! (forward-compatible: an unknown key is accepted), as a deliberate exception
//! to the crate's other, strict, contract types. This file makes that decision
//! explicit and testable rather than incidental: it asserts that the two types
//! accept a forward-compatible unknown key, and that the crate's internal
//! contract types remain strict.

use eggplan_codegg_compat::{CompatItemSummary, CompatProjection, RepositoryPlanProjectionV1};
use eggplan_core::PlanItemStatus;
use serde_json::{Value, json};

fn item() -> CompatItemSummary {
    CompatItemSummary {
        id: "epi_one".into(),
        source_id: "wi_one".into(),
        position: 0,
        status: PlanItemStatus::Actionable,
        description: "One bounded row".into(),
        blocker: None,
        next_action: None,
    }
}

fn projection() -> CompatProjection {
    CompatProjection {
        schema_version: 1,
        assessment_reason_code: "actionable_work_remaining".into(),
        total_items: 1,
        completed_items: 0,
        actionable_count: 1,
        blocked_count: 0,
        in_progress_count: 0,
        truncated: false,
        current_item_id: Some("epi_one".into()),
        items: vec![item()],
    }
}

#[test]
fn compat_projection_accepts_a_forward_compatible_unknown_key() {
    let mut extended = serde_json::to_value(projection()).unwrap();
    extended
        .as_object_mut()
        .unwrap()
        .insert("future_field".into(), json!("v2"));
    let decoded: CompatProjection = serde_json::from_value(extended).unwrap();
    assert_eq!(decoded, projection());
}

#[test]
fn compat_item_summary_accepts_a_forward_compatible_unknown_key() {
    let mut extended = serde_json::to_value(item()).unwrap();
    extended
        .as_object_mut()
        .unwrap()
        .insert("future_field".into(), json!(7));
    let decoded: CompatItemSummary = serde_json::from_value(extended).unwrap();
    assert_eq!(decoded, item());
}

#[test]
fn compat_projection_accepts_a_forward_compatible_unknown_key_inside_an_item() {
    let mut extended = serde_json::to_value(projection()).unwrap();
    extended["items"][0]
        .as_object_mut()
        .unwrap()
        .insert("future_field".into(), json!(true));
    let decoded: CompatProjection = serde_json::from_value(extended).unwrap();
    assert_eq!(decoded, projection());
}

#[test]
fn the_open_decision_is_scoped_to_the_outward_projection_types() {
    // The crate's internal contract types stay strict: the open decision is a
    // recorded exception, not a crate-wide relaxation.
    let mut strict = json!({
        "schema_version": 1,
        "plan_id": "ep_binding",
        "revision": 0,
        "objective": "Bind repository plans",
        "status": "active",
        "content_digest": "sha256:".to_string() + &"a".repeat(64),
        "intent_digest": "sha256:".to_string() + &"b".repeat(64),
        "projection_digest": "sha256:".to_string() + &"c".repeat(64),
        "items": []
    });
    assert!(
        serde_json::from_value::<RepositoryPlanProjectionV1>(strict.clone()).is_ok(),
        "the strict internal contract type must still accept its own keys"
    );
    strict
        .as_object_mut()
        .unwrap()
        .insert("future_field".into(), json!("v2"));
    assert!(
        serde_json::from_value::<RepositoryPlanProjectionV1>(strict).is_err(),
        "the strict internal contract type must still reject an unknown key"
    );
}

#[test]
fn compat_projection_round_trips_without_an_unknown_key() {
    let value = serde_json::to_value(projection()).unwrap();
    let decoded: CompatProjection = serde_json::from_value(value).unwrap();
    assert_eq!(decoded, projection());
    let encoded = serde_json::to_string(&decoded).unwrap();
    let reparsed: CompatProjection = serde_json::from_str(&encoded).unwrap();
    assert_eq!(reparsed, projection());
}

#[test]
fn compat_projection_keeps_its_version_marker() {
    let value = serde_json::to_value(projection()).unwrap();
    assert_eq!(value["schema_version"], json!(1));
    let decoded: Value =
        serde_json::from_str(&serde_json::to_string(&projection()).unwrap()).unwrap();
    assert_eq!(decoded["schema_version"], json!(1));
}
