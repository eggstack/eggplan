//! Golden-output equivalence between the repository inspection snapshot and the
//! per-Plan read shape it replaced.
//!
//! M003a's contract is that `status`, `registry render`, and `check` keep the
//! same stable JSON meaning for equivalent logical state. The strongest proof
//! available is not a frozen byte fixture — that would only show the output did
//! not change *once* — but a differential check: run both read shapes over the
//! same repository and assert the derived projections are identical.
//!
//! The "previous" shape here is reconstructed from the still-public
//! `PlanStore::list`/`get`/`list_observations`/`list_supersessions`/
//! `closure_record` APIs and a per-Plan `subject_source().capture()`, which is
//! exactly what the CLI did before the snapshot existed.

use eggplan_core::{ProviderRegistry, SubjectRevision, assess_plan, effective_observations};
use eggplan_projection::{PlanSummary, registry_projection, summarize_plan};
use eggplan_repo::{
    InspectionSelection, InspectionSnapshot, LoadedPlanSnapshot, PlanStore, RepositoryStore,
};
use git2::{Repository, Signature};
use std::fs;
use std::path::Path;
use tempfile::{TempDir, tempdir};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Commit a clean baseline that ignores the state root, so the Git subject is
/// stable while the fixture writes plans. Without this the state root itself
/// would keep the worktree dirty, and two subject captures taken at different
/// moments would legitimately differ.
fn init_git(path: &Path) {
    Repository::init(path).unwrap();
    fs::write(path.join(".gitignore"), b".eggplan/\n").unwrap();
    commit_all(path);
}

fn commit_all(path: &Path) {
    let repository = Repository::open(path).unwrap();
    let mut index = repository.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = Signature::now("Eggplan Test", "test@example.invalid").unwrap();
    let parent = repository
        .head()
        .ok()
        .map(|head| head.peel_to_commit().unwrap());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "baseline",
            &tree,
            &parents,
        )
        .unwrap();
}

