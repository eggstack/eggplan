//! Pins the provider SPI's verification-binding gate to the canonical core
//! predicate `eggplan_core::is_execution_evidence`.
//!
//! The SPI used to carry its own copy of the execution-derived kind set. It
//! now consumes the core predicate, so this test asserts the gate's verdict
//! for every `EvidenceKind` variant matches the canonical classification
//! exactly — no variant may be silently accepted or rejected by the SPI.

use eggplan_core::{
    EvidenceKind, EvidenceObservationId, EvidenceProviderId, EvidenceStatus, SubjectRevision,
    SubjectState, VerificationDigest,
};
use eggplan_integrations::{
    AdapterDescriptor, Capabilities, NormalizedProviderResult, ObservationContext, ProviderClass,
    SpiError, finalize_observation,
};
use std::collections::BTreeMap;

const ALL_KINDS: [EvidenceKind; 10] = [
    EvidenceKind::Command,
    EvidenceKind::Test,
    EvidenceKind::StaticAnalysis,
    EvidenceKind::Revision,
    EvidenceKind::Artifact,
    EvidenceKind::DelegatedRun,
    EvidenceKind::Benchmark,
    EvidenceKind::Research,
    EvidenceKind::HumanJudgment,
    EvidenceKind::Attestation,
];

fn descriptor(kind: EvidenceKind) -> AdapterDescriptor {
    // A valid descriptor can never allow `HumanJudgment` — `AdapterDescriptor`
    // rejects it — so that kind is exercised through a descriptor that omits
    // it. The SPI then rejects the request as an unsupported kind, which is
    // not a verification-binding rejection.
    let allowed = if kind == EvidenceKind::HumanJudgment {
        EvidenceKind::Revision
    } else {
        kind
    };
    AdapterDescriptor::new(
        EvidenceProviderId::new("epp_binding_gate").unwrap(),
        ProviderClass::Execution,
        [allowed],
        "1.0",
        Capabilities {
            supports_in_progress: true,
            artifacts: true,
            verification_binding: true,
            research_trust_metadata: true,
        },
    )
    .unwrap()
}

fn context(kind: EvidenceKind, binding: Option<VerificationDigest>) -> ObservationContext {
    ObservationContext {
        observation_id: EvidenceObservationId::new("epe_binding_gate").unwrap(),
        subject: SubjectRevision {
            subject_kind: "git".into(),
            repository_id: "epr_binding_gate".into(),
            revision: "abc123".into(),
            state: SubjectState::Clean,
            dirty_digest: None,
        },
        requested_kind: kind,
        observed_at_unix_ms: 1_700_000_000_000,
        invocation_ref: None,
        verification_digest: binding,
        metadata: BTreeMap::new(),
    }
}

fn result() -> NormalizedProviderResult {
    NormalizedProviderResult {
        status: EvidenceStatus::Passed,
        source_trust: None,
        result_metadata: BTreeMap::new(),
        artifacts: vec![],
    }
}

#[test]
fn missing_binding_gate_matches_the_canonical_core_predicate_for_every_kind() {
    for kind in ALL_KINDS {
        let outcome = finalize_observation(&descriptor(kind), &context(kind, None), &result());
        if eggplan_core::is_execution_evidence(kind) {
            assert_eq!(
                outcome,
                Err(SpiError::MissingVerificationBinding),
                "execution-derived kind {kind:?} must still require a verification binding"
            );
        } else {
            assert!(
                !matches!(outcome, Err(SpiError::MissingVerificationBinding)),
                "non-execution kind {kind:?} must not be rejected for a missing binding"
            );
        }
    }
}

#[test]
fn a_non_execution_kind_is_finalized_without_a_binding() {
    for kind in [
        EvidenceKind::Revision,
        EvidenceKind::Artifact,
        EvidenceKind::Research,
        EvidenceKind::Attestation,
    ] {
        assert!(!eggplan_core::is_execution_evidence(kind));
        let outcome = finalize_observation(&descriptor(kind), &context(kind, None), &result());
        assert!(
            outcome.is_ok(),
            "non-execution kind {kind:?} must finalize without a binding, got {outcome:?}"
        );
    }
}

#[test]
fn a_bound_execution_kind_is_accepted_by_the_same_gate() {
    let binding = VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
    for kind in ALL_KINDS {
        if !eggplan_core::is_execution_evidence(kind) {
            continue;
        }
        let outcome = finalize_observation(
            &descriptor(kind),
            &context(kind, Some(binding.clone())),
            &result(),
        );
        assert!(
            outcome.is_ok(),
            "execution-derived kind {kind:?} must be accepted with a binding, got {outcome:?}"
        );
    }
}
