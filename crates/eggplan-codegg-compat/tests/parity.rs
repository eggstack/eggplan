use eggplan_codegg_compat::{
    AcceptanceDisposition, CodeggEvidenceKind, CodeggEvidenceRef, CodeggItemStatus,
    CodeggPlanSnapshot, CodeggPlanStatus, EvidenceResolver, Fixture, ResolvedEvidence,
    SOURCE_CODEGG_SHA, completion_family, create_snapshot, normalize, project_bounded,
};
use eggplan_core::{
    AssessmentStatus, ClosureCandidate, ClosureId, EvidenceKind, EvidenceObservation,
    EvidenceObservationId, EvidenceObservationInput, EvidenceProviderId, EvidenceStatus,
    PlanItemStatus, ProviderDescriptor, ProviderPolicyEntry, ProviderRegistry, SubjectRevision,
    SubjectState, VerificationDigest, assess_plan,
};
use eggplan_repo::{PlanStore, RepoError, RepositoryStore};
use std::sync::{Arc, Barrier};
use tempfile::tempdir;

const FOUNDATION: &str = include_str!("fixtures/work_plan_foundation.json");
const PROJECTION: &str = include_str!("fixtures/work_plan_projection_arbiter.json");
const TRAJECTORY: &str = include_str!("fixtures/long_horizon_trajectory_qualification.json");
const FIXTURE_MANIFEST: &str = include_str!("fixtures/manifest.json");

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusManifest {
    schema_version: u32,
    source_repository: String,
    source_sha: String,
    fixture_files: Vec<String>,
}

fn subject() -> SubjectRevision {
    SubjectRevision {
        subject_kind: "git".into(),
        repository_id: "epr_fixture".into(),
        revision: "fixture-revision-1".into(),
        state: SubjectState::Clean,
        dirty_digest: None,
    }
}

struct Resolver {
    status: EvidenceStatus,
}

#[derive(Clone, Copy)]
enum ResolverFault {
    StaleSubject,
    MissingBinding,
    MismatchedBinding,
}

struct FaultyResolver(ResolverFault);

impl EvidenceResolver for Resolver {
    fn resolve(
        &mut self,
        reference: &CodeggEvidenceRef,
        subject: &SubjectRevision,
    ) -> Result<ResolvedEvidence, String> {
        let kind = match reference.kind {
            CodeggEvidenceKind::TestJob => EvidenceKind::Test,
            CodeggEvidenceKind::DelegatedRun | CodeggEvidenceKind::AgentRun => {
                EvidenceKind::DelegatedRun
            }
            CodeggEvidenceKind::SchedulerJob => EvidenceKind::Command,
            CodeggEvidenceKind::Artifact => EvidenceKind::Artifact,
            CodeggEvidenceKind::Commit => EvidenceKind::Revision,
        };
        let binding = VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
        let observation = EvidenceObservation::finalize(EvidenceObservationInput {
            id: EvidenceObservationId::new(format!("epe_{}", reference.ref_id)).unwrap(),
            provider_id: EvidenceProviderId::new("epp_host").unwrap(),
            kind,
            status: self.status,
            subject: subject.clone(),
            observed_at_unix_ms: 100,
            invocation_ref: Some(reference.ref_id.clone()),
            verification_digest: matches!(
                kind,
                EvidenceKind::Test
                    | EvidenceKind::Command
                    | EvidenceKind::DelegatedRun
                    | EvidenceKind::Benchmark
                    | EvidenceKind::StaticAnalysis
            )
            .then_some(binding.clone()),
            result_metadata: Default::default(),
            artifacts: vec![],
        })
        .map_err(|error| error.to_string())?;
        Ok(ResolvedEvidence {
            observation,
            expected_verification: matches!(
                kind,
                EvidenceKind::Test
                    | EvidenceKind::Command
                    | EvidenceKind::DelegatedRun
                    | EvidenceKind::Benchmark
                    | EvidenceKind::StaticAnalysis
            )
            .then_some(binding),
        })
    }
}

