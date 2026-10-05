//! Bounded DTO adapter for host-verified Eggbench bundles and comparisons.
//!
//! The host runs `eggbench inspect --json` / `eggbench compare --json` (or
//! reads the finalized `.eggb` manifest and the standalone comparison receipt)
//! and performs native integrity verification before submitting an Eggplan
//! DTO. This module never executes Eggbench, walks a bundle directory,
//! computes a manifest digest, or opens a transport.
//!
//! Two claims are deliberately kept apart:
//!
//! 1. [`EvidenceKind::Artifact`] — a finalized immutable bundle exists and its
//!    digest tree verified natively. This is an integrity claim only.
//! 2. [`EvidenceKind::Benchmark`] — a comparison policy produced a verdict.
//!    This requires execution-derived verification binding from the host.
//!
//! A verified bundle is never a passing benchmark, and an integrity claim can
//! never be promoted into a verdict claim.
//!
//! Reviewed upstream revision: [`REVIEWED_EGGBENCH_SHA`].

use crate::{
    AdapterDescriptor, Capabilities, NormalizedProviderResult, ObservationContext, ProviderClass,
    SpiError, finalize_observation,
};
use eggplan_core::{
    ArtifactRef, EvidenceKind, EvidenceObservation, EvidenceProviderId, EvidenceStatus,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// Exact `eggstack/eggbench` revision these DTOs were qualified against.
///
/// Dual-green at freeze time: ordinary hosted CI `37143714313` (success) and
/// live external-tool qualification `37143714261` (success).
pub const REVIEWED_EGGBENCH_SHA: &str = "30a38251bccb5157beb68202ffe630f6253771e0";

/// Reviewed Eggbench host-green workflow identifiers for [`REVIEWED_EGGBENCH_SHA`].
pub const REVIEWED_EGGBENCH_CI_RUN: u64 = 37143714313;
pub const REVIEWED_EGGBENCH_LIVE_RUN: u64 = 37143714261;

/// Eggplan-owned DTO schema versions. Distinct from upstream versions.
pub const BUNDLE_DTO_SCHEMA: u32 = 1;
pub const COMPARISON_DTO_SCHEMA: u32 = 1;

/// Upstream `.eggb` manifest schema versions this adapter understands.
pub const SUPPORTED_MANIFEST_SCHEMA: u32 = 2;
/// Legacy manifest schema; only accepted together with an explicit legacy status.
pub const LEGACY_MANIFEST_SCHEMA: u32 = 1;

/// Upstream comparison receipt schema versions this adapter understands.
pub const MIN_SUPPORTED_RECEIPT_SCHEMA: u32 = 1;
/// Receipt schemas at or above this version carry independent performance and
/// correctness sections. Legacy receipts must never be projected into them.
pub const MIN_SEPARATED_RECEIPT_SCHEMA: u32 = 3;
pub const MAX_SUPPORTED_RECEIPT_SCHEMA: u32 = 4;

pub const MAX_DTO_BYTES: usize = 262_144;
pub const MAX_BUNDLE_ARTIFACT_HANDLES: usize = 16;
pub const MAX_BUNDLE_DRIVERS: usize = 32;
pub const MAX_WARNING_CATEGORIES: usize = 16;
pub const MAX_RUN_ID_CHARS: usize = 128;
pub const MAX_LABEL_CHARS: usize = 96;
pub const MAX_MEDIA_TYPE_CHARS: usize = 255;
pub const MAX_ARTIFACT_PATH_CHARS: usize = 512;
/// Eggplan's own artifact-reference ceiling leaves room for the bundle handle
/// plus every selected artifact handle.
pub const MAX_BUNDLE_ARTIFACT_REFS: usize = MAX_BUNDLE_ARTIFACT_HANDLES + 1;

/// Hex characters retained in the durable comparison artifact handle.
pub const COMPARISON_HANDLE_DIGEST_PREFIX: usize = 16;

const CAPS: Capabilities = Capabilities {
    // Every fact this adapter accepts is terminal: a finalized bundle, a
    // terminal execution status, or a completed comparison.
    supports_in_progress: false,
    artifacts: true,
    verification_binding: true,
    research_trust_metadata: false,
};

// ---------------------------------------------------------------------------
// Upstream enums, reproduced as strict Eggplan-owned vocabulary.
// ---------------------------------------------------------------------------

/// Upstream `ExecutionStatus` — lifecycle outcome, carries no comparison meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Completed,
    Failed,
    Cancelled,
    Invalid,
}