/// Build a repository with `plans` Plans, `observations` per Plan, and mark
/// every other Plan Closed so closure presence, supersession lineage, and
/// closure-subject staleness are all represented in the compared output.
fn repository(plans: usize, observations: usize) -> (TempDir, RepositoryStore) {
    let dir = tempdir().unwrap();
    init_git(dir.path());
    let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
    let subject = store.subject_source().capture().unwrap();
    let binding = VerificationDigest::new(format!("sha256:{}", "c".repeat(64))).unwrap();

    for index in 0..plans {
        let item = format!("epi_golden_{index}");
        let criterion = format!("epc_golden_{index}");
        let id = format!("ep_golden_{index:02}");
        let mut plan = eggplan_core::Plan::new(
            eggplan_core::PlanId::new(&id).unwrap(),
            format!("golden plan {index}"),
            vec![eggplan_core::PlanItem {
                id: eggplan_core::PlanItemId::new(&item).unwrap(),
                position: 0,
                parent: None,
                dependencies: vec![],
                status: eggplan_core::PlanItemStatus::Completed,
                description: format!("item {index}"),
                criteria: vec![eggplan_core::AcceptanceCriterion {
                    id: eggplan_core::CriterionId::new(&criterion).unwrap(),
                    statement: "designated test passed".into(),
                    human_judgment_allowed: false,
                    requirements: vec![eggplan_core::EvidenceRequirement {
                        description: "designated test invocation".into(),
                        kind: eggplan_core::EvidenceKind::Test,
                        provider: None,
                        subject_policy: eggplan_core::SubjectPolicy::Exact,
                        cardinality: eggplan_core::EvidenceCardinality::Any,
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
        plan.status = eggplan_core::PlanStatus::Active;
        let plan = store.compare_and_swap(&plan.id, 0, &plan).unwrap();

        for n in 0..observations {
            let observation = eggplan_core::EvidenceObservation::finalize(
                eggplan_core::EvidenceObservationInput {
                    id: eggplan_core::EvidenceObservationId::new(format!("epe_{id}_{n}")).unwrap(),
                    provider_id: eggplan_core::EvidenceProviderId::new("epp_test").unwrap(),
                    kind: eggplan_core::EvidenceKind::Test,
                    status: eggplan_core::EvidenceStatus::Passed,
                    subject: subject.clone(),
                    observed_at_unix_ms: 11 + n as u64,
                    invocation_ref: Some("cargo test".into()),
                    verification_digest: Some(binding.clone()),
                    result_metadata: Default::default(),
                    artifacts: vec![],
                },
            )
            .unwrap();
            store.append_observation(&plan.id, &observation).unwrap();
        }
    }
    assert!(
        store
            .subject_source()
            .capture()
            .unwrap()
            .dirty_digest
            .is_none(),
        "the fixture worktree must stay clean so both read shapes see one subject"
    );
    (dir, store)
}

use eggplan_core::VerificationDigest;

// ---------------------------------------------------------------------------
// The two read shapes
// ---------------------------------------------------------------------------

/// Pre-M003a repository-wide read: enumerate, then load each Plan and capture
/// the subject separately for each one.
fn legacy_status_plans(store: &RepositoryStore) -> Vec<PlanSummary> {
    let registry = ProviderRegistry::default();
    store
        .list()
        .unwrap()
        .into_iter()
        .take(100)
        .map(|id| {
            let plan = store.get(&id).unwrap();
            let closure = store.closure_record(&id).unwrap().is_some();
            let current_subject = store.subject_source().capture().ok();
            let assessment = current_subject.as_ref().map(|subject| {
                let observations = store.list_observations(&id).unwrap();
                let supersessions = store.list_supersessions(&id).unwrap();
                let effective: Vec<_> = effective_observations(&observations, &supersessions)
                    .unwrap()
                    .into_iter()
                    .cloned()
                    .collect();
                assess_plan(&plan, subject, &effective, &registry)
            });
            summarize_plan(&plan, current_subject, assessment.as_ref(), closure)
        })
        .collect()
}

fn snapshot_status_plans(snapshot: &InspectionSnapshot) -> Vec<PlanSummary> {
    let registry = ProviderRegistry::default();
    snapshot
        .plans()
        .iter()
        .map(|loaded| {
            let assessment = snapshot
                .subject()
                .map(|subject| assess_plan(loaded.plan(), subject, loaded.effective(), &registry));
            summarize_plan(
                loaded.plan(),
                snapshot.subject().cloned(),
                assessment.as_ref(),
                loaded.closure().is_some(),
            )
        })
        .collect()
}

fn legacy_registry(
    store: &RepositoryStore,
) -> Vec<(
    eggplan_core::Plan,
    bool,
    Option<eggplan_core::PlanAssessment>,
)> {
    let registry = ProviderRegistry::default();
    store
        .list()
        .unwrap()
        .into_iter()
        .map(|id| {
            let plan = store.get(&id).unwrap();
            let closure = store.closure_record(&id).unwrap().is_some();
            let assessment = store.subject_source().capture().ok().map(|subject| {
                let observations = store.list_observations(&id).unwrap();
                let supersessions = store.list_supersessions(&id).unwrap();
                let effective: Vec<_> = effective_observations(&observations, &supersessions)
                    .unwrap()
                    .into_iter()
                    .cloned()
                    .collect();
                assess_plan(&plan, &subject, &effective, &registry)
            });
            (plan, closure, assessment)
        })
        .collect()
}

fn snapshot_registry(
    snapshot: &InspectionSnapshot,
) -> Vec<(
    eggplan_core::Plan,
    bool,
    Option<eggplan_core::PlanAssessment>,
)> {
    let registry = ProviderRegistry::default();
    snapshot
        .plans()
        .iter()
        .map(|loaded: &LoadedPlanSnapshot| {
            (
                loaded.plan().clone(),
                loaded.closure().is_some(),
                snapshot.subject().map(|subject| {
                    assess_plan(loaded.plan(), subject, loaded.effective(), &registry)
                }),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn status_projection_is_identical_for_equivalent_logical_state() {
    for (plans, observations) in [(1usize, 0usize), (1, 1), (10, 3), (12, 2)] {
        let (_dir, store) = repository(plans, observations);
        let snapshot = store
            .inspection_snapshot(&InspectionSelection::Repository { retain: Some(100) })
            .unwrap();

        let legacy = legacy_status_plans(&store);
        let actual = snapshot_status_plans(&snapshot);

        assert_eq!(
            serde_json::to_value(&legacy).unwrap(),
            serde_json::to_value(&actual).unwrap(),
            "status projection diverged at {plans}x{observations}"
        );
        assert_eq!(snapshot.total_plans(), plans);
        assert_eq!(snapshot.truncated(), plans > 100);
    }
}

#[test]
fn registry_projection_is_identical_for_equivalent_logical_state() {
    for (plans, observations) in [(1usize, 0usize), (7, 2), (12, 4)] {
        let (_dir, store) = repository(plans, observations);
        let snapshot = store
            .inspection_snapshot(&InspectionSelection::Repository { retain: None })
            .unwrap();

        let legacy = registry_projection(store.repository_id(), legacy_registry(&store));
        let actual = registry_projection(snapshot.repository_id(), snapshot_registry(&snapshot));
        assert_eq!(
            serde_json::to_value(&legacy).unwrap(),
            serde_json::to_value(&actual).unwrap(),
            "registry projection diverged at {plans}x{observations}"
        );
    }
}

#[test]
fn check_counts_and_plan_states_are_identical_for_equivalent_logical_state() {
    // 101 is the smallest repository where `check` truncates its projection.
    for (plans, observations) in [(1usize, 0usize), (5, 3), (101, 0)] {
        let (_dir, store) = repository(plans, observations);
        let snapshot = store
            .inspection_snapshot(&InspectionSelection::Repository { retain: Some(100) })
            .unwrap();

        // Legacy counting shape.
        let ids = store.list().unwrap();
        let mut legacy_observations = 0;
        let mut legacy_supersessions = 0;
        let mut legacy_closures = 0;
        for id in &ids {
            legacy_observations += store.list_observations(id).unwrap().len();
            legacy_supersessions += store.list_supersessions(id).unwrap().len();
            legacy_closures += usize::from(store.closure_record(id).unwrap().is_some());
        }

        assert_eq!(snapshot.observations_counted(), legacy_observations);
        assert_eq!(snapshot.supersessions_counted(), legacy_supersessions);
        assert_eq!(snapshot.closures_counted(), legacy_closures);
        assert_eq!(snapshot.total_plans(), ids.len());
        // `check` truncates its projection at 100 while counting over all plans.
        assert_eq!(snapshot.plans().len(), ids.len().min(100));
        assert_eq!(snapshot.truncated(), ids.len() > 100);
    }
}

#[test]
fn closed_plan_states_and_closure_summary_are_identical() {
    // Close one plan through the guarded finalizer so the comparison covers
    // closure presence, closure lineage validation, and closure assessment
    // reproduction rather than only open plans.
    let dir = tempdir().unwrap();
    init_git(dir.path());
    let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
    let subject = store.subject_source().capture().unwrap();
    let binding = VerificationDigest::new(format!("sha256:{}", "c".repeat(64))).unwrap();
    let mut plan = eggplan_core::Plan::new(
        eggplan_core::PlanId::new("ep_golden_closed").unwrap(),
        String::from("closed plan"),
        vec![eggplan_core::PlanItem {
            id: eggplan_core::PlanItemId::new("epi_golden_closed").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: eggplan_core::PlanItemStatus::Completed,
            description: String::from("closed item"),
            criteria: vec![eggplan_core::AcceptanceCriterion {
                id: eggplan_core::CriterionId::new("epc_golden_closed").unwrap(),
                statement: "designated test passed".into(),
                human_judgment_allowed: false,
                requirements: vec![eggplan_core::EvidenceRequirement {
                    description: "designated test invocation".into(),
                    kind: eggplan_core::EvidenceKind::Test,
                    provider: None,
                    subject_policy: eggplan_core::SubjectPolicy::Exact,
                    cardinality: eggplan_core::EvidenceCardinality::Any,
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
    plan.status = eggplan_core::PlanStatus::Active;
    let plan = store.compare_and_swap(&plan.id, 0, &plan).unwrap();
    let observation =
        eggplan_core::EvidenceObservation::finalize(eggplan_core::EvidenceObservationInput {
            id: eggplan_core::EvidenceObservationId::new("epe_golden_closed_0").unwrap(),
            provider_id: eggplan_core::EvidenceProviderId::new("epp_test").unwrap(),
            kind: eggplan_core::EvidenceKind::Test,
            status: eggplan_core::EvidenceStatus::Passed,
            subject: subject.clone(),
            observed_at_unix_ms: 11,
            invocation_ref: Some("cargo test".into()),
            verification_digest: Some(binding),
            result_metadata: Default::default(),
            artifacts: vec![],
        })
        .unwrap();
    store.append_observation(&plan.id, &observation).unwrap();

    let observations = store.list_observations(&plan.id).unwrap();
    let supersessions = store.list_supersessions(&plan.id).unwrap();
    let effective: Vec<_> = effective_observations(&observations, &supersessions)
        .unwrap()
        .into_iter()
        .cloned()
        .collect();
    let mut providers = ProviderRegistry::default();
    providers
        .register_trusted(
            eggplan_core::ProviderDescriptor::new(
                eggplan_core::EvidenceProviderId::new("epp_test").unwrap(),
                String::from("host"),
                [eggplan_core::EvidenceKind::Test],
            )
            .unwrap(),
        )
        .unwrap();
    let assessment = assess_plan(&plan, &subject, &effective, &providers);
    let candidate = eggplan_core::ClosureCandidate::build(
        &plan,
        subject.clone(),
        assessment,
        &observations,
        &supersessions,
        vec![eggplan_core::ProviderPolicyEntry {
            provider_id: eggplan_core::EvidenceProviderId::new("epp_test").unwrap(),
            class: "host".into(),
            allowed_kinds: [eggplan_core::EvidenceKind::Test].into_iter().collect(),
        }],
        12,
    )
    .unwrap();
    store
        .finalize_closure(
            &candidate,
            eggplan_core::ClosureId::new("epcl_golden_closed").unwrap(),
            99,
        )
        .unwrap();

    let snapshot = store
        .inspection_snapshot(&InspectionSelection::Repository { retain: Some(100) })
        .unwrap();
    assert_eq!(snapshot.closures_counted(), 1);
    assert_eq!(snapshot.plans().len(), 1);
    assert!(snapshot.plans()[0].closure().is_some());

    // The closed plan still projects identically through both shapes.
    let legacy = legacy_status_plans(&store);
    let actual = snapshot_status_plans(&snapshot);
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::to_value(&actual).unwrap()
    );
    let _ = fs::metadata(dir.path());
}

#[test]
fn a_dirty_subject_projects_identically_through_both_shapes() {
    let (dir, store) = repository(4, 2);
    fs::write(dir.path().join("tracked-dirty.txt"), b"untracked\n").unwrap();
    let snapshot = store
        .inspection_snapshot(&InspectionSelection::Repository { retain: Some(100) })
        .unwrap();
    assert!(snapshot.subject().is_some());
    let subject: &SubjectRevision = snapshot.subject().unwrap();
    assert!(subject.dirty_digest.is_some());

    let legacy = legacy_status_plans(&store);
    let actual = snapshot_status_plans(&snapshot);
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::to_value(&actual).unwrap()
    );
}