impl EvidenceResolver for FaultyResolver {
    fn resolve(
        &mut self,
        reference: &CodeggEvidenceRef,
        subject: &SubjectRevision,
    ) -> Result<ResolvedEvidence, String> {
        let mut resolved = Resolver {
            status: EvidenceStatus::Passed,
        }
        .resolve(reference, subject)?;
        let original = &resolved.observation;
        let authoritative_expected = resolved.expected_verification.clone();
        let mut observation_subject = original.subject().clone();
        let mut binding = original.verification_digest().cloned();
        match self.0 {
            ResolverFault::StaleSubject => observation_subject.revision = "stale".into(),
            ResolverFault::MissingBinding => binding = None,
            ResolverFault::MismatchedBinding => {
                binding =
                    Some(VerificationDigest::new(format!("sha256:{}", "b".repeat(64))).unwrap())
            }
        }
        let rebuilt = EvidenceObservation::finalize(EvidenceObservationInput {
            id: original.id().clone(),
            provider_id: original.provider_id().clone(),
            kind: original.kind(),
            status: original.status(),
            subject: observation_subject,
            observed_at_unix_ms: original.observed_at_unix_ms(),
            invocation_ref: original.invocation_ref().map(str::to_owned),
            verification_digest: binding.clone(),
            result_metadata: original.result_metadata().clone(),
            artifacts: original.artifacts().to_vec(),
        })
        .map_err(|error| error.to_string())?;
        resolved.observation = rebuilt;
        resolved.expected_verification = match self.0 {
            ResolverFault::MissingBinding => None,
            ResolverFault::MismatchedBinding => authoritative_expected,
            ResolverFault::StaleSubject => resolved.expected_verification,
        };
        Ok(resolved)
    }
}

fn load(raw: &str) -> Fixture {
    let fixture = Fixture::parse(raw.as_bytes()).unwrap();
    assert_eq!(fixture.source_sha, SOURCE_CODEGG_SHA);
    fixture
}

fn providers() -> ProviderRegistry {
    let mut providers = ProviderRegistry::default();
    providers
        .register_trusted(
            ProviderDescriptor::new(
                EvidenceProviderId::new("epp_host").unwrap(),
                "host",
                [
                    EvidenceKind::Test,
                    EvidenceKind::Artifact,
                    EvidenceKind::Revision,
                ],
            )
            .unwrap(),
        )
        .unwrap();
    providers
}

#[test]
fn strict_versioned_fixture_corpus_and_stable_identity_mapping() {
    let foundation = load(FOUNDATION);
    let corpus: CorpusManifest = serde_json::from_str(FIXTURE_MANIFEST).unwrap();
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.source_repository, "codegg");
    assert_eq!(corpus.source_sha, SOURCE_CODEGG_SHA);
    assert_eq!(corpus.fixture_files.len(), 3);
    assert!(load(PROJECTION).source_case.contains("projection_arbiter"));
    assert!(
        load(TRAJECTORY)
            .source_case
            .contains("long_horizon_trajectory_qualification")
    );
    let bad_version: Fixture =
        serde_json::from_str(&FOUNDATION.replace("\"schema_version\": 1", "\"schema_version\": 2"))
            .unwrap();
    assert!(
        normalize(
            &bad_version,
            subject(),
            &mut Resolver {
                status: EvidenceStatus::Passed
            }
        )
        .is_err()
    );
    let mut unknown: serde_json::Value = serde_json::from_str(FOUNDATION).unwrap();
    unknown["future_field"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Fixture>(unknown).is_err());
    let mut wrong_sha = foundation.clone();
    wrong_sha.source_sha = "0000000000000000000000000000000000000000".into();
    assert!(
        normalize(
            &wrong_sha,
            subject(),
            &mut Resolver {
                status: EvidenceStatus::Passed
            }
        )
        .is_err()
    );

    let mut first_resolver = Resolver {
        status: EvidenceStatus::Passed,
    };
    let first = normalize(&foundation, subject(), &mut first_resolver).unwrap();
    let mut second_resolver = Resolver {
        status: EvidenceStatus::Passed,
    };
    let second = normalize(&foundation, subject(), &mut second_resolver).unwrap();
    assert_eq!(first.plan.id, second.plan.id);
    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.manifest.id_map.len(), 2);
    let mapped: std::collections::BTreeSet<_> = first
        .manifest
        .id_map
        .iter()
        .map(|entry| &entry.eggplan_id)
        .collect();
    assert_eq!(mapped.len(), first.manifest.id_map.len());
    assert_eq!(first.manifest.schema_version, 1);
    first.manifest.validate().unwrap();
    let mut tampered = first.manifest.clone();
    tampered.source_revision += 1;
    assert!(tampered.validate().is_err());
    assert_eq!(first.plan.status, eggplan_core::PlanStatus::Draft);
}