/// Upstream `ComparisonVerdict` embedded in a bundle manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonVerdict {
    Pass,
    Fail,
    Inconclusive,
    Invalid,
}

/// Upstream `LegacyRunStatus` — manifest v1 overloaded status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyRunStatus {
    Succeeded,
    Failed,
    Cancelled,
    Invalid,
    Inconclusive,
}

/// Upstream `AggregateVerdict` — never carries descriptive effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateVerdict {
    Pass,
    Fail,
    Inconclusive,
    Invalid,
}

/// Upstream subject discriminants. No argv, environment, or secret reference
/// is retained: only identity hints needed to interpret the evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKindHint {
    ManagedCommand,
    External,
    Label,
}

/// Upstream `ArtifactRole` discriminants retained in bounded handles.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactRoleHint {
    ExperimentPlan,
    ResolvedPlan,
    EnvironmentFingerprint,
    Topology,
    Subject,
    TrialResult,
    Telemetry,
    Stdout,
    Stderr,
    TrialArtifact,
    Comparison,
    Report,
    Other { label: String },
}

impl ArtifactRoleHint {
    fn label(&self) -> &str {
        match self {
            Self::ExperimentPlan => "experiment_plan",
            Self::ResolvedPlan => "resolved_plan",
            Self::EnvironmentFingerprint => "environment_fingerprint",
            Self::Topology => "topology",
            Self::Subject => "subject",
            Self::TrialResult => "trial_result",
            Self::Telemetry => "telemetry",
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::TrialArtifact => "trial_artifact",
            Self::Comparison => "comparison",
            Self::Report => "report",
            Self::Other { .. } => "other",
        }
    }
}

// ---------------------------------------------------------------------------
// Bundle DTO
// ---------------------------------------------------------------------------

/// Eggplan-owned bounded projection of one natively verified `.eggb` bundle.
///
/// The verifying host supplies this after `eggbench inspect --json` plus its
/// own native integrity verification. `manifest_sha256` is the digest of the
/// exact finalized `manifest.json` bytes; upstream `inspect` does not expose
/// it, so the host computes it after native verification.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EggbenchBundleEvidenceV1 {
    pub schema_version: u32,
    /// Upstream revision the host executed, as reviewed provenance.
    pub reviewed_revision: String,
    pub manifest_schema_version: u32,
    pub run_id: String,
    /// 64 lowercase hex; digest of the exact finalized manifest bytes.
    pub manifest_sha256: String,
    /// A bundle is only finalized evidence when its manifest says so.
    pub finalized: bool,
    pub execution_status: Option<ExecutionStatus>,
    pub comparison_verdict: Option<ComparisonVerdict>,
    /// Present only for manifest v1, whose status is inherently ambiguous.
    pub legacy_status: Option<LegacyRunStatus>,
    pub subject: SubjectHint,
    pub artifact_count: u32,
    pub total_retained_bytes: u64,
    pub artifacts: Vec<BundleArtifactHandle>,
    pub drivers: Vec<DriverProvenance>,
}

/// Bounded subject provenance hints. Never an Eggplan `SubjectRevision`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectHint {
    pub kind: SubjectKindHint,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub digest: Option<String>,
}

/// One selected bundle artifact. The bundle-relative path is a confined native
/// path, never a temporary host path; durable identity is the run-scoped handle.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleArtifactHandle {
    pub path: String,
    pub role: ArtifactRoleHint,
    #[serde(default)]
    pub media_type: Option<String>,
    pub byte_size: u64,
    /// 64 lowercase hex.
    pub sha256: String,
}

/// Bounded driver/producer provenance needed to interpret the measurement.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriverProvenance {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// Comparison DTO
// ---------------------------------------------------------------------------

