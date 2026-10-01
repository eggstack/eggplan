use eggplan_codegg_compat::{
    RepositoryProjectionError, project_repository_plan, repository_plan_binding_manifest,
};
use eggplan_core::{
    AcceptanceCriterion, CriterionId, EvidenceCardinality, EvidenceKind, EvidenceProviderId,
    EvidenceRequirement, Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus,
    SubjectPolicy, VerificationDigest,
};
fn plan() -> Plan {
    let first = PlanItemId::new("epi_first").unwrap();
    let second = PlanItemId::new("epi_second").unwrap();
    let verification = VerificationDigest::new(format!("sha256:{}", "b".repeat(64))).unwrap();
    let criterion = AcceptanceCriterion {
        id: CriterionId::new("epc_contract").unwrap(),
        statement: "The contract is preserved".into(),
        human_judgment_allowed: true,
        requirements: vec![EvidenceRequirement {
            description: "Run the contract suite".into(),
            kind: EvidenceKind::Test,
            provider: Some(EvidenceProviderId::new("epp_ci").unwrap()),
            subject_policy: SubjectPolicy::Exact,
            cardinality: EvidenceCardinality::All,
            min_count: 2,
            allow_human_judgment: false,
            expected_verification_digest: Some(verification),
        }],
    };
    let mut plan = Plan::new(
        PlanId::new("ep_binding").unwrap(),
        "Bind repository plans",
        vec![
            PlanItem {
                id: first.clone(),
                position: 0,
                parent: None,
                dependencies: vec![],
                status: PlanItemStatus::Completed,
                description: "Define contract".into(),
                criteria: vec![criterion],
                blocker: None,
                next_action: None,
            },
            PlanItem {
                id: second,
                position: 1,
                parent: Some(first.clone()),
                dependencies: vec![first],
                status: PlanItemStatus::Blocked,
                description: "Consume contract".into(),
                criteria: vec![],
                blocker: Some("waiting on host".into()),
                next_action: Some("resume after handoff".into()),
            },
        ],
    )
    .unwrap();
    plan.status = PlanStatus::Active;
    plan
}

#[test]
fn projection_preserves_structured_requirements_and_source_identity() {
    let projection = project_repository_plan(&plan()).unwrap();
    assert_eq!(projection.schema_version, 1);
    assert_eq!(projection.plan_id.as_str(), "ep_binding");
    assert_eq!(projection.items[0].status, PlanItemStatus::Completed);
    assert_eq!(projection.items[1].dependencies[0].as_str(), "epi_first");
    let requirement = &projection.items[0].criteria[0].requirements[0];
    assert_eq!(requirement.kind, EvidenceKind::Test);
    assert_eq!(requirement.cardinality, EvidenceCardinality::All);
    assert_eq!(requirement.min_count, 2);
    assert_eq!(requirement.provider.as_ref().unwrap().as_str(), "epp_ci");
    assert_eq!(
        requirement
            .expected_verification_digest
            .as_ref()
            .unwrap()
            .as_str(),
        format!("sha256:{}", "b".repeat(64))
    );
    assert!(
        projection
            .items
            .iter()
            .all(|item| item.criteria.iter().all(|criterion| !criterion
                .requirements
                .iter()
                .any(|r| r.kind == EvidenceKind::HumanJudgment)))
    );
    let manifest = repository_plan_binding_manifest(&projection);
    assert_eq!(manifest.intent_digest, projection.intent_digest);
}

#[test]
fn intent_digest_ignores_progress_while_projection_digest_tracks_it() {
    let original = plan();
    let before = project_repository_plan(&original).unwrap();
    let mut changed = original.clone();
    changed.revision += 1;
    changed.items[1].status = PlanItemStatus::InProgress;
    changed.items[1].blocker = None;
    changed.items[1].next_action = Some("resume now".into());
    let after = project_repository_plan(&changed).unwrap();
    assert_eq!(before.intent_digest, after.intent_digest);
    assert_ne!(before.projection_digest, after.projection_digest);
    changed.objective.push_str(" updated");
    assert_ne!(
        after.intent_digest,
        project_repository_plan(&changed).unwrap().intent_digest
    );

    let after_objective = project_repository_plan(&changed).unwrap();
    changed.objective = original.objective.clone();
    changed.items[0].criteria[0]
        .statement
        .push_str(" with care");
    assert_ne!(
        before.intent_digest,
        project_repository_plan(&changed).unwrap().intent_digest
    );

    let mut dependency_change = original;
    dependency_change.items[1].dependencies.clear();
    assert_ne!(
        before.intent_digest,
        project_repository_plan(&dependency_change)
            .unwrap()
            .intent_digest
    );
    assert_ne!(before.intent_digest, after_objective.intent_digest);
}

#[test]
fn only_active_and_blocked_plans_are_bindable() {
    for status in [PlanStatus::Draft, PlanStatus::Closed, PlanStatus::Cancelled] {
        let mut candidate = plan();
        candidate.status = status;
        assert_eq!(
            project_repository_plan(&candidate),
            Err(RepositoryProjectionError::NonBindableLifecycle)
        );
    }
    let mut blocked = plan();
    blocked.status = PlanStatus::Blocked;
    let projected = project_repository_plan(&blocked).unwrap();
    assert_eq!(projected.status, PlanStatus::Blocked);
}

#[test]
fn projection_is_deterministic_and_strictly_versioned() {
    let p = plan();
    let one = project_repository_plan(&p).unwrap();
    let two = project_repository_plan(&p).unwrap();
    assert_eq!(one, two);
    let json = serde_json::to_string(&one).unwrap();
    assert!(
        serde_json::from_str::<eggplan_codegg_compat::RepositoryPlanProjectionV1>(&json).is_ok()
    );
}