#[test]
fn serialized_satisfied_owner_and_completed_labels_do_not_authorize_evidence() {
    let fixture = load(FOUNDATION);
    let mut resolver = Resolver {
        status: EvidenceStatus::Unavailable,
    };
    let mapped = normalize(&fixture, subject(), &mut resolver).unwrap();
    assert!(
        mapped
            .manifest
            .losses
            .contains(&eggplan_codegg_compat::Loss::SerializedDispositionIsNotEvidence)
    );
    assert!(
        mapped
            .manifest
            .losses
            .contains(&eggplan_codegg_compat::Loss::CompletedPlanNeedsGuardedClose)
    );
    assert!(
        mapped
            .manifest
            .losses
            .contains(&eggplan_codegg_compat::Loss::EvidenceDetailOmitted)
    );
    assert_eq!(mapped.plan.status, eggplan_core::PlanStatus::Draft);
    let mut plan = mapped.plan.clone();
    plan.status = eggplan_core::PlanStatus::Active;
    plan.revision = 1;
    let result = assess_plan(&plan, &subject(), &mapped.observations, &providers());
    assert_ne!(result.status, AssessmentStatus::Complete);

    let mut untrusted = ProviderRegistry::default();
    let passing = normalize(
        &fixture,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Passed,
        },
    )
    .unwrap();
    let mut active = passing.plan.clone();
    active.status = eggplan_core::PlanStatus::Active;
    active.revision = 1;
    assert_ne!(
        assess_plan(&active, &subject(), &passing.observations, &untrusted).status,
        AssessmentStatus::Complete
    );
    untrusted
        .register_trusted(
            ProviderDescriptor::new(
                EvidenceProviderId::new("epp_host").unwrap(),
                "host",
                [EvidenceKind::Test],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        assess_plan(&active, &subject(), &passing.observations, &untrusted).status,
        AssessmentStatus::Complete
    );
}

#[test]
fn completed_acceptance_without_any_host_reference_remains_incomplete() {
    let mut fixture = load(FOUNDATION);
    fixture.snapshot.items[0].evidence.clear();
    let mapped = normalize(
        &fixture,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Passed,
        },
    )
    .unwrap();
    assert!(mapped.observations.is_empty());
    assert_eq!(
        mapped.manifest.owner_provenance[0].owner_job_id.as_deref(),
        Some("job-fixture-1")
    );
    let mut plan = mapped.plan;
    plan.status = eggplan_core::PlanStatus::Active;
    plan.revision = 1;
    assert_eq!(
        assess_plan(&plan, &subject(), &[], &providers()).status,
        AssessmentStatus::EvidenceMissingOrUnavailable
    );
}

#[test]
fn stale_and_unbound_or_mismatched_execution_evidence_fail_closed() {
    let fixture = load(FOUNDATION);
    for fault in [
        ResolverFault::StaleSubject,
        ResolverFault::MissingBinding,
        ResolverFault::MismatchedBinding,
    ] {
        assert!(normalize(&fixture, subject(), &mut FaultyResolver(fault)).is_err());
    }
}