/// Eggplan-owned bounded projection of one standalone comparison receipt.
///
/// Receipt schema v3/v4 separate `performance_verdict` (metric gates),
/// `correctness` (independent security evidence), and `aggregate_verdict` (the
/// combined verdict). Legacy v1/v2 receipts never carried a correctness
/// section, so none is ever projected into them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EggbenchComparisonEvidenceV1 {
    pub schema_version: u32,
    pub reviewed_revision: String,
    pub receipt_schema_version: u32,
    pub policy_id: String,
    pub created_by_version: String,
    pub candidate: BundleRef,
    #[serde(default)]
    pub baseline: Option<BundleRef>,
    /// Candidate execution status, as reported by `inspect`.
    pub execution_status: Option<ExecutionStatus>,
    pub aggregate_verdict: Option<AggregateVerdict>,
    #[serde(default)]
    pub performance_verdict: Option<AggregateVerdict>,
    #[serde(default)]
    pub correctness: Option<CorrectnessSummary>,
    pub comparability: ComparabilitySummary,
    #[serde(default)]
    pub warning_categories: Vec<String>,
    /// 64 lowercase hex of the standalone receipt bytes, when one exists.
    #[serde(default)]
    pub receipt_sha256: Option<String>,
}

/// Bounded immutable bundle identity as recorded in a receipt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleRef {
    pub manifest_schema_version: u32,
    pub run_id: String,
    /// 64 lowercase hex.
    pub manifest_sha256: String,
    #[serde(default)]
    pub subject_revision: Option<String>,
    #[serde(default)]
    pub subject_digest: Option<String>,
}

/// Bounded correctness aggregate. Per-check records, reasons, and evidence
/// paths stay in the native receipt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectnessSummary {
    pub policy_id: String,
    pub check_count: u32,
    pub aggregate_verdict: AggregateVerdict,
}

/// Bounded comparability axes. `critical_mismatch` is upstream's
/// `critical_mismatch`; the `compare` envelope's `comparability_match` is its
/// inverse and must be inverted by the host before populating this field.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparabilitySummary {
    pub workload_match: bool,
    pub driver_match: bool,
    pub topology_match: bool,
    pub critical_mismatch: bool,
}

// ---------------------------------------------------------------------------
// Provider descriptor
// ---------------------------------------------------------------------------

/// Adapter-fixed provider identity. Callers cannot select a different provider
/// and cannot widen the allowed kinds.
pub fn descriptor() -> AdapterDescriptor {
    AdapterDescriptor::new(
        EvidenceProviderId::new("epp_eggbench").expect("static id"),
        ProviderClass::Benchmark,
        [EvidenceKind::Artifact, EvidenceKind::Benchmark],
        "1.0",
        CAPS,
    )
    .expect("static descriptor")
}

pub fn parse_bundle(bytes: &[u8]) -> Result<EggbenchBundleEvidenceV1, SpiError> {
    if bytes.len() > MAX_DTO_BYTES {
        return Err(SpiError::Invalid("Eggbench bundle DTO exceeds byte limit"));
    }
    let value: EggbenchBundleEvidenceV1 = serde_json::from_slice(bytes)
        .map_err(|_| SpiError::Invalid("invalid Eggbench bundle DTO JSON or contract"))?;
    validate_bundle(&value)?;
    Ok(value)
}

