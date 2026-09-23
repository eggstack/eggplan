#![forbid(unsafe_code)]

//! Bounded, deterministic planning domain types.
//!
//! This crate has no filesystem, process, network, database, scheduler, or
//! model-runtime responsibilities.

mod assessment;
mod evidence;
mod graph;
mod identity;
mod model;
mod schema;

pub use assessment::{
    AssessmentReason, AssessmentStatus, CriterionAssessment, ItemAssessment, PlanAssessment,
    RequirementAssessment, assess_plan,
};
pub use evidence::{
    EVIDENCE_SCHEMA_VERSION, EvidenceError, EvidenceObservation, EvidenceObservationInput,
    EvidenceStatus, ProviderDescriptor, ProviderRegistry,
};
pub use graph::{GraphError, ItemReadiness, Readiness, readiness};
pub use identity::EvidenceObservationId;
pub use identity::{
    CriterionId, EvidenceProviderId, PlanId, PlanItemId, TypedId, VerificationDigest,
};
pub use model::*;
pub use schema::{SCHEMA_VERSION, canonical_json, digest_json, parse_plan};

pub(crate) fn is_execution_evidence(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::Command
            | EvidenceKind::Test
            | EvidenceKind::StaticAnalysis
            | EvidenceKind::DelegatedRun
            | EvidenceKind::Benchmark
    )
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