#[test]
fn user_judgment_stays_pending_and_completion_families_are_lossy_only_at_the_edge() {
    let mut fixture = load(PROJECTION);
    fixture.snapshot.status = CodeggPlanStatus::Active;
    fixture.snapshot.items.truncate(1);
    fixture.snapshot.current_item_id = Some("wi_first".into());
    fixture.snapshot.items[0].status = eggplan_codegg_compat::CodeggItemStatus::Actionable;
    fixture.snapshot.items[0].acceptance = vec![eggplan_codegg_compat::CodeggAcceptance {
        description: "Reviewer judgment required".into(),
        disposition: AcceptanceDisposition::RequiresUserJudgment,
        note: Some("explicit reviewer decision".into()),
    }];
    fixture.snapshot.items[0].evidence.clear();
    let mapped = normalize(
        &fixture,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Passed,
        },
    )
    .unwrap();
    let mut plan = mapped.plan.clone();
    plan.status = eggplan_core::PlanStatus::Active;
    plan.revision = 1;
    let assessment = assess_plan(&plan, &subject(), &mapped.observations, &providers());
    assert_eq!(assessment.status, AssessmentStatus::AwaitingHumanJudgment);
    assert_eq!(
        completion_family(AssessmentStatus::EvidenceFailed),
        eggplan_codegg_compat::CodeggCompletionFamily::ActionableWorkRemaining
    );
    assert_eq!(
        completion_family(AssessmentStatus::Blocked),
        eggplan_codegg_compat::CodeggCompletionFamily::Blocked
    );
    assert_eq!(
        completion_family(AssessmentStatus::AwaitingHumanJudgment),
        eggplan_codegg_compat::CodeggCompletionFamily::AwaitingUserJudgment
    );
}

#[test]
fn current_projection_is_bounded_stable_and_excludes_completed_history() {
    let fixture = load(PROJECTION);
    let mapped = normalize(
        &fixture,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::InProgress,
        },
    )
    .unwrap();
    let projection = project_bounded(&mapped, AssessmentStatus::InFlight, 1, 16);
    assert_eq!(projection.total_items, 3);
    assert_eq!(projection.completed_items, 1);
    assert_eq!(projection.assessment_reason_code, "in_flight");
    assert!(projection.truncated);
    assert_eq!(projection.items.len(), 1);
    assert_eq!(projection.items[0].source_id, "wi_second");
    assert!(projection.items[0].description.chars().count() <= 16);
    assert_eq!(
        projection.current_item_id.as_deref(),
        Some(projection.items[0].id.as_str())
    );
    assert_eq!(
        projection.items[0].next_action.as_deref(),
        Some("reload host resu")
    );
}

#[test]
fn snapshot_creation_uses_cas_and_restart_preserves_mapped_state() {
    let fixture = load(FOUNDATION);
    let mapped = normalize(
        &fixture,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Passed,
        },
    )
    .unwrap();
    let directory = tempdir().unwrap();
    let store = RepositoryStore::open(directory.path().join(".eggplan")).unwrap();
    let plan = create_snapshot(&store, &mapped).unwrap();
    assert_eq!(plan.status, eggplan_core::PlanStatus::Active);
    let mut stale = plan.clone();
    stale.revision += 1;
    stale.objective.push_str(" stale writer");
    let mut advanced = plan.clone();
    advanced.revision += 1;
    advanced.objective.push_str(" current writer");
    let advanced = store
        .compare_and_swap(&plan.id, plan.revision, &advanced)
        .unwrap();
    assert!(matches!(
        store.compare_and_swap(&plan.id, plan.revision, &stale),
        Err(RepoError::Conflict { .. })
    ));
    let restarted = RepositoryStore::open(directory.path().join(".eggplan")).unwrap();
    assert_eq!(restarted.get(&plan.id).unwrap(), advanced);
}