pub fn parse_comparison(bytes: &[u8]) -> Result<EggbenchComparisonEvidenceV1, SpiError> {
    if bytes.len() > MAX_DTO_BYTES {
        return Err(SpiError::Invalid(
            "Eggbench comparison DTO exceeds byte limit",
        ));
    }
    let value: EggbenchComparisonEvidenceV1 = serde_json::from_slice(bytes)
        .map_err(|_| SpiError::Invalid("invalid Eggbench comparison DTO JSON or contract"))?;
    validate_comparison(&value)?;
    Ok(value)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_bundle(value: &EggbenchBundleEvidenceV1) -> Result<(), SpiError> {
    if value.schema_version != BUNDLE_DTO_SCHEMA {
        return Err(SpiError::Invalid("unknown Eggbench bundle DTO schema"));
    }
    check_reviewed_revision(&value.reviewed_revision)?;
    match value.manifest_schema_version {
        SUPPORTED_MANIFEST_SCHEMA => {
            if value.legacy_status.is_some() {
                return Err(SpiError::Invalid(
                    "legacy status is only valid for a manifest v1 bundle",
                ));
            }
        }
        LEGACY_MANIFEST_SCHEMA => {
            if value.legacy_status.is_none() {
                return Err(SpiError::Invalid(
                    "manifest v1 requires an explicit legacy status",
                ));
            }
        }
        _ => {
            return Err(SpiError::Invalid(
                "unsupported Eggbench manifest schema version",
            ));
        }
    }
    if !value.finalized {
        return Err(SpiError::Invalid(
            "only a finalized Eggbench bundle is admissible evidence",
        ));
    }
    check_run_id(&value.run_id)?;
    check_sha256(&value.manifest_sha256)?;
    check_subject(&value.subject)?;
    if value.artifacts.len() > MAX_BUNDLE_ARTIFACT_HANDLES
        || value.drivers.len() > MAX_BUNDLE_DRIVERS
    {
        return Err(SpiError::Invalid("Eggbench bundle element limit exceeded"));
    }
    if (value.artifact_count as usize) < value.artifacts.len() {
        return Err(SpiError::Invalid(
            "selected artifacts exceed the declared artifact count",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut selected_bytes: u64 = 0;
    for artifact in &value.artifacts {
        check_artifact_path(&artifact.path)?;
        if !seen.insert(artifact.path.as_str()) {
            return Err(SpiError::Invalid("duplicate Eggbench artifact handle"));
        }
        check_sha256(&artifact.sha256)?;
        if let Some(media_type) = &artifact.media_type {
            check_text(media_type, MAX_MEDIA_TYPE_CHARS)?;
        }
        if let ArtifactRoleHint::Other { label } = &artifact.role {
            check_label(label)?;
        }
        selected_bytes = selected_bytes.saturating_add(artifact.byte_size);
    }
    if selected_bytes > value.total_retained_bytes {
        return Err(SpiError::Invalid(
            "selected artifact bytes exceed the declared retained bytes",
        ));
    }
    let mut drivers = BTreeSet::new();
    for driver in &value.drivers {
        check_label(&driver.name)?;
        if let Some(version) = &driver.version {
            check_label(version)?;
        }
        if !drivers.insert(driver.name.as_str()) {
            return Err(SpiError::Invalid("duplicate Eggbench driver provenance"));
        }
    }
    Ok(())
}

fn validate_comparison(value: &EggbenchComparisonEvidenceV1) -> Result<(), SpiError> {
    if value.schema_version != COMPARISON_DTO_SCHEMA {
        return Err(SpiError::Invalid("unknown Eggbench comparison DTO schema"));
    }
    check_reviewed_revision(&value.reviewed_revision)?;
    if !(MIN_SUPPORTED_RECEIPT_SCHEMA..=MAX_SUPPORTED_RECEIPT_SCHEMA)
        .contains(&value.receipt_schema_version)
    {
        return Err(SpiError::Invalid(
            "unsupported Eggbench comparison receipt schema version",
        ));
    }
    check_label(&value.policy_id)?;
    check_label(&value.created_by_version)?;
    check_bundle_ref(&value.candidate)?;
    if let Some(baseline) = &value.baseline {
        check_bundle_ref(baseline)?;
        if baseline.run_id == value.candidate.run_id
            || baseline.manifest_sha256 == value.candidate.manifest_sha256
        {
            return Err(SpiError::Invalid(
                "candidate and baseline comparison identities must differ",
            ));
        }
    }
    let separated = value.receipt_schema_version >= MIN_SEPARATED_RECEIPT_SCHEMA;
    if !separated && (value.performance_verdict.is_some() || value.correctness.is_some()) {
        return Err(SpiError::Invalid(
            "legacy comparison receipt cannot carry performance or correctness sections",
        ));
    }
    if let Some(correctness) = &value.correctness {
        if !separated {
            return Err(SpiError::Invalid(
                "correctness cannot be projected into a legacy comparison receipt",
            ));
        }
        check_label(&correctness.policy_id)?;
    }
    if value.warning_categories.len() > MAX_WARNING_CATEGORIES {
        return Err(SpiError::Invalid(
            "comparison warning category limit exceeded",
        ));
    }
    let mut categories = BTreeSet::new();
    for category in &value.warning_categories {
        check_stable_label(category)?;
        if !categories.insert(category.as_str()) {
            return Err(SpiError::Invalid("duplicate comparison warning category"));
        }
    }
    if let Some(digest) = &value.receipt_sha256 {
        check_sha256(digest)?;
    }
    Ok(())
}

fn check_bundle_ref(value: &BundleRef) -> Result<(), SpiError> {
    if !matches!(
        value.manifest_schema_version,
        LEGACY_MANIFEST_SCHEMA | SUPPORTED_MANIFEST_SCHEMA
    ) {
        return Err(SpiError::Invalid(
            "unsupported bundle manifest schema version in comparison identity",
        ));
    }
    check_run_id(&value.run_id)?;
    check_sha256(&value.manifest_sha256)?;
    if let Some(revision) = &value.subject_revision {
        check_text(revision, MAX_LABEL_CHARS)?;
    }
    if let Some(digest) = &value.subject_digest {
        check_text(digest, MAX_LABEL_CHARS)?;
    }
    Ok(())
}

fn check_subject(value: &SubjectHint) -> Result<(), SpiError> {
    if let Some(target) = &value.target {
        check_label(target)?;
    }
    if let Some(revision) = &value.revision {
        check_text(revision, MAX_LABEL_CHARS)?;
    }
    if let Some(digest) = &value.digest {
        check_text(digest, MAX_LABEL_CHARS)?;
    }
    Ok(())
}

fn check_reviewed_revision(value: &str) -> Result<(), SpiError> {
    if value.len() != 40 || !is_lowercase_hex(value) {
        return Err(SpiError::Invalid("malformed reviewed Eggbench revision"));
    }
    Ok(())
}

fn check_run_id(value: &str) -> Result<(), SpiError> {
    if value.is_empty()
        || value.chars().count() > MAX_RUN_ID_CHARS
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
    {
        return Err(SpiError::Invalid("malformed Eggbench run id"));
    }
    Ok(())
}

fn check_sha256(value: &str) -> Result<(), SpiError> {
    if value.len() != 64 || !is_lowercase_hex(value) {
        return Err(SpiError::Invalid("malformed Eggbench SHA-256 digest"));
    }
    Ok(())
}

fn is_lowercase_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn check_label(value: &str) -> Result<(), SpiError> {
    check_text(value, MAX_LABEL_CHARS)
}

/// Bounded free-ish text that still rejects URLs, secrets, control characters,
/// and NUL. Callers supply the specific diagnostic.
fn check_text(value: &str, max: usize) -> Result<(), SpiError> {
    if value.is_empty()
        || value.contains('\0')
        || value.chars().count() > max
        || value.contains("://")
        || value.bytes().any(|b| b.is_ascii_control() && b != b'\t')
    {
        return Err(SpiError::Invalid(
            "malformed, unsafe, or oversized Eggbench text field",
        ));
    }
    Ok(())
}

/// Strict `snake_case` stable upstream label.
fn check_stable_label(value: &str) -> Result<(), SpiError> {
    if value.is_empty()
        || value.len() > 64
        || value.starts_with('_')
        || value.ends_with('_')
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(SpiError::Invalid(
            "malformed Eggbench stable snake_case label",
        ));
    }
    Ok(())
}

fn check_artifact_path(value: &str) -> Result<(), SpiError> {
    if value.starts_with('/')
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(SpiError::Invalid(
            "malformed or escaping Eggbench artifact path",
        ));
    }
    check_text(value, MAX_ARTIFACT_PATH_CHARS)
        .map_err(|_| SpiError::Invalid("malformed or escaping Eggbench artifact path"))
}

// ---------------------------------------------------------------------------
// Subject agreement
// ---------------------------------------------------------------------------

/// Require an Eggbench-declared subject revision to agree with the host's
/// Eggplan subject when both are Git identities.
///
/// An Eggbench revision hint is provenance only. It is never translated into an
/// Eggplan repository identity, and a non-Git Eggplan subject is never compared
/// against a Git hint.
fn check_subject_agreement(
    hint: Option<&str>,
    context: &ObservationContext,
) -> Result<(), SpiError> {
    let (Some(hint), true) = (hint, context.subject.subject_kind == "git") else {
        return Ok(());
    };
    if hint != context.subject.revision {
        return Err(SpiError::Invalid(
            "Eggbench subject revision disagrees with the Eggplan Git subject",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Bundle normalization
// ---------------------------------------------------------------------------

/// Normalize a natively verified bundle.
///
/// `EvidenceKind::Artifact` records bundle existence and structural integrity
/// only. `EvidenceKind::Benchmark` additionally requires the host's
/// verification digest and a real upstream comparison verdict.
pub fn normalize_bundle(
    bundle: &EggbenchBundleEvidenceV1,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_bundle(bundle)?;
    match context.requested_kind {
        EvidenceKind::Artifact => {}
        EvidenceKind::Benchmark => {}
        _ => return Err(SpiError::UnsupportedKind),
    }
    check_subject_agreement(bundle.subject.revision.as_deref(), context)?;

    let refs = bundle_artifact_refs(bundle)?;
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "eggbench_reviewed_revision".into(),
        bundle.reviewed_revision.clone(),
    );
    metadata.insert(
        "manifest_schema_version".into(),
        bundle.manifest_schema_version.to_string(),
    );
    metadata.insert("run_id".into(), bundle.run_id.clone());
    metadata.insert("manifest_sha256".into(), bundle.manifest_sha256.clone());
    metadata.insert("artifact_count".into(), bundle.artifact_count.to_string());
    metadata.insert(
        "total_retained_bytes".into(),
        bundle.total_retained_bytes.to_string(),
    );
    metadata.insert(
        "selected_artifact_count".into(),
        bundle.artifacts.len().to_string(),
    );
    metadata.insert("driver_count".into(), bundle.drivers.len().to_string());
    metadata.insert(
        "subject_kind".into(),
        subject_kind_label(bundle.subject.kind).to_string(),
    );
    if let Some(target) = &bundle.subject.target {
        metadata.insert("subject_target".into(), target.clone());
    }
    if let Some(revision) = &bundle.subject.revision {
        metadata.insert("upstream_subject_revision_hint".into(), revision.clone());
    }
    if let Some(digest) = &bundle.subject.digest {
        metadata.insert("upstream_subject_digest_hint".into(), digest.clone());
    }
    if let Some(status) = bundle.execution_status {
        metadata.insert(
            "execution_status".into(),
            execution_label(status).to_string(),
        );
    }
    if let Some(verdict) = bundle.comparison_verdict {
        metadata.insert(
            "embedded_comparison_verdict".into(),
            comparison_label(verdict).to_string(),
        );
    }
    if let Some(legacy) = bundle.legacy_status {
        metadata.insert("legacy_status".into(), legacy_label(legacy).to_string());
        metadata.insert("legacy_status_ambiguous".into(), "true".into());
    }
    if let Some(driver) = bundle.drivers.first()
        && let Some(producer) = driver_provenance(driver)
    {
        metadata.insert("producer".into(), producer);
    }

    let status = match context.requested_kind {
        // Structural integrity of a finalized, natively verified bundle. This
        // says nothing about whether the measured run or a comparison passed.
        EvidenceKind::Artifact => EvidenceStatus::Passed,
        EvidenceKind::Benchmark => bundle_benchmark_status(bundle),
        _ => unreachable!("kind narrowed above"),
    };

    finalize_observation(
        &descriptor(),
        context,
        &NormalizedProviderResult {
            status,
            source_trust: None,
            result_metadata: metadata,
            artifacts: refs,
        },
    )
}

fn bundle_benchmark_status(bundle: &EggbenchBundleEvidenceV1) -> EvidenceStatus {
    match bundle.execution_status {
        None => return EvidenceStatus::Inconclusive,
        Some(ExecutionStatus::Failed) => return EvidenceStatus::Failed,
        Some(ExecutionStatus::Cancelled) => return EvidenceStatus::Skipped,
        Some(ExecutionStatus::Invalid) => return EvidenceStatus::Inconclusive,
        Some(ExecutionStatus::Completed) => {}
    }
    if bundle.manifest_schema_version == LEGACY_MANIFEST_SCHEMA {
        // The legacy `inconclusive` state is ambiguous between "no comparison"
        // and "inconclusive comparison"; it is never disambiguated by inference.
        return EvidenceStatus::Inconclusive;
    }
    match bundle.comparison_verdict {
        Some(ComparisonVerdict::Pass) => EvidenceStatus::Passed,
        Some(ComparisonVerdict::Fail) => EvidenceStatus::Failed,
        Some(ComparisonVerdict::Inconclusive) | Some(ComparisonVerdict::Invalid) => {
            EvidenceStatus::Inconclusive
        }
        None => EvidenceStatus::Inconclusive,
    }
}

fn bundle_artifact_refs(bundle: &EggbenchBundleEvidenceV1) -> Result<Vec<ArtifactRef>, SpiError> {
    let mut refs = Vec::with_capacity(bundle.artifacts.len() + 1);
    refs.push(ArtifactRef {
        reference: format!("eggbench:bundle:{}", bundle.run_id),
        digest: Some(format!("sha256:{}", bundle.manifest_sha256)),
        media_type: Some(BUNDLE_MANIFEST_MEDIA_TYPE.to_string()),
    });
    for artifact in &bundle.artifacts {
        refs.push(ArtifactRef {
            reference: format!("eggbench:bundle:{}#{}", bundle.run_id, artifact.path),
            digest: Some(format!("sha256:{}", artifact.sha256)),
            media_type: artifact.media_type.clone(),
        });
    }
    if refs.len() > MAX_BUNDLE_ARTIFACT_REFS {
        return Err(SpiError::Invalid(
            "Eggbench artifact reference limit exceeded",
        ));
    }
    Ok(refs)
}

const BUNDLE_MANIFEST_MEDIA_TYPE: &str = "application/vnd.eggbench.bundle-manifest+json";

// ---------------------------------------------------------------------------
// Comparison normalization
// --------------------------------------------------------------------------

/// Normalize one standalone comparison receipt.
///
/// Only `EvidenceKind::Benchmark` is admissible here: a receipt is a verdict
/// record, not an artifact-integrity claim.
pub fn normalize_comparison(
    comparison: &EggbenchComparisonEvidenceV1,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_comparison(comparison)?;
    if context.requested_kind != EvidenceKind::Benchmark {
        return Err(SpiError::UnsupportedKind);
    }
    check_subject_agreement(comparison.candidate.subject_revision.as_deref(), context)?;

    let refs = comparison_artifact_refs(comparison)?;
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "eggbench_reviewed_revision".into(),
        comparison.reviewed_revision.clone(),
    );
    metadata.insert(
        "receipt_schema_version".into(),
        comparison.receipt_schema_version.to_string(),
    );
    metadata.insert("policy_id".into(), comparison.policy_id.clone());
    metadata.insert(
        "eggbench_version".into(),
        comparison.created_by_version.clone(),
    );
    metadata.insert(
        "candidate_run_id".into(),
        comparison.candidate.run_id.clone(),
    );
    metadata.insert(
        "candidate_manifest_sha256".into(),
        comparison.candidate.manifest_sha256.clone(),
    );
    if let Some(baseline) = &comparison.baseline {
        metadata.insert("baseline_run_id".into(), baseline.run_id.clone());
        metadata.insert(
            "baseline_manifest_sha256".into(),
            baseline.manifest_sha256.clone(),
        );
    }
    if let Some(status) = comparison.execution_status {
        metadata.insert(
            "execution_status".into(),
            execution_label(status).to_string(),
        );
    }
    if let Some(verdict) = comparison.aggregate_verdict {
        metadata.insert(
            "aggregate_verdict".into(),
            aggregate_label(verdict).to_string(),
        );
    }
    if let Some(verdict) = comparison.performance_verdict {
        metadata.insert(
            "performance_verdict".into(),
            aggregate_label(verdict).to_string(),
        );
    }
    if let Some(correctness) = &comparison.correctness {
        metadata.insert(
            "correctness_verdict".into(),
            aggregate_label(correctness.aggregate_verdict).to_string(),
        );
        metadata.insert(
            "correctness_check_count".into(),
            correctness.check_count.to_string(),
        );
    }
    metadata.insert(
        "comparability_critical_mismatch".into(),
        comparison.comparability.critical_mismatch.to_string(),
    );
    if !comparison.warning_categories.is_empty() {
        metadata.insert(
            "warning_categories".into(),
            comparison.warning_categories.join(","),
        );
    }
    if let Some(digest) = &comparison.receipt_sha256 {
        metadata.insert("receipt_sha256".into(), digest.clone());
    }

    let status = comparison_benchmark_status(comparison);
    finalize_observation(
        &descriptor(),
        context,
        &NormalizedProviderResult {
            status,
            source_trust: None,
            result_metadata: metadata,
            artifacts: refs,
        },
    )
}

fn comparison_benchmark_status(comparison: &EggbenchComparisonEvidenceV1) -> EvidenceStatus {
    match comparison.execution_status {
        None => return EvidenceStatus::Inconclusive,
        Some(ExecutionStatus::Failed) => return EvidenceStatus::Failed,
        Some(ExecutionStatus::Cancelled) => return EvidenceStatus::Skipped,
        Some(ExecutionStatus::Invalid) => return EvidenceStatus::Inconclusive,
        Some(ExecutionStatus::Completed) => {}
    }
    // A critical comparability mismatch downgrades the comparison to
    // descriptive-only upstream. Descriptive evidence never gates, so it can
    // never become a passing benchmark claim.
    if comparison.comparability.critical_mismatch {
        return EvidenceStatus::Inconclusive;
    }
    match comparison.aggregate_verdict {
        Some(AggregateVerdict::Pass) => EvidenceStatus::Passed,
        Some(AggregateVerdict::Fail) => EvidenceStatus::Failed,
        Some(AggregateVerdict::Inconclusive) | Some(AggregateVerdict::Invalid) => {
            EvidenceStatus::Inconclusive
        }
        None => EvidenceStatus::Inconclusive,
    }
}

fn comparison_artifact_refs(
    comparison: &EggbenchComparisonEvidenceV1,
) -> Result<Vec<ArtifactRef>, SpiError> {
    let Some(receipt_digest) = comparison.receipt_sha256.as_deref() else {
        return Ok(Vec::new());
    };
    let prefix = &receipt_digest[..COMPARISON_HANDLE_DIGEST_PREFIX];
    Ok(vec![ArtifactRef {
        reference: format!(
            "eggbench:comparison:{}:{}",
            comparison.candidate.run_id, prefix
        ),
        digest: Some(format!("sha256:{receipt_digest}")),
        media_type: Some("application/json".to_string()),
    }])
}

/// Reject a comparison whose candidate identity does not describe the same
/// bundle the host separately verified.
pub fn assert_same_bundle(
    comparison: &EggbenchComparisonEvidenceV1,
    bundle: &EggbenchBundleEvidenceV1,
) -> Result<(), SpiError> {
    validate_comparison(comparison)?;
    validate_bundle(bundle)?;
    if comparison.candidate.run_id != bundle.run_id
        || comparison.candidate.manifest_sha256 != bundle.manifest_sha256
    {
        return Err(SpiError::Invalid(
            "comparison candidate identity does not match the verified Eggbench bundle",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

fn execution_label(value: ExecutionStatus) -> &'static str {
    match value {
        ExecutionStatus::Completed => "completed",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
        ExecutionStatus::Invalid => "invalid",
    }
}

fn comparison_label(value: ComparisonVerdict) -> &'static str {
    match value {
        ComparisonVerdict::Pass => "pass",
        ComparisonVerdict::Fail => "fail",
        ComparisonVerdict::Inconclusive => "inconclusive",
        ComparisonVerdict::Invalid => "invalid",
    }
}

fn legacy_label(value: LegacyRunStatus) -> &'static str {
    match value {
        LegacyRunStatus::Succeeded => "succeeded",
        LegacyRunStatus::Failed => "failed",
        LegacyRunStatus::Cancelled => "cancelled",
        LegacyRunStatus::Invalid => "invalid",
        LegacyRunStatus::Inconclusive => "inconclusive",
    }
}

fn aggregate_label(value: AggregateVerdict) -> &'static str {
    match value {
        AggregateVerdict::Pass => "pass",
        AggregateVerdict::Fail => "fail",
        AggregateVerdict::Inconclusive => "inconclusive",
        AggregateVerdict::Invalid => "invalid",
    }
}

fn subject_kind_label(value: SubjectKindHint) -> &'static str {
    match value {
        SubjectKindHint::ManagedCommand => "managed_command",
        SubjectKindHint::External => "external",
        SubjectKindHint::Label => "label",
    }
}

fn driver_provenance(driver: &DriverProvenance) -> Option<String> {
    let version = driver.version.as_deref().unwrap_or("unknown");
    let value = format!("{}@{}", driver.name, version);
    (value.chars().count() <= MAX_LABEL_CHARS).then_some(value)
}

/// Role label helper retained for callers that want to assert on retained roles.
pub fn artifact_role_label(role: &ArtifactRoleHint) -> &str {
    role.label()
}
