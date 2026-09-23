use crate::{
    AssessmentStatus::*, EvidenceCardinality, EvidenceKind, EvidenceObservation,
    EvidenceObservationId, EvidenceProviderId, EvidenceRequirement, EvidenceStatus, Plan, PlanId,
    PlanItem, PlanItemId, PlanItemStatus, PlanStatus, ProviderRegistry, SubjectRevision,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Complete,
    ActionableWorkRemaining,
    Blocked,
    EvidenceFailed,
    EvidenceMissingOrUnavailable,
    InFlight,
    AwaitingHumanJudgment,
    Inconclusive,
    InvalidOrStale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum AssessmentReason {
    MissingObservation,
    NoAcceptanceCriteria,
    NoEvidenceRequirement,
    HumanJudgmentRequired,
    StaleSubject(EvidenceObservationId),
    UntrustedProvider(EvidenceProviderId),
    ProviderKindNotAllowed(EvidenceProviderId),
    ProviderMismatch(EvidenceProviderId),
    LegacyUnboundExecutionRequirement,
    ObservationMissingVerificationBinding(EvidenceObservationId),
    VerificationDigestMismatch(EvidenceObservationId),
    EvidenceStatus(EvidenceObservationId, EvidenceStatus),
    HumanJudgmentNotAllowed,
    InvalidObservation(EvidenceObservationId),
    DuplicateObservationId(EvidenceObservationId),
    PlanNotActive,
    InvalidPlan,
    ItemNotCompleted(PlanItemStatus),
    PlanBlocked,
    PlanCancelled,
    NoPlanItems,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementAssessment {
    pub status: AssessmentStatus,
    pub requirement_kind: EvidenceKind,
    pub satisfying_observation_ids: Vec<EvidenceObservationId>,
    pub reasons: Vec<AssessmentReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionAssessment {
    pub criterion_id: crate::CriterionId,
    pub status: AssessmentStatus,
    pub requirements: Vec<RequirementAssessment>,
    pub reasons: Vec<AssessmentReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemAssessment {
    pub item_id: PlanItemId,
    pub status: AssessmentStatus,
    pub criteria: Vec<CriterionAssessment>,
    pub reasons: Vec<AssessmentReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanAssessment {
    pub plan_id: PlanId,
    pub plan_revision: u64,
    pub subject: SubjectRevision,
    pub status: AssessmentStatus,
    pub items: Vec<ItemAssessment>,
    pub reasons: Vec<AssessmentReason>,
}

/// Pure deterministic assessment. The provider registry is explicit host
/// policy: observation payload text cannot add or widen trusted providers.
pub fn assess_plan(
    plan: &Plan,
    current_subject: &SubjectRevision,
    observations: &[EvidenceObservation],
    providers: &ProviderRegistry,
) -> PlanAssessment {
    let mut reasons = Vec::new();
    let plan_valid = plan.validate().is_ok() && current_subject.validate().is_ok();
    if !plan_valid {
        reasons.push(AssessmentReason::InvalidPlan);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut sorted = observations.to_vec();
    sorted.sort_by(|a, b| a.id().cmp(b.id()));
    let mut duplicate_ids = false;
    for observation in &sorted {
        if !seen.insert(observation.id().clone()) {
            duplicate_ids = true;
            reasons.push(AssessmentReason::DuplicateObservationId(
                observation.id().clone(),
            ));
        }
    }

    let mut items = Vec::with_capacity(plan.items.len());
    for item in &plan.items {
        items.push(assess_item(item, current_subject, &sorted, providers));
    }
    let mut candidates: Vec<AssessmentStatus> = items.iter().map(|item| item.status).collect();
    if !plan_valid {
        candidates.push(InvalidOrStale);
    }
    if duplicate_ids {
        candidates.push(InvalidOrStale);
    }
    match plan.status {
        PlanStatus::Blocked => {
            reasons.push(AssessmentReason::PlanBlocked);
            candidates.push(Blocked);
        }
        PlanStatus::Cancelled => {
            reasons.push(AssessmentReason::PlanCancelled);
            candidates.push(InvalidOrStale);
        }
        PlanStatus::Draft => {
            reasons.push(AssessmentReason::PlanNotActive);
            candidates.push(ActionableWorkRemaining);
        }
        PlanStatus::Active | PlanStatus::Closed => {}
    }
    if plan.items.is_empty() {
        reasons.push(AssessmentReason::NoPlanItems);
        candidates.push(ActionableWorkRemaining);
    }
    if plan.status == PlanStatus::Closed
        && items.iter().all(|item| item.status == Complete)
        && plan_valid
        && !plan.items.is_empty()
    {
        candidates.push(Complete);
    }
    let status = highest(candidates);
    PlanAssessment {
        plan_id: plan.id.clone(),
        plan_revision: plan.revision,
        subject: current_subject.clone(),
        status,
        items,
        reasons,
    }
}

fn assess_item(
    item: &PlanItem,
    current: &SubjectRevision,
    observations: &[EvidenceObservation],
    providers: &ProviderRegistry,
) -> ItemAssessment {
    let mut criteria = Vec::with_capacity(item.criteria.len());
    let mut reasons = Vec::new();
    for criterion in &item.criteria {
        let mut requirements = Vec::with_capacity(criterion.requirements.len());
        if criterion.requirements.is_empty() {
            let status = if criterion.human_judgment_allowed {
                AwaitingHumanJudgment
            } else {
                EvidenceMissingOrUnavailable
            };
            let reason = if criterion.human_judgment_allowed {
                AssessmentReason::HumanJudgmentRequired
            } else {
                AssessmentReason::NoEvidenceRequirement
            };
            requirements.push(RequirementAssessment {
                status,
                requirement_kind: EvidenceKind::HumanJudgment,
                satisfying_observation_ids: vec![],
                reasons: vec![reason],
            });
        } else {
            for requirement in &criterion.requirements {
                requirements.push(assess_requirement(
                    requirement,
                    criterion.human_judgment_allowed,
                    current,
                    observations,
                    providers,
                ));
            }
        }
        let status = highest(requirements.iter().map(|r| r.status).collect());
        let criterion_reasons: Vec<AssessmentReason> = requirements
            .iter()
            .flat_map(|r| r.reasons.clone())
            .collect();
        reasons.extend(criterion_reasons.iter().cloned());
        criteria.push(CriterionAssessment {
            criterion_id: criterion.id.clone(),
            status,
            requirements,
            reasons: criterion_reasons,
        });
    }

    let status = if item.criteria.is_empty() {
        reasons.push(AssessmentReason::NoAcceptanceCriteria);
        EvidenceMissingOrUnavailable
    } else {
        let criteria_status = highest(criteria.iter().map(|criterion| criterion.status).collect());
        match item.status {
            PlanItemStatus::Blocked => Blocked,
            PlanItemStatus::InProgress if criteria_status == Complete => InFlight,
            PlanItemStatus::Pending | PlanItemStatus::Actionable if criteria_status == Complete => {
                ActionableWorkRemaining
            }
            PlanItemStatus::Completed if criteria_status == Complete => Complete,
            PlanItemStatus::Cancelled if criteria_status == Complete => ActionableWorkRemaining,
            _ => criteria_status,
        }
    };
    if item.status != PlanItemStatus::Completed {
        reasons.push(AssessmentReason::ItemNotCompleted(item.status));
    }
    ItemAssessment {
        item_id: item.id.clone(),
        status,
        criteria,
        reasons,
    }
}

fn assess_requirement(
    requirement: &EvidenceRequirement,
    criterion_allows_human: bool,
    current: &SubjectRevision,
    observations: &[EvidenceObservation],
    providers: &ProviderRegistry,
) -> RequirementAssessment {
    let mut reasons = Vec::new();
    if crate::is_execution_evidence(requirement.kind)
        && requirement.expected_verification_digest.is_none()
    {
        return RequirementAssessment {
            status: InvalidOrStale,
            requirement_kind: requirement.kind,
            satisfying_observation_ids: vec![],
            reasons: vec![AssessmentReason::LegacyUnboundExecutionRequirement],
        };
    }
    let mut eligible = Vec::new();
    let mut invalid = false;
    for observation in observations
        .iter()
        .filter(|observation| observation.kind() == requirement.kind)
    {
        let Some(provider) = providers.get(observation.provider_id()) else {
            reasons.push(AssessmentReason::UntrustedProvider(
                observation.provider_id().clone(),
            ));
            invalid = true;
            continue;
        };
        if !provider.allowed_kinds.contains(&observation.kind()) {
            reasons.push(AssessmentReason::ProviderKindNotAllowed(
                provider.id.clone(),
            ));
            invalid = true;
            continue;
        }
        if let Some(required_provider) = &requirement.provider
            && required_provider != observation.provider_id()
        {
            reasons.push(AssessmentReason::ProviderMismatch(
                observation.provider_id().clone(),
            ));
            continue;
        }
        if observation.kind() == EvidenceKind::HumanJudgment
            && (!requirement.allow_human_judgment
                || !criterion_allows_human
                || provider.class != "human")
        {
            reasons.push(AssessmentReason::HumanJudgmentNotAllowed);
            invalid = true;
            continue;
        }
        if observation.validate().is_err() {
            reasons.push(AssessmentReason::InvalidObservation(
                observation.id().clone(),
            ));
            invalid = true;
            continue;
        }
        if observation.subject() != current {
            reasons.push(AssessmentReason::StaleSubject(observation.id().clone()));
            continue;
        }
        if let Some(expected) = &requirement.expected_verification_digest {
            match observation.verification_digest() {
                None => {
                    reasons.push(AssessmentReason::ObservationMissingVerificationBinding(
                        observation.id().clone(),
                    ));
                    continue;
                }
                Some(actual) if actual != expected => {
                    reasons.push(AssessmentReason::VerificationDigestMismatch(
                        observation.id().clone(),
                    ));
                    continue;
                }
                Some(_) => {}
            }
        }
        eligible.push(observation);
    }

    let passed: Vec<_> = eligible
        .iter()
        .filter(|o| o.status() == EvidenceStatus::Passed)
        .map(|o| o.id().clone())
        .collect();
    let enough_passed = passed.len() >= requirement.min_count as usize;
    let all_passed = eligible.len() >= requirement.min_count as usize
        && eligible
            .iter()
            .all(|o| o.status() == EvidenceStatus::Passed);
    let satisfied = match requirement.cardinality {
        EvidenceCardinality::Any => enough_passed,
        EvidenceCardinality::All => all_passed,
    };
    let status = if satisfied {
        Complete
    } else if invalid {
        InvalidOrStale
    } else if eligible
        .iter()
        .any(|o| o.status() == EvidenceStatus::Failed)
    {
        EvidenceFailed
    } else if eligible
        .iter()
        .any(|o| o.status() == EvidenceStatus::Blocked)
    {
        Blocked
    } else if eligible
        .iter()
        .any(|o| o.status() == EvidenceStatus::InProgress)
    {
        InFlight
    } else if eligible
        .iter()
        .any(|o| o.status() == EvidenceStatus::Inconclusive)
    {
        Inconclusive
    } else if eligible.iter().any(|o| {
        matches!(
            o.status(),
            EvidenceStatus::NotRun | EvidenceStatus::Skipped | EvidenceStatus::Unavailable
        )
    }) {
        EvidenceMissingOrUnavailable
    } else if !reasons.is_empty()
        && reasons
            .iter()
            .any(|reason| matches!(reason, AssessmentReason::StaleSubject(_)))
    {
        InvalidOrStale
    } else {
        EvidenceMissingOrUnavailable
    };

    for observation in &eligible {
        reasons.push(AssessmentReason::EvidenceStatus(
            observation.id().clone(),
            observation.status(),
        ));
    }
    if eligible.is_empty() && reasons.is_empty() {
        reasons.push(AssessmentReason::MissingObservation);
    }
    let satisfying = if satisfied { passed } else { vec![] };
    RequirementAssessment {
        status,
        requirement_kind: requirement.kind,
        satisfying_observation_ids: satisfying,
        reasons,
    }
}

fn highest(statuses: Vec<AssessmentStatus>) -> AssessmentStatus {
    statuses
        .into_iter()
        .max_by_key(|status| match status {
            InvalidOrStale => 9,
            EvidenceFailed => 8,
            Blocked => 7,
            InFlight => 6,
            EvidenceMissingOrUnavailable => 5,
            AwaitingHumanJudgment => 4,
            Inconclusive => 3,
            ActionableWorkRemaining => 2,
            Complete => 1,
        })
        .unwrap_or(Complete)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AcceptanceCriterion, CriterionId, EvidenceObservation, EvidenceObservationInput,
        EvidenceProviderId, PlanItem, SubjectState,
    };
    use std::collections::BTreeMap;

    fn subject(state: SubjectState, revision: &str) -> SubjectRevision {
        SubjectRevision {
            subject_kind: "git".into(),
            repository_id: "epr_test".into(),
            revision: revision.into(),
            state,
            dirty_digest: (state == SubjectState::Dirty)
                .then(|| format!("sha256:{}", "a".repeat(64))),
        }
    }
    fn requirement(
        kind: EvidenceKind,
        cardinality: EvidenceCardinality,
        min_count: u16,
    ) -> EvidenceRequirement {
        EvidenceRequirement {
            description: "run check".into(),
            kind,
            provider: None,
            subject_policy: crate::SubjectPolicy::Exact,
            cardinality,
            min_count,
            allow_human_judgment: false,
            expected_verification_digest: Some(
                crate::VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
            ),
        }
    }
    fn plan(status: PlanItemStatus, requirements: Vec<EvidenceRequirement>, human: bool) -> Plan {
        Plan {
            schema_version: crate::SCHEMA_VERSION,
            id: PlanId::new("ep_assess").unwrap(),
            revision: 0,
            objective: "assess".into(),
            status: PlanStatus::Active,
            provenance: BTreeMap::new(),
            items: vec![PlanItem {
                id: PlanItemId::new("epi_item").unwrap(),
                position: 0,
                parent: None,
                dependencies: vec![],
                status,
                description: "check".into(),
                criteria: vec![AcceptanceCriterion {
                    id: CriterionId::new("epc_test").unwrap(),
                    statement: "passes".into(),
                    human_judgment_allowed: human,
                    requirements,
                }],
                blocker: None,
                next_action: None,
            }],
            subject: None,
        }
    }
    fn observation(
        id: &str,
        kind: EvidenceKind,
        status: EvidenceStatus,
        subject: SubjectRevision,
        provider: &str,
    ) -> EvidenceObservation {
        observation_with_digest(id, kind, status, subject, provider, "a")
    }
    fn observation_with_digest(
        id: &str,
        kind: EvidenceKind,
        status: EvidenceStatus,
        subject: SubjectRevision,
        provider: &str,
        digest_byte: &str,
    ) -> EvidenceObservation {
        EvidenceObservation::finalize(EvidenceObservationInput {
            id: EvidenceObservationId::new(format!("epe_{id}")).unwrap(),
            provider_id: EvidenceProviderId::new(format!("epp_{provider}")).unwrap(),
            kind,
            status,
            subject,
            observed_at_unix_ms: 1,
            invocation_ref: None,
            verification_digest: Some(
                crate::VerificationDigest::new(format!("sha256:{}", digest_byte.repeat(64)))
                    .unwrap(),
            ),
            result_metadata: BTreeMap::new(),
            artifacts: vec![],
        })
        .unwrap()
    }

    #[test]
    fn execution_evidence_requires_exact_verification_binding() {
        let current = subject(SubjectState::Clean, "bound");
        let p = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1)],
            false,
        );
        let providers = registry("host", "host", &[EvidenceKind::Test]);
        let wrong = observation_with_digest(
            "wrong",
            EvidenceKind::Test,
            EvidenceStatus::Passed,
            current.clone(),
            "host",
            "b",
        );
        let wrong_result = assess_plan(&p, &current, &[wrong], &providers);
        let requirement_result = &wrong_result.items[0].criteria[0].requirements[0];
        assert_ne!(wrong_result.status, AssessmentStatus::Complete);
        assert!(requirement_result.satisfying_observation_ids.is_empty());
        assert!(
            requirement_result
                .reasons
                .iter()
                .any(|r| matches!(r, AssessmentReason::VerificationDigestMismatch(_)))
        );

        let exact = observation(
            "exact",
            EvidenceKind::Test,
            EvidenceStatus::Passed,
            current.clone(),
            "host",
        );
        assert_eq!(
            assess_plan(&p, &current, &[exact], &providers).status,
            AssessmentStatus::Complete
        );
    }

    #[test]
    fn mismatched_observations_are_excluded_from_any_and_all_cardinality() {
        let current = subject(SubjectState::Clean, "cardinality");
        let providers = registry("host", "host", &[EvidenceKind::Test]);
        let passing_x = observation_with_digest(
            "pass_x",
            EvidenceKind::Test,
            EvidenceStatus::Passed,
            current.clone(),
            "host",
            "a",
        );
        let failed_y = observation_with_digest(
            "fail_y",
            EvidenceKind::Test,
            EvidenceStatus::Failed,
            current.clone(),
            "host",
            "b",
        );

        for cardinality in [EvidenceCardinality::Any, EvidenceCardinality::All] {
            let p = plan(
                PlanItemStatus::Completed,
                vec![requirement(EvidenceKind::Test, cardinality, 1)],
                false,
            );
            let result = assess_plan(
                &p,
                &current,
                &[passing_x.clone(), failed_y.clone()],
                &providers,
            );
            assert_eq!(result.status, AssessmentStatus::Complete);
            assert_eq!(
                result.items[0].criteria[0].requirements[0].satisfying_observation_ids,
                vec![EvidenceObservationId::new("epe_pass_x").unwrap()]
            );
        }
    }

    #[test]
    fn legacy_unbound_execution_requirement_fails_closed() {
        let mut req = requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1);
        req.expected_verification_digest = None;
        let mut p = plan(PlanItemStatus::Completed, vec![req], false);
        p.schema_version = 1;
        let current = subject(SubjectState::Clean, "legacy");
        let observation = observation(
            "legacy",
            EvidenceKind::Test,
            EvidenceStatus::Passed,
            current.clone(),
            "host",
        );
        let result = assess_plan(
            &p,
            &current,
            &[observation],
            &registry("host", "host", &[EvidenceKind::Test]),
        );
        assert_ne!(result.status, AssessmentStatus::Complete);
        assert!(
            result.items[0].criteria[0].requirements[0]
                .reasons
                .contains(&AssessmentReason::LegacyUnboundExecutionRequirement)
        );
    }

    #[test]
    fn legacy_unbound_observation_cannot_satisfy_a_v2_requirement() {
        let current = subject(SubjectState::Clean, "legacy-observation");
        let p = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1)],
            false,
        );
        let legacy = crate::evidence::finalize_v1_for_test(EvidenceObservationInput {
            id: EvidenceObservationId::new("epe_legacy").unwrap(),
            provider_id: EvidenceProviderId::new("epp_host").unwrap(),
            kind: EvidenceKind::Test,
            status: EvidenceStatus::Passed,
            subject: current.clone(),
            observed_at_unix_ms: 1,
            invocation_ref: Some("cargo test".into()),
            verification_digest: None,
            result_metadata: BTreeMap::new(),
            artifacts: vec![],
        });
        let result = assess_plan(
            &p,
            &current,
            &[legacy],
            &registry("host", "host", &[EvidenceKind::Test]),
        );
        let requirement_result = &result.items[0].criteria[0].requirements[0];
        assert_ne!(result.status, AssessmentStatus::Complete);
        assert!(requirement_result.satisfying_observation_ids.is_empty());
        assert!(requirement_result.reasons.iter().any(|r| matches!(
            r,
            AssessmentReason::ObservationMissingVerificationBinding(_)
        )));
    }
    fn registry(provider: &str, class: &str, kinds: &[EvidenceKind]) -> ProviderRegistry {
        let mut registry = ProviderRegistry::default();
        registry
            .register_trusted(
                crate::ProviderDescriptor::new(
                    EvidenceProviderId::new(format!("epp_{provider}")).unwrap(),
                    class,
                    kinds.iter().copied(),
                )
                .unwrap(),
            )
            .unwrap();
        registry
    }

    #[test]
    fn each_status_maps_to_distinct_deterministic_assessment() {
        let current = subject(SubjectState::Clean, "a");
        for (evidence, expected) in [
            (EvidenceStatus::Passed, AssessmentStatus::Complete),
            (EvidenceStatus::Failed, AssessmentStatus::EvidenceFailed),
            (EvidenceStatus::InProgress, AssessmentStatus::InFlight),
            (
                EvidenceStatus::NotRun,
                AssessmentStatus::EvidenceMissingOrUnavailable,
            ),
            (
                EvidenceStatus::Skipped,
                AssessmentStatus::EvidenceMissingOrUnavailable,
            ),
            (EvidenceStatus::Blocked, AssessmentStatus::Blocked),
            (
                EvidenceStatus::Unavailable,
                AssessmentStatus::EvidenceMissingOrUnavailable,
            ),
            (EvidenceStatus::Inconclusive, AssessmentStatus::Inconclusive),
        ] {
            let p = plan(
                PlanItemStatus::Completed,
                vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1)],
                false,
            );
            let obs = observation("one", EvidenceKind::Test, evidence, current.clone(), "host");
            let providers = registry("host", "host", &[EvidenceKind::Test]);
            assert_eq!(
                assess_plan(&p, &current, std::slice::from_ref(&obs), &providers),
                assess_plan(&p, &current, std::slice::from_ref(&obs), &providers)
            );
            assert_eq!(
                assess_plan(
                    &p,
                    &current,
                    &[observation(
                        "one",
                        EvidenceKind::Test,
                        evidence,
                        current.clone(),
                        "host"
                    )],
                    &providers
                )
                .status,
                expected
            );
        }
    }

    #[test]
    fn stale_clean_dirty_subject_and_untrusted_provider_never_pass() {
        let current = subject(SubjectState::Dirty, "a");
        let p = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1)],
            false,
        );
        let stale = observation(
            "stale",
            EvidenceKind::Test,
            EvidenceStatus::Passed,
            subject(SubjectState::Clean, "a"),
            "host",
        );
        let trusted = registry("host", "host", &[EvidenceKind::Test]);
        assert_eq!(
            assess_plan(&p, &current, &[stale], &trusted).status,
            AssessmentStatus::InvalidOrStale
        );
        let stale_dirty = observation(
            "stale_dirty",
            EvidenceKind::Test,
            EvidenceStatus::Passed,
            SubjectRevision {
                state: SubjectState::Dirty,
                dirty_digest: Some(format!("sha256:{}", "b".repeat(64))),
                ..current.clone()
            },
            "host",
        );
        assert_eq!(
            assess_plan(&p, &current, &[stale_dirty], &trusted).status,
            AssessmentStatus::InvalidOrStale
        );
        let forged = observation(
            "forged",
            EvidenceKind::Test,
            EvidenceStatus::Passed,
            current.clone(),
            "spoof",
        );
        assert_eq!(
            assess_plan(&p, &current, &[forged], &trusted).status,
            AssessmentStatus::InvalidOrStale
        );
    }

    #[test]
    fn completed_without_proof_and_missing_evidence_do_not_close() {
        let current = subject(SubjectState::Clean, "a");
        let p = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1)],
            false,
        );
        let providers = registry("host", "host", &[EvidenceKind::Test]);
        assert_eq!(
            assess_plan(&p, &current, &[], &providers).status,
            AssessmentStatus::EvidenceMissingOrUnavailable
        );
        let p = plan(PlanItemStatus::Completed, vec![], false);
        assert_eq!(
            assess_plan(&p, &current, &[], &providers).status,
            AssessmentStatus::EvidenceMissingOrUnavailable
        );
    }

    #[test]
    fn closed_plan_with_unmet_criterion_is_not_complete() {
        let current = subject(SubjectState::Clean, "a");
        let mut p = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1)],
            false,
        );
        p.status = PlanStatus::Closed;
        let providers = registry("host", "host", &[EvidenceKind::Test]);
        assert_eq!(
            assess_plan(&p, &current, &[], &providers).status,
            AssessmentStatus::EvidenceMissingOrUnavailable
        );
    }

    #[test]
    fn any_all_cardinality_and_human_policy_are_explicit() {
        let current = subject(SubjectState::Clean, "a");
        let observations = [
            observation(
                "pass",
                EvidenceKind::Test,
                EvidenceStatus::Passed,
                current.clone(),
                "host",
            ),
            observation(
                "fail",
                EvidenceKind::Test,
                EvidenceStatus::Failed,
                current.clone(),
                "host",
            ),
        ];
        let providers = registry("host", "host", &[EvidenceKind::Test]);
        let any = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1)],
            false,
        );
        let all = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::All, 1)],
            false,
        );
        assert_eq!(
            assess_plan(&any, &current, &observations, &providers).status,
            AssessmentStatus::Complete
        );
        assert_eq!(
            assess_plan(&all, &current, &observations, &providers).status,
            AssessmentStatus::EvidenceFailed
        );

        let mut human_req = requirement(EvidenceKind::HumanJudgment, EvidenceCardinality::Any, 1);
        human_req.allow_human_judgment = true;
        let human_plan = plan(PlanItemStatus::Completed, vec![human_req], true);
        let human_obs = observation(
            "judge",
            EvidenceKind::HumanJudgment,
            EvidenceStatus::Passed,
            current.clone(),
            "reviewer",
        );
        let human_providers = registry("reviewer", "human", &[EvidenceKind::HumanJudgment]);
        assert_eq!(
            assess_plan(&human_plan, &current, &[human_obs], &human_providers).status,
            AssessmentStatus::Complete
        );
        let not_allowed = plan(
            PlanItemStatus::Completed,
            vec![requirement(
                EvidenceKind::HumanJudgment,
                EvidenceCardinality::Any,
                1,
            )],
            false,
        );
        assert!(not_allowed.validate().is_err());
    }

    #[test]
    fn deterministic_reason_codes_have_stable_observation_order() {
        let current = subject(SubjectState::Clean, "a");
        let p = plan(
            PlanItemStatus::Completed,
            vec![requirement(EvidenceKind::Test, EvidenceCardinality::Any, 2)],
            false,
        );
        let a = observation(
            "aaa",
            EvidenceKind::Test,
            EvidenceStatus::NotRun,
            current.clone(),
            "host",
        );
        let b = observation(
            "bbb",
            EvidenceKind::Test,
            EvidenceStatus::Skipped,
            current.clone(),
            "host",
        );
        let providers = registry("host", "host", &[EvidenceKind::Test]);
        let first = assess_plan(&p, &current, &[b.clone(), a.clone()], &providers);
        let second = assess_plan(&p, &current, &[a, b], &providers);
        assert_eq!(first, second);
    }

    #[test]
    fn provider_mismatch_and_mixed_criteria_remain_unsatisfied() {
        let current = subject(SubjectState::Clean, "a");
        let mut first = requirement(EvidenceKind::Test, EvidenceCardinality::Any, 1);
        first.provider = Some(EvidenceProviderId::new("epp_expected").unwrap());
        let second = requirement(EvidenceKind::StaticAnalysis, EvidenceCardinality::Any, 1);
        let p = plan(PlanItemStatus::Completed, vec![first, second], false);
        let observations = [
            observation(
                "wrong_provider",
                EvidenceKind::Test,
                EvidenceStatus::Passed,
                current.clone(),
                "other",
            ),
            observation(
                "test_pass",
                EvidenceKind::Test,
                EvidenceStatus::Passed,
                current.clone(),
                "expected",
            ),
        ];
        let mut providers = registry("other", "host", &[EvidenceKind::Test]);
        providers
            .register_trusted(
                crate::ProviderDescriptor::new(
                    EvidenceProviderId::new("epp_expected").unwrap(),
                    "host",
                    [EvidenceKind::Test],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            assess_plan(&p, &current, &observations, &providers).status,
            AssessmentStatus::EvidenceMissingOrUnavailable
        );
        assert_eq!(
            assess_plan(&p, &current, &observations, &providers),
            assess_plan(&p, &current, &observations, &providers)
        );
    }
}