#[test]
fn concurrent_cancel_and_guarded_close_have_one_cas_winner() {
    let fixture = load(FOUNDATION);
    let mapped = normalize(
        &fixture,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Passed,
        },
    )
    .unwrap();
    let directory = tempdir().unwrap();
    let store = RepositoryStore::open(directory.path().join(".eggplan")).unwrap();
    let plan = create_snapshot(&store, &mapped).unwrap();
    for observation in &mapped.observations {
        store.append_observation(&plan.id, observation).unwrap();
    }
    let policy = vec![ProviderPolicyEntry {
        provider_id: EvidenceProviderId::new("epp_host").unwrap(),
        class: "host".into(),
        allowed_kinds: [EvidenceKind::Test].into_iter().collect(),
    }];
    let assessment = assess_plan(&plan, &subject(), &mapped.observations, &providers());
    assert_eq!(assessment.status, AssessmentStatus::Complete);
    let candidate = ClosureCandidate::build(
        &plan,
        subject(),
        assessment,
        &mapped.observations,
        &[],
        policy,
        101,
    )
    .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let close_store = store.clone();
    let close_barrier = barrier.clone();
    let close_candidate = candidate.clone();
    let close_subject = subject();
    let close = std::thread::spawn(move || {
        close_barrier.wait();
        close_store.finalize_closure(
            &close_candidate,
            &close_subject,
            ClosureId::new("epcl_race").unwrap(),
            102,
        )
    });
    let cancel_store = store.clone();
    let cancel_barrier = barrier.clone();
    let mut cancel = plan.clone();
    cancel.revision += 1;
    cancel.status = eggplan_core::PlanStatus::Cancelled;
    let cancel_id = plan.id.clone();
    let expected = plan.revision;
    let cancel_thread = std::thread::spawn(move || {
        cancel_barrier.wait();
        cancel_store.compare_and_swap(&cancel_id, expected, &cancel)
    });
    barrier.wait();
    let results = [
        close.join().unwrap().map(|_| ()),
        cancel_thread.join().unwrap().map(|_| ()),
    ];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(
        store.get(&plan.id).unwrap().status == eggplan_core::PlanStatus::Closed
            || store.get(&plan.id).unwrap().status == eggplan_core::PlanStatus::Cancelled
    );
    assert_eq!(store.get(&plan.id).unwrap().revision, 2);
}

#[test]
fn dependency_mapping_and_input_bounds_are_explicit() {
    let trajectory = normalize(
        &load(TRAJECTORY),
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Unavailable,
        },
    )
    .unwrap();
    assert_eq!(trajectory.plan.items[1].status, PlanItemStatus::Blocked);
    assert_eq!(
        trajectory.plan.items[1].dependencies,
        [trajectory.plan.items[0].id.clone()]
    );

    let mut oversized = load(FOUNDATION);
    oversized.snapshot.items[0].evidence.resize(
        eggplan_codegg_compat::MAX_EVIDENCE_REFS_PER_ITEM + 1,
        CodeggEvidenceRef {
            kind: CodeggEvidenceKind::TestJob,
            ref_id: "job-limit".into(),
            detail: None,
        },
    );
    assert!(
        normalize(
            &oversized,
            subject(),
            &mut Resolver {
                status: EvidenceStatus::Passed
            }
        )
        .is_err()
    );
    let oversized_bytes = vec![0; eggplan_codegg_compat::MAX_FIXTURE_BYTES + 1];
    assert!(Fixture::parse(&oversized_bytes).is_err());
}

#[test]
fn unavailable_artifact_and_empty_acceptance_do_not_become_complete() {
    let mut artifact = load(TRAJECTORY);
    artifact.snapshot.status = CodeggPlanStatus::Active;
    artifact.snapshot.items.truncate(1);
    artifact.snapshot.current_item_id = Some("wi_done".into());
    let mapped = normalize(
        &artifact,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Unavailable,
        },
    )
    .unwrap();
    let mut active = mapped.plan.clone();
    active.status = eggplan_core::PlanStatus::Active;
    active.revision = 1;
    assert_eq!(
        assess_plan(&active, &subject(), &mapped.observations, &providers()).status,
        AssessmentStatus::EvidenceMissingOrUnavailable
    );

    let mut no_criteria = load(PROJECTION);
    no_criteria.snapshot.status = CodeggPlanStatus::Active;
    no_criteria.snapshot.items.truncate(1);
    no_criteria.snapshot.current_item_id = None;
    no_criteria.snapshot.items[0].status = CodeggItemStatus::Completed;
    let mapped = normalize(
        &no_criteria,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Passed,
        },
    )
    .unwrap();
    let mut active = mapped.plan.clone();
    active.status = eggplan_core::PlanStatus::Active;
    active.revision = 1;
    assert_eq!(
        assess_plan(&active, &subject(), &mapped.observations, &providers()).status,
        AssessmentStatus::EvidenceMissingOrUnavailable
    );
}

