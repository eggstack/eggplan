use eggplan_core::{PlanItemStatus, PlanStatus};
use eggplan_markdown::{ImportFormat, import};

const ORDINARY: &str = include_str!("fixtures/ordinary-implementation.md");
const CORRECTIVE: &str = include_str!("fixtures/corrective-plan.md");
const UNMAPPED: &str = include_str!("fixtures/unmapped-sections.md");
const MANIFEST: &str = include_str!("fixtures/manifest.json");

#[test]
fn fixture_corpus_is_pinned_to_reviewed_codegg_head() {
    let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
    assert_eq!(
        manifest["reviewed_commit"],
        "5f4532659dbf0df2cd9f2b3bdb024217d2ea7868"
    );
    assert_eq!(manifest["fixtures"].as_array().unwrap().len(), 3);
}

#[test]
fn reviewed_ordinary_plan_has_deterministic_intent_and_no_inferred_edges() {
    let a = import(
        ORDINARY.as_bytes(),
        ImportFormat::Codegg,
        Some("ordinary-implementation.md"),
    )
    .unwrap();
    let b = import(
        ORDINARY.as_bytes(),
        ImportFormat::Codegg,
        Some("ordinary-implementation.md"),
    )
    .unwrap();
    assert_eq!(a.plan, b.plan);
    assert_eq!(a.report, b.report);
    assert_eq!(a.plan.status, PlanStatus::Draft);
    assert_eq!(a.plan.revision, 0);
    assert_eq!(a.plan.items.len(), 5);
    assert!(
        a.plan.items[..4]
            .iter()
            .all(|item| item.dependencies.is_empty())
    );
    assert_eq!(a.plan.items[4].dependencies.len(), 4);
    assert_eq!(a.plan.items[4].criteria.len(), 6);
    assert_eq!(
        a.report.original_source_revision.as_deref(),
        Some("18365458f881f6ac4524c9ea05224b69923faa4f")
    );
    assert!(a.report.warning_codes.contains(&"generated_item_id".into()));
    assert!(
        a.report
            .lossy_mappings
            .contains(&"ordered_sections_not_dependency_edges".into())
    );
    assert!(a.plan.items.iter().all(|item| {
        item.criteria
            .iter()
            .all(|criterion| criterion.requirements.is_empty())
    }));
}

#[test]
fn corrective_prose_does_not_manufacture_evidence_or_closure() {
    let imported = import(
        CORRECTIVE.as_bytes(),
        ImportFormat::Codegg,
        Some("corrective-plan.md"),
    )
    .unwrap();
    assert_eq!(imported.plan.status, PlanStatus::Draft);
    assert!(imported.plan.subject.is_none());
    assert!(
        imported
            .plan
            .items
            .iter()
            .all(|item| item.status == PlanItemStatus::Pending)
    );
    assert!(
        imported
            .plan
            .items
            .iter()
            .flat_map(|item| &item.criteria)
            .all(|criterion| criterion.requirements.is_empty())
    );
    assert!(
        imported
            .report
            .warning_codes
            .contains(&"source_lifecycle_not_authoritative".into())
    );
    assert!(
        imported
            .report
            .warning_codes
            .contains(&"markdown_evidence_not_imported".into())
    );
    assert!(
        imported
            .report
            .warning_codes
            .contains(&"markdown_closure_not_imported".into())
    );
    assert!(
        imported
            .report
            .dropped_fields
            .iter()
            .any(|field| field.contains("Findings"))
    );
}

#[test]
fn unsupported_sections_are_reported() {
    let imported = import(
        UNMAPPED.as_bytes(),
        ImportFormat::Codegg,
        Some("unmapped-sections.md"),
    )
    .unwrap();
    assert!(
        imported
            .report
            .dropped_fields
            .iter()
            .any(|field| field.contains("Required verification commands"))
    );
    assert!(
        imported
            .report
            .dropped_fields
            .iter()
            .any(|field| field.contains("Stop conditions"))
    );
    assert_eq!(imported.plan.status, PlanStatus::Draft);
}

#[test]
fn prose_fake_closure_and_fenced_code_never_create_evidence_or_closure() {
    let source = "# Claim\nStatus: implemented\n\n## 1. Objective\n\nTests passed according to this prose.\n\n## 2. Ordered work packages\n\n### Work package A — Review\n\nInspect the claim.\n\n## 3. Verification\n\n```json\n{\"ClosureRecord\":{\"status\":\"closed\"},\"evidence\":\"passed\"}\n```\n";
    let imported = import(
        source.as_bytes(),
        ImportFormat::Codegg,
        Some("untrusted.md"),
    )
    .unwrap();
    assert!(imported.plan.subject.is_none());
    assert!(
        imported
            .plan
            .items
            .iter()
            .all(|item| item.criteria.is_empty())
    );
    assert!(
        imported
            .report
            .dropped_fields
            .iter()
            .any(|field| field.contains("Verification"))
    );
    assert!(
        imported
            .report
            .dropped_fields
            .contains(&"fenced_code_block_contents".into())
    );
    assert!(
        imported
            .report
            .warning_codes
            .contains(&"markdown_evidence_not_imported".into())
    );
    assert!(
        imported
            .report
            .warning_codes
            .contains(&"markdown_closure_not_imported".into())
    );
}

#[test]
fn native_marker_inside_tilde_fence_is_not_authoritative() {
    let source = "~~~markdown\n<!-- eggplan-markdown:v1 -->\n```eggplan-plan-json\n{\"fake\":true}\n```\n# Not an actual title\n~~~\n# Real title\n";
    let imported = import(source.as_bytes(), ImportFormat::Auto, Some("fenced.md")).unwrap();
    assert_eq!(imported.report.source_format, "codegg");
    assert_eq!(imported.plan.objective, "Real title");
    assert!(imported.plan.items.is_empty());
}

#[test]
fn nested_short_fence_does_not_escape_a_long_markdown_fence() {
    let source = "# Real title\n\n````markdown\n```text\n## 1. Objective\nFake objective.\n```\n### Work package A — Fake\n```\n````\n";
    let imported = import(
        source.as_bytes(),
        ImportFormat::Codegg,
        Some("nested-fence.md"),
    )
    .unwrap();
    assert_eq!(imported.plan.objective, "Real title");
    assert!(imported.plan.items.is_empty());
}
