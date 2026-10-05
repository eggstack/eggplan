//! Crate-internal test fixtures shared by the store and snapshot test modules.
//!
//! Compiled only under `cfg(test)` and not part of the public API.

use crate::{PlanStore, RepositoryStore};
use eggplan_core::{
    AcceptanceCriterion, AssessmentStatus, ClosureCandidate, CriterionId, EvidenceCardinality,
    EvidenceKind, EvidenceObservation, EvidenceObservationId, EvidenceObservationInput,
    EvidenceProviderId, EvidenceRequirement, EvidenceStatus, Plan, PlanId, PlanItem, PlanItemId,
    PlanItemStatus, PlanStatus, ProviderDescriptor, ProviderPolicyEntry, ProviderRegistry,
    SubjectRevision, SubjectState, VerificationDigest, assess_plan, effective_observations,
};
use git2::{Repository, Signature};
use std::path::Path;

/// Deterministic baseline repository with one committed file.
pub(crate) fn init_git_repo(root: &Path) -> Repository {
    let repo = Repository::init(root).unwrap();
    std::fs::write(root.join("tracked.txt"), b"base\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Eggplan Test", "eggplan@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();
    drop(tree);
    repo
}

/// Make the worktree dirty by adding enough untracked content that repeated
/// dirty-subject hashing is measurable rather than a single small file.
pub(crate) fn dirty_worktree(root: &Path, files: usize, bytes: usize) {
    std::fs::write(root.join("tracked.txt"), b"base\nchanged\n").unwrap();
    for index in 0..files {
        let body = vec![b'x'; bytes];
        std::fs::write(root.join(format!("untracked-{index}.bin")), body).unwrap();
    }
}

/// A ready-to-close single-item Plan bound to one exact Test requirement.
pub(crate) fn ready_plan(store: &RepositoryStore, id: &str, subject: &SubjectRevision) -> Plan {
    let item_id = format!("epi_{id}");
    let criterion_id = format!("epc_{id}");
    let binding = VerificationDigest::new(format!("sha256:{}", "c".repeat(64))).unwrap();
    let mut plan = Plan::new(
        PlanId::new(id).unwrap(),
        format!("plan {id}"),
        vec![PlanItem {
            id: PlanItemId::new(&item_id).unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Completed,
            description: format!("item {id}"),
            criteria: vec![AcceptanceCriterion {
                id: CriterionId::new(&criterion_id).unwrap(),
                statement: "designated test passed".into(),
                human_judgment_allowed: false,
                requirements: vec![EvidenceRequirement {
                    description: "designated test invocation".into(),
                    kind: EvidenceKind::Test,
                    provider: None,
                    subject_policy: eggplan_core::SubjectPolicy::Exact,
                    cardinality: EvidenceCardinality::Any,
                    min_count: 1,
                    allow_human_judgment: false,
                    expected_verification_digest: Some(binding.clone()),
                }],
            }],
            blocker: None,
            next_action: None,
        }],
    )
    .unwrap();
    plan.subject = Some(subject.clone());
    store.create(&plan).unwrap();
    plan.revision = 1;
    plan.status = PlanStatus::Active;
    store.compare_and_swap(&plan.id, 0, &plan).unwrap()
}

/// Append `count` Passed Test observations for `subject`, each bound to the
/// plan's expected verification digest.
pub(crate) fn seed_observations(
    store: &RepositoryStore,
    plan: &Plan,
    subject: &SubjectRevision,
    count: usize,
) -> Vec<EvidenceObservation> {
    let binding = VerificationDigest::new(format!("sha256:{}", "c".repeat(64))).unwrap();
    let mut written = Vec::new();
    for index in 0..count {
        let observation = EvidenceObservation::finalize(EvidenceObservationInput {
            id: EvidenceObservationId::new(format!("epe_{}_obs_{index}", plan.id.as_str()))
                .unwrap(),
            provider_id: EvidenceProviderId::new("epp_test").unwrap(),
            kind: EvidenceKind::Test,
            status: EvidenceStatus::Passed,
            subject: subject.clone(),
            observed_at_unix_ms: 11 + index as u64,
            invocation_ref: Some("cargo test".into()),
            verification_digest: Some(binding.clone()),
            result_metadata: Default::default(),
            artifacts: vec![],
        })
        .unwrap();
        store.append_observation(&plan.id, &observation).unwrap();
        written.push(observation);
    }
    written
}

pub(crate) fn trusted_registry() -> ProviderRegistry {
    let mut providers = ProviderRegistry::default();
    providers
        .register_trusted(
            ProviderDescriptor::new(
                EvidenceProviderId::new("epp_test").unwrap(),
                String::from("host"),
                [EvidenceKind::Test],
            )
            .unwrap(),
        )
        .unwrap();
    providers
}

pub(crate) fn complete_candidate(
    store: &RepositoryStore,
    plan: &Plan,
    subject: &SubjectRevision,
    snapshot_observations: Option<&[EvidenceObservation]>,
    snapshot_supersessions: Option<&[eggplan_core::EvidenceSupersessionRecord]>,
) -> ClosureCandidate {
    let policy = vec![ProviderPolicyEntry {
        provider_id: EvidenceProviderId::new("epp_test").unwrap(),
        class: "host".into(),
        allowed_kinds: [EvidenceKind::Test].into_iter().collect(),
    }];
    let (observations, supersessions) = match (snapshot_observations, snapshot_supersessions) {
        (Some(observations), Some(supersessions)) => {
            (observations.to_vec(), supersessions.to_vec())
        }
        _ => (
            store.list_observations(&plan.id).unwrap(),
            store.list_supersessions(&plan.id).unwrap(),
        ),
    };
    let effective: Vec<_> = effective_observations(&observations, &supersessions)
        .unwrap()
        .into_iter()
        .cloned()
        .collect();
    let providers = trusted_registry();
    let assessment = assess_plan(plan, subject, &effective, &providers);
    assert_eq!(assessment.status, AssessmentStatus::Complete);
    ClosureCandidate::build(
        plan,
        subject.clone(),
        assessment,
        &observations,
        &supersessions,
        policy,
        12,
    )
    .unwrap()
}

/// Convenience: a store with `plan_count` ready Plans and `observations` per
/// Plan, all bound to the current clean subject.
pub(crate) fn seeded_repository(
    root: &Path,
    plan_count: usize,
    observations: usize,
) -> (RepositoryStore, Vec<Plan>) {
    let store = RepositoryStore::open(root.join(".eggplan")).unwrap();
    let subject = store.subject_source().capture().unwrap();
    let mut plans = Vec::new();
    for index in 0..plan_count {
        let id = format!("ep_snapshot_{index:03}");
        let plan = ready_plan(&store, &id, &subject);
        seed_observations(&store, &plan, &subject, observations);
        plans.push(plan);
    }
    (store, plans)
}

/// Seeded fixtures always bind a clean subject. Callers that need a dirty one
/// capture it explicitly, so a silent clean fallback is not possible.
pub(crate) fn assert_clean(subject: &SubjectRevision) {
    assert_eq!(subject.state, SubjectState::Clean);
}