#[test]
fn explicit_status_mapping_does_not_replay_transitions_or_close_completed_snapshot() {
    let fixture = load(FOUNDATION);
    let mapped = normalize(
        &fixture,
        subject(),
        &mut Resolver {
            status: EvidenceStatus::Passed,
        },
    )
    .unwrap();
    let directory = tempdir().unwrap();
    let store = RepositoryStore::open(directory.path().join(".eggplan")).unwrap();
    let plan = create_snapshot(&store, &mapped).unwrap();
    assert_eq!(plan.status, eggplan_core::PlanStatus::Active);
    assert_eq!(plan.items[0].status, PlanItemStatus::Completed);
}

#[test]
fn acceptance_disposition_enum_matches_current_codegg_contract() {
    let parsed: AcceptanceDisposition = serde_json::from_str("\"requires_user_judgment\"").unwrap();
    assert_eq!(parsed, AcceptanceDisposition::RequiresUserJudgment);
    let _: CodeggPlanSnapshot = load(FOUNDATION).snapshot;
}

#[test]
fn item_status_snapshot_mapping_preserves_state_without_transition_replay() {
    let table = [
        (CodeggItemStatus::Pending, PlanItemStatus::Pending),
        (CodeggItemStatus::Actionable, PlanItemStatus::Actionable),
        (CodeggItemStatus::InProgress, PlanItemStatus::InProgress),
        (CodeggItemStatus::Blocked, PlanItemStatus::Blocked),
        (CodeggItemStatus::Completed, PlanItemStatus::Completed),
        (CodeggItemStatus::Cancelled, PlanItemStatus::Cancelled),
    ];
    for (source, expected) in table {
        let mut fixture = load(PROJECTION);
        fixture.snapshot.items[0].status = source;
        fixture.snapshot.items[0].blocker =
            (source == CodeggItemStatus::Blocked).then(|| "source blocker".into());
        let mapped = normalize(
            &fixture,
            subject(),
            &mut Resolver {
                status: EvidenceStatus::Passed,
            },
        )
        .unwrap();
        assert_eq!(mapped.plan.items[0].status, expected);
    }
}

#[test]
fn plan_status_mapping_keeps_completed_outside_closed_and_preserves_terminal_intent() {
    let table = [
        (CodeggPlanStatus::Active, eggplan_core::PlanStatus::Active),
        (CodeggPlanStatus::Blocked, eggplan_core::PlanStatus::Blocked),
        (
            CodeggPlanStatus::Completed,
            eggplan_core::PlanStatus::Active,
        ),
        (
            CodeggPlanStatus::Cancelled,
            eggplan_core::PlanStatus::Cancelled,
        ),
    ];
    for (source, expected) in table {
        let mut fixture = load(FOUNDATION);
        fixture.snapshot.status = source;
        let mapped = normalize(
            &fixture,
            subject(),
            &mut Resolver {
                status: EvidenceStatus::Passed,
            },
        )
        .unwrap();
        let directory = tempdir().unwrap();
        let store = RepositoryStore::open(directory.path().join(".eggplan")).unwrap();
        let stored = create_snapshot(&store, &mapped).unwrap();
        assert_eq!(stored.status, expected);
        if source == CodeggPlanStatus::Completed {
            assert!(
                mapped
                    .manifest
                    .losses
                    .contains(&eggplan_codegg_compat::Loss::CompletedPlanNeedsGuardedClose)
            );
        }
    }
}
