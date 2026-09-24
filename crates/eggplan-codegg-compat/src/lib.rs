//! One-way, fixture-oriented compatibility mapping for CodeGG WorkPlan
//! snapshots. This crate never acquires native evidence or owns CodeGG state.

use eggplan_core::{
    AcceptanceCriterion, AssessmentReason, EvidenceCardinality, EvidenceKind, EvidenceObservation,
    EvidenceRequirement, Plan, PlanAssessment, PlanId, PlanItem, PlanItemId, PlanItemStatus,
    PlanStatus, ProviderRegistry, SubjectPolicy, SubjectRevision, VerificationDigest, assess_plan,
    digest_json,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_SCHEMA_VERSION: u32 = 1;
pub const SOURCE_CODEGG_SHA: &str = "a3c87fc18ee55aaf630401a562c11bb83112fd82";
pub const MAX_ITEMS: usize = 64;
pub const MAX_ACCEPTANCES_PER_ITEM: usize = 16;
pub const MAX_EVIDENCE_REFS_PER_ITEM: usize = 32;
pub const MAX_TEXT_CHARS: usize = 4_000;
pub const MAX_SOURCE_ID_CHARS: usize = 128;
pub const MAX_DETAIL_CHARS: usize = 2_000;
pub const MAX_FIXTURE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub schema_version: u32,
    pub source_repository: String,
    pub source_sha: String,
    pub source_case: String,
    pub snapshot: CodeggPlanSnapshot,
}

