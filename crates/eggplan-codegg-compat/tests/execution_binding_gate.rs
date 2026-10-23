//! Pins the CodeGG bridge's verification-binding requirement to the canonical
//! core predicate `eggplan_core::is_execution_evidence`.
//!
//! The bridge used to carry its own copy of the execution-derived kind set. It
//! now consumes the core predicate, so this test asserts that a mapped CodeGG
//! evidence reference demands a verification binding exactly when the core
//! classification says the mapped kind is execution-derived.

use eggplan_codegg_compat::{
    CodeggEvidenceKind, CodeggEvidenceRef, CodeggItemSnapshot, CodeggItemStatus,
    CodeggPlanSnapshot, CodeggPlanStatus, EvidenceResolver, ResolvedEvidence, normalize_snapshot,
};
use eggplan_core::{
    EvidenceKind, EvidenceObservation, EvidenceObservationId, EvidenceObservationInput,
    EvidenceProviderId, EvidenceStatus, SubjectRevision, SubjectState, VerificationDigest,
};

const ALL_CODEGG_KINDS: [CodeggEvidenceKind; 6] = [
    CodeggEvidenceKind::TestJob,
    CodeggEvidenceKind::DelegatedRun,
    CodeggEvidenceKind::SchedulerJob,
    CodeggEvidenceKind::AgentRun,
    CodeggEvidenceKind::Artifact,
    CodeggEvidenceKind::Commit,
];

/// Mirrors the bridge's private `map_kind` so the test can state the expected
/// classification independently of the crate's implementation.
fn mapped_kind(kind: CodeggEvidenceKind) -> EvidenceKind {
    match kind {
        CodeggEvidenceKind::TestJob => EvidenceKind::Test,
        CodeggEvidenceKind::DelegatedRun | CodeggEvidenceKind::AgentRun => {
            EvidenceKind::DelegatedRun
        }
        CodeggEvidenceKind::SchedulerJob => EvidenceKind::Command,
        CodeggEvidenceKind::Artifact => EvidenceKind::Artifact,
        CodeggEvidenceKind::Commit => EvidenceKind::Revision,
    }
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

/// A resolver that never supplies the host's authoritative expected binding.
///
/// Core requires an execution-kind observation to carry *some* verification
/// digest, so the resolver attaches the canonical digest for execution-derived
/// kinds and leaves every other kind unbound. What it never does is return the
/// authoritative `expected_verification`, which is exactly the situation the
/// bridge's binding gate exists to reject.
struct UnboundResolver;

impl EvidenceResolver for UnboundResolver {
    fn resolve(
        &mut self,
        reference: &CodeggEvidenceRef,
        subject: &SubjectRevision,
    ) -> Result<ResolvedEvidence, String> {
        let kind = mapped_kind(reference.kind);
        let binding = eggplan_core::is_execution_evidence(kind)
            .then(|| VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap());
        let observation = EvidenceObservation::finalize(EvidenceObservationInput {
            id: EvidenceObservationId::new(format!("epe_{}", reference.ref_id)).unwrap(),
            provider_id: EvidenceProviderId::new("epp_host").unwrap(),
            kind,
            status: EvidenceStatus::Passed,
            subject: subject.clone(),
            observed_at_unix_ms: 100,
            invocation_ref: None,
            verification_digest: binding,
            result_metadata: Default::default(),
            artifacts: vec![],
        })
        .map_err(|error| error.to_string())?;
        Ok(ResolvedEvidence {
            observation,
            expected_verification: None,
        })
    }
}

fn snapshot_with(kind: CodeggEvidenceKind) -> CodeggPlanSnapshot {
    CodeggPlanSnapshot {
        plan_id: "wp_binding_gate".into(),
        revision: 0,
        status: CodeggPlanStatus::Active,
        objective: "Exercise the bridge binding gate".into(),
        current_phase: None,
        current_item_id: None,
        items: vec![CodeggItemSnapshot {
            item_id: "wi_binding_gate".into(),
            position: 0,
            parent_item_id: None,
            dependencies: vec![],
            status: CodeggItemStatus::Actionable,
            description: "One evidence reference".into(),
            acceptance: vec![],
            evidence: vec![CodeggEvidenceRef {
                kind,
                ref_id: "ref_binding_gate".into(),
                detail: None,
            }],
            owner_run_id: None,
            owner_job_id: None,
            blocker: None,
            next_action: None,
        }],
    }
}

#[test]
fn bridge_binding_requirement_matches_the_canonical_core_predicate() {
    for kind in ALL_CODEGG_KINDS {
        let mut resolver = UnboundResolver;
        let outcome = normalize_snapshot(&snapshot_with(kind), subject(), &mut resolver);
        let execution_derived = eggplan_core::is_execution_evidence(mapped_kind(kind));
        if execution_derived {
            match outcome {
                Err(message) => assert_eq!(
                    message,
                    "execution evidence lacks the authoritative matching verification binding",
                    "mapped kind {kind:?} is execution-derived and must demand a binding"
                ),
                Ok(_) => {
                    panic!("mapped kind {kind:?} is execution-derived and must demand a binding")
                }
            }
        } else {
            assert!(
                outcome.is_ok(),
                "mapped kind {kind:?} is not execution-derived and must not demand a binding, got {outcome:?}"
            );
        }
    }
}
