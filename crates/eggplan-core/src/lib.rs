#![forbid(unsafe_code)]

//! Bounded, deterministic planning domain types.
//!
//! This crate has no filesystem, process, network, database, scheduler, or
//! model-runtime responsibilities.

mod assessment;
mod closure;
mod evidence;
mod graph;
mod identity;
mod model;
mod schema;

pub use assessment::{
    AssessmentReason, AssessmentStatus, CriterionAssessment, ItemAssessment, PlanAssessment,
    RequirementAssessment, assess_plan,
};
pub use closure::*;
pub use evidence::{
    EVIDENCE_SCHEMA_VERSION, EvidenceError, EvidenceObservation, EvidenceObservationInput,
    EvidenceStatus, ProviderDescriptor, ProviderRegistry,
};
pub use graph::{GraphError, ItemReadiness, Readiness, readiness};
pub use identity::{ClosureId, EvidenceObservationId, EvidenceSupersessionId};
pub use identity::{
    CriterionId, EvidenceProviderId, PlanId, PlanItemId, TypedId, VerificationDigest,
};
pub use model::*;
pub use schema::{SCHEMA_VERSION, canonical_json, digest_json, parse_plan, verification_digest};

/// Canonical classification of which evidence kinds are execution-derived.
///
/// Execution-derived evidence is trust-bearing: it is produced by running
/// something, so it must be bound to a verification digest before it can be
/// promoted to proof. This function is the single owner of that classification
/// in the workspace. The provider SPI (`eggplan-integrations`) and the CodeGG
/// bridge (`eggplan-codegg-compat`) call it instead of re-deriving the set, so
/// the rule cannot drift between crates.
///
/// The match is deliberately exhaustive: it has no wildcard arm. Adding a
/// variant to [`EvidenceKind`] is a compile error until the new variant is
/// classified here, which forces the trust question to be answered in the same
/// place the variant is declared rather than silently defaulting to
/// "not execution-derived".
pub fn is_execution_evidence(kind: EvidenceKind) -> bool {
    match kind {
        EvidenceKind::Command
        | EvidenceKind::Test
        | EvidenceKind::StaticAnalysis
        | EvidenceKind::DelegatedRun
        | EvidenceKind::Benchmark => true,
        EvidenceKind::Revision
        | EvidenceKind::Artifact
        | EvidenceKind::Research
        | EvidenceKind::HumanJudgment
        | EvidenceKind::Attestation => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the classification of every current [`EvidenceKind`] variant by
    /// name. The table is the reviewable record of which kinds are
    /// execution-derived; reclassifying one is an explicit edit here.
    #[test]
    fn execution_evidence_classification_pins_every_kind() {
        let expected: [(EvidenceKind, bool); 10] = [
            (EvidenceKind::Command, true),
            (EvidenceKind::Test, true),
            (EvidenceKind::StaticAnalysis, true),
            (EvidenceKind::Revision, false),
            (EvidenceKind::Artifact, false),
            (EvidenceKind::DelegatedRun, true),
            (EvidenceKind::Benchmark, true),
            (EvidenceKind::Research, false),
            (EvidenceKind::HumanJudgment, false),
            (EvidenceKind::Attestation, false),
        ];
        for (kind, execution_derived) in expected {
            assert_eq!(
                is_execution_evidence(kind),
                execution_derived,
                "execution-evidence classification changed for {kind:?}"
            );
        }
    }
}

/// Explicit resource and text bounds for schema v1 and v2. Text counts Unicode scalar
/// values, not UTF-8 bytes. IDs and digests count ASCII bytes.
pub mod bounds {
    pub const ID_CHARS: usize = 96;
    pub const OBJECTIVE_CHARS: usize = 4_000;
    pub const DESCRIPTION_CHARS: usize = 4_000;
    pub const BLOCKER_CHARS: usize = 2_000;
    pub const NEXT_ACTION_CHARS: usize = 2_000;
    pub const PROVENANCE_CHARS: usize = 2_000;
    pub const CRITERION_CHARS: usize = 2_000;
    pub const REQUIREMENT_CHARS: usize = 1_000;
    pub const ARTIFACT_REF_CHARS: usize = 2_000;
    pub const SUPERSESSION_REASON_CHARS: usize = 2_000;
    pub const MAX_ITEMS: usize = 512;
    pub const MAX_DEPENDENCIES: usize = 64;
    pub const MAX_CRITERIA: usize = 128;
    pub const MAX_REQUIREMENTS: usize = 32;
    pub const MAX_ARTIFACT_REFS: usize = 64;
    pub const MAX_EXTENSION_ENTRIES: usize = 32;
    pub const MAX_EXTENSION_VALUE_CHARS: usize = 2_000;
    pub const INVOCATION_REF_CHARS: usize = 2_000;
    pub const MAX_OBSERVATION_METADATA: usize = 32;
    pub const OBSERVATION_METADATA_VALUE_CHARS: usize = 2_000;
    pub const MAX_OBSERVATIONS_PER_PLAN: usize = 10_000;
}