impl Fixture {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_FIXTURE_BYTES {
            return Err("fixture exceeds the 4 MiB input bound".into());
        }
        serde_json::from_slice(bytes).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeggPlanSnapshot {
    pub plan_id: String,
    pub revision: i64,
    pub status: CodeggPlanStatus,
    pub objective: String,
    #[serde(default)]
    pub current_phase: Option<String>,
    #[serde(default)]
    pub current_item_id: Option<String>,
    pub items: Vec<CodeggItemSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeggPlanStatus {
    Active,
    Blocked,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeggItemSnapshot {
    pub item_id: String,
    pub position: i64,
    #[serde(default)]
    pub parent_item_id: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub status: CodeggItemStatus,
    pub description: String,
    #[serde(default)]
    pub acceptance: Vec<CodeggAcceptance>,
    #[serde(default)]
    pub evidence: Vec<CodeggEvidenceRef>,
    #[serde(default)]
    pub owner_run_id: Option<String>,
    #[serde(default)]
    pub owner_job_id: Option<String>,
    #[serde(default)]
    pub blocker: Option<String>,
    #[serde(default)]
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeggItemStatus {
    Pending,
    Actionable,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeggAcceptance {
    pub description: String,
    pub disposition: AcceptanceDisposition,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceDisposition {
    Unmet,
    Satisfied,
    RequiresUserJudgment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeggEvidenceRef {
    pub kind: CodeggEvidenceKind,
    pub ref_id: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeggEvidenceKind {
    TestJob,
    DelegatedRun,
    SchedulerJob,
    AgentRun,
    Artifact,
    Commit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEvidence {
    /// Host-normalized observation. Its provider identity comes from the host.
    pub observation: EvidenceObservation,
    /// Binding derived by the host from the authoritative native verification
    /// specification, when this kind requires one.
    pub expected_verification: Option<VerificationDigest>,
}

pub trait EvidenceResolver {
    fn resolve(
        &mut self,
        reference: &CodeggEvidenceRef,
        subject: &SubjectRevision,
    ) -> Result<ResolvedEvidence, String>;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "code", content = "detail")]
pub enum Loss {
    CompletedPlanNeedsGuardedClose,
    AcceptanceEvidenceAssignmentIsItemScoped,
    OwnerReferencesAreProvenanceOnly,
    SourceRevisionIsProvenanceOnly,
    SerializedDispositionIsNotEvidence,
    AcceptanceNoteOmitted,
    EvidenceDetailOmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeggCompletionFamily {
    Complete,
    ActionableWorkRemaining,
    Blocked,
    AwaitingUserJudgment,
    InFlight,
}

impl CodeggCompletionFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionableWorkRemaining => "actionable_work_remaining",
            Self::Blocked => "blocked",
            Self::AwaitingUserJudgment => "awaiting_user_judgment",
            Self::InFlight => "in_flight",
        }
    }
}

/// Compatibility display family. Eggplan's detailed status and reasons stay
/// intact in the returned assessment; failure/missing/inconclusive fold into
/// CodeGG's existing actionable family with an explicit lossy marker.
pub fn completion_family(status: eggplan_core::AssessmentStatus) -> CodeggCompletionFamily {
    use eggplan_core::AssessmentStatus as Status;
    match status {
        Status::Complete => CodeggCompletionFamily::Complete,
        Status::Blocked => CodeggCompletionFamily::Blocked,
        Status::AwaitingHumanJudgment => CodeggCompletionFamily::AwaitingUserJudgment,
        Status::InFlight => CodeggCompletionFamily::InFlight,
        Status::ActionableWorkRemaining
        | Status::EvidenceFailed
        | Status::EvidenceMissingOrUnavailable
        | Status::Inconclusive
        | Status::InvalidOrStale => CodeggCompletionFamily::ActionableWorkRemaining,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityMapEntry {
    pub source_id: String,
    pub eggplan_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidenceRef {
    pub item_id: String,
    pub kind: CodeggEvidenceKind,
    pub ref_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerProvenance {
    pub item_id: String,
    pub owner_run_id: Option<String>,
    pub owner_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAcceptance {
    pub item_id: String,
    pub description: String,
    pub disposition: AcceptanceDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingManifest {
    pub schema_version: u32,
    pub source_plan_id: String,
    pub source_revision: i64,
    pub id_map: Vec<IdentityMapEntry>,
    pub source_evidence_refs: Vec<SourceEvidenceRef>,
    pub owner_provenance: Vec<OwnerProvenance>,
    pub source_acceptances: Vec<SourceAcceptance>,
    pub losses: Vec<Loss>,
    pub content_digest: String,
}

impl MappingManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.source_revision < 0 {
            return Err("unsupported mapping manifest version or revision".into());
        }
        let mut source_ids = BTreeSet::new();
        let mut target_ids = BTreeSet::new();
        for entry in &self.id_map {
            if !source_ids.insert(entry.source_id.as_str())
                || !target_ids.insert(entry.eggplan_id.as_str())
            {
                return Err("mapping manifest contains duplicate identities".into());
            }
        }
        let expected = digest_json(&(
            self.schema_version,
            &self.source_plan_id,
            self.source_revision,
            &self.id_map,
            &self.source_evidence_refs,
            &self.owner_provenance,
            &self.source_acceptances,
            &self.losses,
        ))
        .map_err(|error| error.to_string())?;
        if expected != self.content_digest {
            return Err("mapping manifest digest mismatch".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MappedPlan {
    /// Snapshot intent at revision zero. Persist with `create_snapshot` so the
    /// source lifecycle is reached through legal Eggplan CAS transitions.
    pub plan: Plan,
    pub observations: Vec<EvidenceObservation>,
    pub manifest: MappingManifest,
    source_status: CodeggPlanStatus,
    source_current_item_id: Option<String>,
}

impl MappedPlan {
    pub fn source_status(&self) -> CodeggPlanStatus {
        self.source_status
    }
}

#[derive(Debug, Clone)]
pub struct CodeggAssessmentBridgeResult {
    pub plan: Plan,
    pub manifest: MappingManifest,
    pub assessment: PlanAssessment,
    pub completion_family: CodeggCompletionFamily,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatProjection {
    pub schema_version: u32,
    pub assessment_reason_code: String,
    pub total_items: usize,
    pub completed_items: usize,
    pub actionable_count: usize,
    pub blocked_count: usize,
    pub in_progress_count: usize,
    pub truncated: bool,
    pub current_item_id: Option<String>,
    pub items: Vec<CompatItemSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatItemSummary {
    pub id: String,
    pub source_id: String,
    pub position: u32,
    pub status: PlanItemStatus,
    pub description: String,
    pub blocker: Option<String>,
    pub next_action: Option<String>,
}

fn hash_id(namespace: &str, source: &str) -> Result<String, String> {
    let digest = digest_json(&("eggplan-codegg-compat", namespace, source))
        .map_err(|error| error.to_string())?;
    let hex = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| "unexpected digest format".to_string())?;
    Ok(hex[..32].to_owned())
}

fn check_text(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty() || value.contains('\0') || value.chars().count() > MAX_TEXT_CHARS {
        Err(format!("invalid or oversized {field}"))
    } else {
        Ok(())
    }
}

fn check_source_id(value: &str, field: &str) -> Result<(), String> {
    check_text(value, field)?;
    if value.chars().count() > MAX_SOURCE_ID_CHARS {
        Err(format!("oversized {field}"))
    } else {
        Ok(())
    }
}

fn check_optional_text(value: Option<&str>, field: &str) -> Result<(), String> {
    if let Some(value) = value
        && (value.contains('\0') || value.chars().count() > MAX_DETAIL_CHARS)
    {
        return Err(format!("invalid or oversized {field}"));
    }
    Ok(())
}

fn check_source_id_opt(value: Option<&str>, field: &str) -> Result<(), String> {
    if let Some(value) = value {
        check_source_id(value, field)?;
    }
    Ok(())
}

fn map_kind(kind: CodeggEvidenceKind) -> EvidenceKind {
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

fn requires_verification_binding(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::Command
            | EvidenceKind::Test
            | EvidenceKind::StaticAnalysis
            | EvidenceKind::DelegatedRun
            | EvidenceKind::Benchmark
    )
}

fn map_item_status(status: CodeggItemStatus) -> PlanItemStatus {
    match status {
        CodeggItemStatus::Pending => PlanItemStatus::Pending,
        CodeggItemStatus::Actionable => PlanItemStatus::Actionable,
        CodeggItemStatus::InProgress => PlanItemStatus::InProgress,
        CodeggItemStatus::Blocked => PlanItemStatus::Blocked,
        CodeggItemStatus::Completed => PlanItemStatus::Completed,
        CodeggItemStatus::Cancelled => PlanItemStatus::Cancelled,
    }
}

pub fn normalize_fixture<R: EvidenceResolver>(
    fixture: &Fixture,
    subject: SubjectRevision,
    resolver: &mut R,
) -> Result<MappedPlan, String> {
    if fixture.schema_version != FIXTURE_SCHEMA_VERSION
        || fixture.source_repository != "codegg"
        || fixture.source_sha != SOURCE_CODEGG_SHA
    {
        return Err("unsupported fixture version, repository, or source SHA".into());
    }
    check_text(&fixture.source_repository, "source repository")?;
    check_text(&fixture.source_case, "source case")?;
    normalize_snapshot(&fixture.snapshot, subject, resolver)
}

/// Normalize a live CodeGG snapshot. Fixture provenance is intentionally
/// absent from this runtime path: source SHA is qualification metadata only.
pub fn normalize_snapshot<R: EvidenceResolver>(
    snapshot: &CodeggPlanSnapshot,
    subject: SubjectRevision,
    resolver: &mut R,
) -> Result<MappedPlan, String> {
    if snapshot.revision < 0 || snapshot.items.is_empty() || snapshot.items.len() > MAX_ITEMS {
        return Err("invalid snapshot revision or item count".into());
    }
    check_source_id(&snapshot.plan_id, "plan ID")?;
    check_text(&snapshot.objective, "objective")?;
    if let Some(current) = &snapshot.current_item_id {
        check_source_id(current, "current item ID")?;
    }
    check_optional_text(snapshot.current_phase.as_deref(), "current phase")?;
    subject.validate().map_err(|error| error.to_string())?;

    let plan_id = PlanId::new(format!("ep_{}", hash_id("plan", &snapshot.plan_id)?))
        .map_err(|error| error.to_string())?;
    let mut id_map = vec![IdentityMapEntry {
        source_id: snapshot.plan_id.clone(),
        eggplan_id: plan_id.to_string(),
    }];
    let mut mapped_ids = BTreeMap::new();
    let mut mapped_id_values = BTreeSet::new();
    let mut losses = BTreeSet::new();
    let mut source_evidence_refs = Vec::new();
    let mut owner_provenance = Vec::new();
    let mut source_acceptances = Vec::new();
    losses.insert(Loss::SourceRevisionIsProvenanceOnly);
    if snapshot.status == CodeggPlanStatus::Completed {
        losses.insert(Loss::CompletedPlanNeedsGuardedClose);
    }

    for item in &snapshot.items {
        check_source_id(&item.item_id, "item ID")?;
        check_text(&item.description, "item description")?;
        if item.position < 0
            || item.position > u32::MAX as i64
            || item.acceptance.len() > MAX_ACCEPTANCES_PER_ITEM
            || item.evidence.len() > MAX_EVIDENCE_REFS_PER_ITEM
            || item.dependencies.len() > eggplan_core::bounds::MAX_DEPENDENCIES
        {
            return Err("item position or bounded collection limit is invalid".into());
        }
        check_optional_text(item.blocker.as_deref(), "blocker")?;
        check_optional_text(item.next_action.as_deref(), "next action")?;
        for reference in item.parent_item_id.iter().chain(item.dependencies.iter()) {
            check_source_id(reference, "parent or dependency ID")?;
        }
        check_source_id_opt(item.owner_run_id.as_deref(), "owner run ID")?;
        check_source_id_opt(item.owner_job_id.as_deref(), "owner job ID")?;
        for acceptance in &item.acceptance {
            check_text(&acceptance.description, "acceptance description")?;
            check_optional_text(acceptance.note.as_deref(), "acceptance note")?;
        }
        for reference in &item.evidence {
            check_source_id(&reference.ref_id, "evidence reference")?;
            check_optional_text(reference.detail.as_deref(), "evidence detail")?;
        }
        let id = PlanItemId::new(format!("epi_{}", hash_id("item", &item.item_id)?))
            .map_err(|error| error.to_string())?;
        if !mapped_id_values.insert(id.clone()) {
            return Err("deterministic item ID collision".into());
        }
        if mapped_ids
            .insert(item.item_id.clone(), id.clone())
            .is_some()
        {
            return Err("duplicate source item ID".into());
        }
        id_map.push(IdentityMapEntry {
            source_id: item.item_id.clone(),
            eggplan_id: id.to_string(),
        });
        if item.owner_run_id.is_some() || item.owner_job_id.is_some() {
            losses.insert(Loss::OwnerReferencesAreProvenanceOnly);
        }
        if item.owner_run_id.is_some() || item.owner_job_id.is_some() {
            owner_provenance.push(OwnerProvenance {
                item_id: item.item_id.clone(),
                owner_run_id: item.owner_run_id.clone(),
                owner_job_id: item.owner_job_id.clone(),
            });
        }
        for acceptance in &item.acceptance {
            if acceptance.disposition == AcceptanceDisposition::Satisfied {
                losses.insert(Loss::SerializedDispositionIsNotEvidence);
            }
            source_acceptances.push(SourceAcceptance {
                item_id: item.item_id.clone(),
                description: acceptance.description.clone(),
                disposition: acceptance.disposition,
            });
            if acceptance
                .note
                .as_ref()
                .is_some_and(|note| !note.is_empty())
            {
                losses.insert(Loss::AcceptanceNoteOmitted);
            }
        }
        source_evidence_refs.extend(item.evidence.iter().map(|reference| SourceEvidenceRef {
            item_id: item.item_id.clone(),
            kind: reference.kind,
            ref_id: reference.ref_id.clone(),
        }));
        if item.evidence.iter().any(|reference| {
            reference
                .detail
                .as_ref()
                .is_some_and(|detail| !detail.is_empty())
        }) {
            losses.insert(Loss::EvidenceDetailOmitted);
        }
    }
    if snapshot
        .current_item_id
        .as_ref()
        .is_some_and(|current| !mapped_ids.contains_key(current))
    {
        return Err("current item ID does not identify a snapshot item".into());
    }

    let mut observations = Vec::new();
    let mut observation_ids = BTreeSet::new();
    let mut items = Vec::with_capacity(snapshot.items.len());
    for item in &snapshot.items {
        let mut requirements = Vec::new();
        for reference in &item.evidence {
            let resolved = resolver.resolve(reference, &subject)?;
            let kind = map_kind(reference.kind);
            if resolved.observation.subject() != &subject
                || resolved.observation.kind() != kind
                || !observation_ids.insert(resolved.observation.id().clone())
            {
                return Err("resolver returned stale, mismatched, or reused evidence".into());
            }
            if requires_verification_binding(kind)
                && (resolved.expected_verification.is_none()
                    || resolved.expected_verification.as_ref()
                        != resolved.observation.verification_digest())
            {
                return Err(
                    "execution evidence lacks the authoritative matching verification binding"
                        .into(),
                );
            }
            requirements.push(EvidenceRequirement {
                description: format!("CodeGG {:?} ref {}", reference.kind, reference.ref_id),
                kind,
                provider: None,
                subject_policy: SubjectPolicy::Exact,
                cardinality: EvidenceCardinality::Any,
                min_count: 1,
                allow_human_judgment: false,
                expected_verification_digest: resolved.expected_verification,
            });
            observations.push(resolved.observation);
        }
        if item.acceptance.len() > 1 && !item.evidence.is_empty() {
            losses.insert(Loss::AcceptanceEvidenceAssignmentIsItemScoped);
        }
        let criteria = item
            .acceptance
            .iter()
            .enumerate()
            .map(|(index, acceptance)| {
                let human = acceptance.disposition == AcceptanceDisposition::RequiresUserJudgment;
                Ok(AcceptanceCriterion {
                    id: eggplan_core::CriterionId::new(format!("epc_{index:04x}"))
                        .map_err(|error| error.to_string())?,
                    statement: acceptance.description.clone(),
                    human_judgment_allowed: human,
                    requirements: if human {
                        Vec::new()
                    } else {
                        requirements.clone()
                    },
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let parent = item
            .parent_item_id
            .as_ref()
            .map(|source| {
                mapped_ids
                    .get(source)
                    .cloned()
                    .ok_or_else(|| format!("missing parent item {source}"))
            })
            .transpose()?;
        let dependencies = item
            .dependencies
            .iter()
            .map(|source| {
                mapped_ids
                    .get(source)
                    .cloned()
                    .ok_or_else(|| format!("missing dependency item {source}"))
            })
            .collect::<Result<Vec<_>, String>>()?;
        items.push(PlanItem {
            id: mapped_ids[&item.item_id].clone(),
            position: item.position as u32,
            parent,
            dependencies,
            status: map_item_status(item.status),
            description: item.description.clone(),
            criteria,
            blocker: item.blocker.clone(),
            next_action: item.next_action.clone(),
        });
    }
    items.sort_by_key(|item| (item.position, item.id.clone()));
    id_map.sort_by(|a, b| a.source_id.cmp(&b.source_id));
    let mut provenance = BTreeMap::new();
    provenance.insert("codegg_source_plan".into(), snapshot.plan_id.clone());
    provenance.insert(
        "codegg_source_revision".into(),
        snapshot.revision.to_string(),
    );
    if let Some(current) = &snapshot.current_item_id {
        provenance.insert("codegg_current_item".into(), current.clone());
    }
    if let Some(phase) = &snapshot.current_phase {
        provenance.insert("codegg_current_phase".into(), phase.clone());
    }
    let mut plan =
        Plan::new(plan_id, snapshot.objective.clone(), items).map_err(|error| error.to_string())?;
    plan.subject = Some(subject);
    plan.provenance = provenance;
    plan.validate().map_err(|error| error.to_string())?;

    let mut manifest = MappingManifest {
        schema_version: 1,
        source_plan_id: snapshot.plan_id.clone(),
        source_revision: snapshot.revision,
        id_map,
        source_evidence_refs,
        owner_provenance,
        source_acceptances,
        losses: losses.into_iter().collect(),
        content_digest: String::new(),
    };
    manifest.content_digest = digest_json(&(
        manifest.schema_version,
        &manifest.source_plan_id,
        manifest.source_revision,
        &manifest.id_map,
        &manifest.source_evidence_refs,
        &manifest.owner_provenance,
        &manifest.source_acceptances,
        &manifest.losses,
    ))
    .map_err(|error| error.to_string())?;
    observations.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(MappedPlan {
        plan,
        observations,
        manifest,
        source_status: snapshot.status,
        source_current_item_id: snapshot.current_item_id.clone(),
    })
}

/// Pure live assessment bridge. CodeGG supplies the exact subject, resolves
/// native evidence and supplies provider policy; this function owns no store.
pub fn assess_codegg_snapshot<R: EvidenceResolver>(
    snapshot: &CodeggPlanSnapshot,
    subject: SubjectRevision,
    resolver: &mut R,
    providers: &ProviderRegistry,
) -> Result<CodeggAssessmentBridgeResult, String> {
    let mut mapped = normalize_snapshot(snapshot, subject.clone(), resolver)?;
    mapped.plan.status = match mapped.source_status {
        CodeggPlanStatus::Active | CodeggPlanStatus::Completed => PlanStatus::Active,
        CodeggPlanStatus::Blocked => PlanStatus::Blocked,
        CodeggPlanStatus::Cancelled => PlanStatus::Cancelled,
    };
    let assessment = assess_plan(&mapped.plan, &subject, &mapped.observations, providers);
    let completion_family = completion_family(assessment.status);
    let mut reason_codes: Vec<String> = assessment
        .reasons
        .iter()
        .map(assessment_reason_code)
        .collect();
    for item in &assessment.items {
        for reason in &item.reasons {
            reason_codes.push(assessment_reason_code(reason));
        }
        for criterion in &item.criteria {
            for reason in &criterion.reasons {
                reason_codes.push(assessment_reason_code(reason));
            }
        }
    }
    reason_codes.sort();
    reason_codes.dedup();
    Ok(CodeggAssessmentBridgeResult {
        plan: mapped.plan,
        manifest: mapped.manifest,
        assessment,
        completion_family,
        reason_codes,
    })
}

fn assessment_reason_code(reason: &AssessmentReason) -> String {
    match reason {
        AssessmentReason::MissingObservation => "missing_observation",
        AssessmentReason::NoAcceptanceCriteria => "no_acceptance_criteria",
        AssessmentReason::NoEvidenceRequirement => "no_evidence_requirement",
        AssessmentReason::HumanJudgmentRequired => "human_judgment_required",
        AssessmentReason::StaleSubject(_) => "stale_subject",
        AssessmentReason::UntrustedProvider(_) => "untrusted_provider",
        AssessmentReason::ProviderKindNotAllowed(_) => "provider_kind_not_allowed",
        AssessmentReason::ProviderMismatch(_) => "provider_mismatch",
        AssessmentReason::LegacyUnboundExecutionRequirement => {
            "legacy_unbound_execution_requirement"
        }
        AssessmentReason::ObservationMissingVerificationBinding(_) => {
            "observation_missing_verification_binding"
        }
        AssessmentReason::VerificationDigestMismatch(_) => "verification_digest_mismatch",
        AssessmentReason::EvidenceStatus(_, _) => "evidence_status",
        AssessmentReason::HumanJudgmentNotAllowed => "human_judgment_not_allowed",
        AssessmentReason::InvalidObservation(_) => "invalid_observation",
        AssessmentReason::DuplicateObservationId(_) => "duplicate_observation_id",
        AssessmentReason::PlanNotActive => "plan_not_active",
        AssessmentReason::InvalidPlan => "invalid_plan",
        AssessmentReason::ItemNotCompleted(_) => "item_not_completed",
        AssessmentReason::PlanBlocked => "plan_blocked",
        AssessmentReason::PlanCancelled => "plan_cancelled",
        AssessmentReason::NoPlanItems => "no_plan_items",
    }
    .to_owned()
}

pub fn project_bounded(
    mapped: &MappedPlan,
    assessment_status: eggplan_core::AssessmentStatus,
    max_items: usize,
    max_text_chars: usize,
) -> CompatProjection {
    let max_items = max_items.clamp(1, 8);
    let max_text_chars = max_text_chars.clamp(1, 200);
    let source_current = mapped.source_current_item_id.as_deref();
    let mut candidates: Vec<_> = mapped.plan.items.iter().collect();
    candidates.sort_by_key(|item| {
        let source = mapped
            .manifest
            .id_map
            .iter()
            .find(|entry| entry.eggplan_id == item.id.as_str())
            .map(|entry| entry.source_id.as_str());
        let status_rank = match item.status {
            PlanItemStatus::Pending | PlanItemStatus::Actionable => 0,
            PlanItemStatus::InProgress => 1,
            PlanItemStatus::Blocked => 2,
            PlanItemStatus::Completed | PlanItemStatus::Cancelled => 3,
        };
        (
            usize::from(source != source_current),
            status_rank,
            item.position,
            item.id.clone(),
        )
    });
    let all_count = mapped.plan.items.len();
    let completed_items = mapped
        .plan
        .items
        .iter()
        .filter(|item| item.status == PlanItemStatus::Completed)
        .count();
    let actionable_count = mapped
        .plan
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                PlanItemStatus::Pending | PlanItemStatus::Actionable
            )
        })
        .count();
    let blocked_count = mapped
        .plan
        .items
        .iter()
        .filter(|item| item.status == PlanItemStatus::Blocked)
        .count();
    let in_progress_count = mapped
        .plan
        .items
        .iter()
        .filter(|item| item.status == PlanItemStatus::InProgress)
        .count();
    let mut items: Vec<_> = candidates
        .into_iter()
        .filter(|item| {
            !matches!(
                item.status,
                PlanItemStatus::Completed | PlanItemStatus::Cancelled
            )
        })
        .map(|item| {
            let source_id = mapped
                .manifest
                .id_map
                .iter()
                .find(|entry| entry.eggplan_id == item.id.as_str())
                .map(|entry| entry.source_id.clone())
                .unwrap_or_default();
            CompatItemSummary {
                id: item.id.to_string(),
                source_id,
                position: item.position,
                status: item.status,
                description: item.description.chars().take(max_text_chars).collect(),
                blocker: item
                    .blocker
                    .as_ref()
                    .map(|text| text.chars().take(max_text_chars).collect()),
                next_action: item
                    .next_action
                    .as_ref()
                    .map(|text| text.chars().take(max_text_chars).collect()),
            }
        })
        .collect();
    let nonterminal_items = items.len();
    items.truncate(max_items);
    let current_item_id = mapped
        .source_current_item_id
        .as_ref()
        .and_then(|source| {
            mapped
                .manifest
                .id_map
                .iter()
                .find(|entry| &entry.source_id == source)
        })
        .and_then(|entry| {
            mapped
                .plan
                .items
                .iter()
                .find(|item| item.id.as_str() == entry.eggplan_id)
                .filter(|item| {
                    !matches!(
                        item.status,
                        PlanItemStatus::Completed | PlanItemStatus::Cancelled
                    )
                })
                .map(|item| item.id.to_string())
        })
        .or_else(|| {
            mapped
                .plan
                .items
                .iter()
                .find(|item| {
                    item.status != PlanItemStatus::Completed
                        && item.status != PlanItemStatus::Cancelled
                })
                .map(|item| item.id.to_string())
        });
    CompatProjection {
        schema_version: 1,
        assessment_reason_code: completion_family(assessment_status).as_str().into(),
        total_items: all_count,
        completed_items,
        actionable_count,
        blocked_count,
        in_progress_count,
        truncated: nonterminal_items > items.len(),
        current_item_id,
        items,
    }
}
