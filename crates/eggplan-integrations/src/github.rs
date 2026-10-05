//! Bounded DTO adapter for host-acquired GitHub forge evidence.
//!
//! The host owns every interaction with GitHub: authentication, repository
//! access, API version selection, pagination, run/check discovery, artifact
//! retrieval, rate-limit handling, and any attestation work. This module only
//! decodes strict bounded DTOs a host has already fetched and constructs
//! normalized observations. It contains no HTTP client, no async runtime, no
//! process execution, and no credential handling.
//!
//! GitHub is the first forge target because it is already Eggplan's hosted
//! qualification authority. Nothing here is GitHub-specific at the core layer:
//! `eggplan-core` is untouched, and a later GitLab or other forge adapter can
//! be added beside this one.
//!
//! Reviewed API contract: [`REVIEWED_GITHUB_API_VERSION`].
//!
//! Three separate claims are kept apart:
//!
//! 1. execution outcome — a run or check ran, and its native terminal result;
//! 2. artifact availability — a produced artifact is currently retrievable;
//! 3. commit identity — an exact commit exists at a repository.
//!
//! None of them implies review, safety, merge state, or criterion
//! satisfaction.

use crate::{
    AdapterDescriptor, Capabilities, NormalizedProviderResult, ObservationContext, ProviderClass,
    SpiError, finalize_observation,
};
use eggplan_core::{
    ArtifactRef, EvidenceKind, EvidenceObservation, EvidenceProviderId, EvidenceStatus,
    SubjectState,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// GitHub REST API version whose object shapes these DTOs were qualified
/// against. `2022-11-28` is the stable Actions/Checks/status API version.
pub const REVIEWED_GITHUB_API_VERSION: &str = "2022-11-28";

/// Eggplan-owned DTO schema versions. Distinct from any upstream version.
pub const RUN_DTO_SCHEMA: u32 = 1;
pub const CHECK_DTO_SCHEMA: u32 = 1;
pub const COMMIT_STATUS_DTO_SCHEMA: u32 = 1;
pub const ARTIFACT_DTO_SCHEMA: u32 = 1;
pub const REVISION_DTO_SCHEMA: u32 = 1;

pub const MAX_DTO_BYTES: usize = 262_144;
pub const MAX_JOBS: usize = 256;
pub const MAX_COMMIT_STATUSES: usize = 256;
pub const MAX_REPOSITORY_NAME_CHARS: usize = 256;
pub const MAX_WORKFLOW_NAME_CHARS: usize = 256;
pub const MAX_WORKFLOW_PATH_CHARS: usize = 512;
pub const MAX_JOB_NAME_CHARS: usize = 256;
pub const MAX_CHECK_NAME_CHARS: usize = 256;
pub const MAX_STATUS_CONTEXT_CHARS: usize = 256;
pub const MAX_ARTIFACT_NAME_CHARS: usize = 256;
pub const MAX_RUN_HANDLE_CHARS: usize = 256;

const ACTIONS_CAPS: Capabilities = Capabilities {
    supports_in_progress: true,
    artifacts: true,
    verification_binding: true,
    research_trust_metadata: false,
};

const FORGE_CAPS: Capabilities = Capabilities {
    // Revision identity and artifact availability are terminal facts.
    supports_in_progress: false,
    artifacts: true,
    // Deliberately false: no forge fact may carry a verification binding, so an
    // artifact digest can never be replayed as a verification digest.
    verification_binding: false,
    research_trust_metadata: false,
};

// ---------------------------------------------------------------------------
// Upstream enums, reproduced as strict Eggplan-owned vocabulary.
// ---------------------------------------------------------------------------

/// Workflow run / job status. GitHub sets `conclusion` only once `completed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
    Waiting,
    Requested,
    Pending,
}

/// Workflow run / job conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunConclusion {
    Success,
    Failure,
    Neutral,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
    Stale,
    StartupFailure,
}

/// Check-run status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Queued,
    InProgress,
    Completed,
    Waiting,
    Requested,
    Pending,
}

/// Check-run conclusion. `stale` and `startup_failure` are documented for the
/// Actions surface but are not part of the REST check-run enum; accepting them
/// here costs nothing because every value maps to a non-passing status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion {
    Success,
    Failure,
    Neutral,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
}

/// Classic commit status state. There is no `neutral` and no `pending`
/// conclusion here; `error` is a distinct terminal failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitStatusState {
    Pending,
    Success,
    Failure,
    Error,
}

// ---------------------------------------------------------------------------
// Workflow run DTO
// ---------------------------------------------------------------------------

/// Bounded projection of one GitHub Actions workflow run.
///
/// `run_attempt` is part of execution identity: a rerun is a distinct execution
/// and is never collapsed into the prior attempt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubActionsRunV1 {
    pub schema_version: u32,
    pub repository_id: u64,
    pub repository_full_name: String,
    pub workflow_id: u64,
    pub workflow_path: String,
    pub workflow_name: String,
    pub run_id: u64,
    pub run_number: u64,
    /// Always >= 1; a rerun increments this.
    pub run_attempt: u32,
    pub event: String,
    /// The exact tested commit.
    pub head_sha: String,
    pub status: RunStatus,
    pub conclusion: Option<RunConclusion>,
    pub jobs: Vec<JobSummary>,
    pub created_at_unix_ms: u64,
    #[serde(default)]
    pub started_at_unix_ms: Option<u64>,
    #[serde(default)]
    pub completed_at_unix_ms: Option<u64>,
    /// Bounded operator navigation handle. Never a signed or temporary URL.
    #[serde(default)]
    pub run_handle: Option<String>,
}

/// One bounded job summary inside a run. No logs, annotations, runner secrets,
/// or failure prose are retained.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobSummary {
    pub job_id: u64,
    pub name: String,
    /// Present when GitHub supplies it; must agree with the run's head SHA.
    #[serde(default)]
    pub head_sha: Option<String>,
    pub status: RunStatus,
    pub conclusion: Option<RunConclusion>,
    /// Bounded per-conclusion step counts, not step names or output.
    #[serde(default)]
    pub step_conclusions: BTreeMap<String, u32>,
}

// ---------------------------------------------------------------------------
// Check-run and classic commit status DTOs
// ---------------------------------------------------------------------------

/// Bounded projection of one forge check run.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubCheckRunV1 {
    pub schema_version: u32,
    pub repository_id: u64,
    /// The exact commit the check ran against.
    pub commit_sha: String,
    pub check_run_id: u64,
    pub name: String,
    #[serde(default)]
    pub app_slug: Option<String>,
    pub status: CheckStatus,
    pub conclusion: Option<CheckConclusion>,
    #[serde(default)]
    pub started_at_unix_ms: Option<u64>,
    #[serde(default)]
    pub completed_at_unix_ms: Option<u64>,
}

/// Bounded projection of a commit's classic status list. The host supplies the
/// entries; this adapter never reinterprets branch names or protection rules.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubCommitStatusV1 {
    pub schema_version: u32,
    pub repository_id: u64,
    pub commit_sha: String,
    pub statuses: Vec<CommitStatusEntryV1>,
}

/// One classic commit status entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitStatusEntryV1 {
    pub status_id: u64,
    pub context: String,
    pub state: CommitStatusState,
}

// ---------------------------------------------------------------------------
// Artifact DTO
// ---------------------------------------------------------------------------

/// Bounded projection of one Actions artifact's metadata.
///
/// GitHub supplies `digest` only for artifacts uploaded with `upload-artifact`
/// v4 or newer, and it is `null` for older uploads. An absent digest is
/// recorded as absent, never inferred from the artifact name or size.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubArtifactV1 {
    pub schema_version: u32,
    pub repository_id: u64,
    pub artifact_id: u64,
    pub name: String,
    pub run_id: u64,
    pub run_attempt: u32,
    /// The exact commit whose run produced this artifact.
    pub head_sha: String,
    pub size_in_bytes: u64,
    pub expired: bool,
    pub created_at_unix_ms: u64,
    #[serde(default)]
    pub expires_at_unix_ms: Option<u64>,
    /// `sha256:<64 lowercase hex>` when GitHub supplies one.
    #[serde(default)]
    pub digest: Option<String>,
}

// ---------------------------------------------------------------------------
// Revision DTO
// ---------------------------------------------------------------------------

/// Bounded forge commit identity.
///
/// This is provenance: it asserts that an exact commit exists at a repository.
/// It does not assert that the commit is reviewed, safe, merged, tagged, or
/// release-worthy. No mutable branch or tag name is accepted as identity.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubRevisionV1 {
    pub schema_version: u32,
    pub repository_id: u64,
    pub repository_full_name: String,
    pub commit_sha: String,
}

// ---------------------------------------------------------------------------
// Provider descriptors
// ---------------------------------------------------------------------------

/// Adapter-fixed execution provider identity.
pub fn actions_descriptor() -> AdapterDescriptor {
    AdapterDescriptor::new(
        EvidenceProviderId::new("epp_github_actions").expect("static id"),
        ProviderClass::Execution,
        [
            EvidenceKind::Test,
            EvidenceKind::Command,
            EvidenceKind::DelegatedRun,
            EvidenceKind::Benchmark,
        ],
        "1.0",
        ACTIONS_CAPS,
    )
    .expect("static descriptor")
}

/// Adapter-fixed forge provenance provider identity.
pub fn forge_descriptor() -> AdapterDescriptor {
    AdapterDescriptor::new(
        EvidenceProviderId::new("epp_github_forge").expect("static id"),
        ProviderClass::Utility,
        [EvidenceKind::Revision, EvidenceKind::Artifact],
        "1.0",
        FORGE_CAPS,
    )
    .expect("static descriptor")
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

fn parse_bounded<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    label: &'static str,
) -> Result<T, SpiError> {
    if bytes.len() > MAX_DTO_BYTES {
        return Err(SpiError::Invalid(label));
    }
    serde_json::from_slice(bytes).map_err(|_| SpiError::Invalid(label))
}

pub fn parse_run(bytes: &[u8]) -> Result<GitHubActionsRunV1, SpiError> {
    let value: GitHubActionsRunV1 =
        parse_bounded(bytes, "invalid or oversize GitHub workflow run DTO")?;
    validate_run(&value)?;
    Ok(value)
}

pub fn parse_check_run(bytes: &[u8]) -> Result<GitHubCheckRunV1, SpiError> {
    let value: GitHubCheckRunV1 = parse_bounded(bytes, "invalid or oversize GitHub check run DTO")?;
    validate_check_run(&value)?;
    Ok(value)
}

pub fn parse_commit_status(bytes: &[u8]) -> Result<GitHubCommitStatusV1, SpiError> {
    let value: GitHubCommitStatusV1 =
        parse_bounded(bytes, "invalid or oversize GitHub commit status DTO")?;
    validate_commit_status(&value)?;
    Ok(value)
}

pub fn parse_artifact(bytes: &[u8]) -> Result<GitHubArtifactV1, SpiError> {
    let value: GitHubArtifactV1 = parse_bounded(bytes, "invalid or oversize GitHub artifact DTO")?;
    validate_artifact(&value)?;
    Ok(value)
}

pub fn parse_revision(bytes: &[u8]) -> Result<GitHubRevisionV1, SpiError> {
    let value: GitHubRevisionV1 = parse_bounded(bytes, "invalid or oversize GitHub revision DTO")?;
    validate_revision(&value)?;
    Ok(value)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_run(value: &GitHubActionsRunV1) -> Result<(), SpiError> {
    if value.schema_version != RUN_DTO_SCHEMA {
        return Err(SpiError::Invalid("unknown GitHub run DTO schema"));
    }
    check_repository_id(value.repository_id)?;
    check_text(&value.repository_full_name, MAX_REPOSITORY_NAME_CHARS)?;
    check_repository_full_name(&value.repository_full_name)?;
    check_text(&value.workflow_path, MAX_WORKFLOW_PATH_CHARS)?;
    check_relative_workflow_path(&value.workflow_path)?;
    check_text(&value.workflow_name, MAX_WORKFLOW_NAME_CHARS)?;
    check_text(&value.event, MAX_STATUS_CONTEXT_CHARS)?;
    check_commit_sha(&value.head_sha)?;
    if value.run_id == 0 || value.workflow_id == 0 || value.run_number == 0 {
        return Err(SpiError::Invalid("invalid GitHub run identity"));
    }
    // Attempt numbering is execution identity, so it must be present and sane.
    if value.run_attempt == 0 {
        return Err(SpiError::Invalid("GitHub run attempt must be positive"));
    }
    check_terminal_consistency(value.status, value.conclusion)?;
    if value.jobs.len() > MAX_JOBS {
        return Err(SpiError::Invalid("GitHub job limit exceeded"));
    }
    if let (Some(started), Some(completed)) = (value.started_at_unix_ms, value.completed_at_unix_ms)
        && completed < started
    {
        return Err(SpiError::Invalid("GitHub run completion precedes start"));
    }
    if let Some(handle) = &value.run_handle {
        check_navigation_handle(handle)?;
    }
    let mut job_ids = BTreeSet::new();
    for job in &value.jobs {
        if job.job_id == 0 || !job_ids.insert(job.job_id) {
            return Err(SpiError::Invalid("invalid or duplicate GitHub job id"));
        }
        check_text(&job.name, MAX_JOB_NAME_CHARS)?;
        if let Some(sha) = &job.head_sha {
            check_commit_sha(sha)?;
            // A job's tested commit must be the run's tested commit.
            if sha != &value.head_sha {
                return Err(SpiError::Invalid(
                    "GitHub job head SHA disagrees with its run head SHA",
                ));
            }
        }
        check_terminal_consistency(job.status, job.conclusion)?;
        for (conclusion, count) in &job.step_conclusions {
            check_stable_label(conclusion)?;
            if *count == 0 {
                return Err(SpiError::Invalid("GitHub step conclusion count is zero"));
            }
        }
    }
    Ok(())
}

fn validate_check_run(value: &GitHubCheckRunV1) -> Result<(), SpiError> {
    if value.schema_version != CHECK_DTO_SCHEMA {
        return Err(SpiError::Invalid("unknown GitHub check run DTO schema"));
    }
    check_repository_id(value.repository_id)?;
    check_commit_sha(&value.commit_sha)?;
    if value.check_run_id == 0 {
        return Err(SpiError::Invalid("invalid GitHub check run id"));
    }
    check_text(&value.name, MAX_CHECK_NAME_CHARS)?;
    if let Some(slug) = &value.app_slug {
        // GitHub app slugs commonly contain hyphens, e.g. `github-actions`.
        check_slug(slug)?;
    }
    let conclusion = value.conclusion.map(|value| match value {
        CheckConclusion::Success => RunConclusion::Success,
        CheckConclusion::Failure => RunConclusion::Failure,
        CheckConclusion::Neutral => RunConclusion::Neutral,
        CheckConclusion::Cancelled => RunConclusion::Cancelled,
        CheckConclusion::Skipped => RunConclusion::Skipped,
        CheckConclusion::TimedOut => RunConclusion::TimedOut,
        CheckConclusion::ActionRequired => RunConclusion::ActionRequired,
    });
    check_terminal_consistency(
        match value.status {
            CheckStatus::Queued => RunStatus::Queued,
            CheckStatus::InProgress => RunStatus::InProgress,
            CheckStatus::Completed => RunStatus::Completed,
            CheckStatus::Waiting => RunStatus::Waiting,
            CheckStatus::Requested => RunStatus::Requested,
            CheckStatus::Pending => RunStatus::Pending,
        },
        conclusion,
    )?;
    if let (Some(started), Some(completed)) = (value.started_at_unix_ms, value.completed_at_unix_ms)
        && completed < started
    {
        return Err(SpiError::Invalid("GitHub check completion precedes start"));
    }
    Ok(())
}

fn validate_commit_status(value: &GitHubCommitStatusV1) -> Result<(), SpiError> {
    if value.schema_version != COMMIT_STATUS_DTO_SCHEMA {
        return Err(SpiError::Invalid("unknown GitHub commit status DTO schema"));
    }
    check_repository_id(value.repository_id)?;
    check_commit_sha(&value.commit_sha)?;
    if value.statuses.len() > MAX_COMMIT_STATUSES {
        return Err(SpiError::Invalid("GitHub commit status limit exceeded"));
    }
    let mut ids = BTreeSet::new();
    let mut contexts = BTreeSet::new();
    for status in &value.statuses {
        if status.status_id == 0 || !ids.insert(status.status_id) {
            return Err(SpiError::Invalid("invalid or duplicate GitHub status id"));
        }
        check_text(&status.context, MAX_STATUS_CONTEXT_CHARS)?;
        if !contexts.insert(status.context.as_str()) {
            return Err(SpiError::Invalid("duplicate GitHub status context"));
        }
    }
    Ok(())
}

fn validate_artifact(value: &GitHubArtifactV1) -> Result<(), SpiError> {
    if value.schema_version != ARTIFACT_DTO_SCHEMA {
        return Err(SpiError::Invalid("unknown GitHub artifact DTO schema"));
    }
    check_repository_id(value.repository_id)?;
    if value.artifact_id == 0 || value.run_id == 0 || value.run_attempt == 0 {
        return Err(SpiError::Invalid("invalid GitHub artifact identity"));
    }
    check_text(&value.name, MAX_ARTIFACT_NAME_CHARS)?;
    check_commit_sha(&value.head_sha)?;
    if let Some(expires) = value.expires_at_unix_ms
        && expires < value.created_at_unix_ms
    {
        return Err(SpiError::Invalid(
            "GitHub artifact expiry precedes creation",
        ));
    }
    if let Some(digest) = &value.digest {
        check_github_digest(digest)?;
    }
    Ok(())
}

fn validate_revision(value: &GitHubRevisionV1) -> Result<(), SpiError> {
    if value.schema_version != REVISION_DTO_SCHEMA {
        return Err(SpiError::Invalid("unknown GitHub revision DTO schema"));
    }
    check_repository_id(value.repository_id)?;
    check_repository_full_name(&value.repository_full_name)?;
    check_commit_sha(&value.commit_sha)?;
    Ok(())
}

/// GitHub sets `conclusion` only when `status` is `completed`, and vice versa.
/// A DTO that disagrees with that invariant is rejected rather than guessed.
fn check_terminal_consistency(
    status: RunStatus,
    conclusion: Option<RunConclusion>,
) -> Result<(), SpiError> {
    match (status, conclusion) {
        (RunStatus::Completed, Some(_)) => Ok(()),
        // Absent while completed, or present while still running: both are
        // contradictions a host must resolve rather than a guess for Eggplan.
        (RunStatus::Completed, None) | (_, Some(_)) => Err(SpiError::Invalid(
            "GitHub status and conclusion disagree about terminality",
        )),
        (_, None) => Ok(()),
    }
}

fn check_repository_id(value: u64) -> Result<(), SpiError> {
    if value == 0 {
        return Err(SpiError::Invalid("invalid GitHub repository id"));
    }
    Ok(())
}

/// `owner/repo`, no leading or trailing slash and exactly one separator.
fn check_repository_full_name(value: &str) -> Result<(), SpiError> {
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
        return Err(SpiError::Invalid("malformed GitHub repository full name"));
    }
    Ok(())
}

fn check_relative_workflow_path(value: &str) -> Result<(), SpiError> {
    if value.starts_with('/')
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(SpiError::Invalid("malformed GitHub workflow path"));
    }
    Ok(())
}

fn check_commit_sha(value: &str) -> Result<(), SpiError> {
    if value.len() != 40 || !is_lowercase_hex(value) {
        return Err(SpiError::Invalid("malformed GitHub commit SHA"));
    }
    Ok(())
}

/// GitHub artifact digests arrive as `sha256:<64 lowercase hex>`.
fn check_github_digest(value: &str) -> Result<(), SpiError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(SpiError::Invalid("malformed GitHub artifact digest"));
    };
    if hex.len() != 64 || !is_lowercase_hex(hex) {
        return Err(SpiError::Invalid("malformed GitHub artifact digest"));
    }
    Ok(())
}

fn is_lowercase_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A navigation handle is not a URL. Signed download links, API URLs, and
/// anything with userinfo or a query string are rejected outright.
fn check_navigation_handle(value: &str) -> Result<(), SpiError> {
    if value.is_empty()
        || value.chars().count() > MAX_RUN_HANDLE_CHARS
        || value.contains("://")
        || value.contains('@')
        || value.contains('?')
        || value.contains('#')
        || value.contains('\\')
        || value.contains('\0')
        || value
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
    {
        return Err(SpiError::Invalid("malformed GitHub navigation handle"));
    }
    Ok(())
}

fn check_text(value: &str, max: usize) -> Result<(), SpiError> {
    if value.is_empty()
        || value.contains('\0')
        || value.chars().count() > max
        || value.contains("://")
        || value.contains('@')
        || value.bytes().any(|b| b.is_ascii_control() && b != b'\t')
    {
        return Err(SpiError::Invalid(
            "malformed, unsafe, or oversized GitHub text field",
        ));
    }
    Ok(())
}

/// Lowercase slug charset, which permits the hyphens GitHub uses in app slugs.
fn check_slug(value: &str) -> Result<(), SpiError> {
    if value.is_empty()
        || value.len() > 64
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    {
        return Err(SpiError::Invalid("malformed GitHub slug"));
    }
    Ok(())
}

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
            "malformed GitHub stable snake_case label",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Status mapping
// ---------------------------------------------------------------------------

/// `conclusion` is present exactly when the run reached a terminal state, which
/// `check_terminal_consistency` enforces before this is reached. A terminal
/// conclusion therefore fully determines the normalized status.
fn map_execution(conclusion: Option<RunConclusion>) -> EvidenceStatus {
    match conclusion {
        None => EvidenceStatus::InProgress,
        Some(RunConclusion::Success) => EvidenceStatus::Passed,
        Some(RunConclusion::Failure | RunConclusion::TimedOut | RunConclusion::StartupFailure) => {
            EvidenceStatus::Failed
        }
        Some(RunConclusion::Cancelled | RunConclusion::Skipped) => EvidenceStatus::Skipped,
        Some(RunConclusion::ActionRequired) => EvidenceStatus::Blocked,
        // `neutral` and `stale` are neither success nor failure; guessing either
        // would manufacture authority the forge did not assert.
        Some(RunConclusion::Neutral | RunConclusion::Stale) => EvidenceStatus::Inconclusive,
    }
}

/// Normalize one classic commit status state. Exposed because a host may hold
/// individual contexts rather than a whole list.
///
/// Classic commit status has no neutral or stale vocabulary, so this mapping is
/// total over its four documented states.
pub fn map_commit_status(state: CommitStatusState) -> EvidenceStatus {
    match state {
        CommitStatusState::Pending => EvidenceStatus::InProgress,
        CommitStatusState::Success => EvidenceStatus::Passed,
        CommitStatusState::Failure | CommitStatusState::Error => EvidenceStatus::Failed,
    }
}

/// A set of classic statuses aggregates conservatively: any failure dominates,
/// then any pending, then all-success. An empty list is Inconclusive because it
/// asserts nothing at all.
fn aggregate_commit_status(statuses: &[CommitStatusEntryV1]) -> EvidenceStatus {
    if statuses.is_empty() {
        return EvidenceStatus::Inconclusive;
    }
    let mut pending = false;
    let mut success = false;
    for status in statuses {
        match status.state {
            CommitStatusState::Pending => pending = true,
            CommitStatusState::Success => success = true,
            CommitStatusState::Failure | CommitStatusState::Error => {
                return EvidenceStatus::Failed;
            }
        }
    }
    if pending {
        EvidenceStatus::InProgress
    } else if success {
        EvidenceStatus::Passed
    } else {
        EvidenceStatus::Inconclusive
    }
}

// ---------------------------------------------------------------------------
// Subject agreement
// ---------------------------------------------------------------------------

/// Execution-derived CI evidence must be revision exact.
///
/// For a Git Eggplan subject the forge's tested commit must equal the subject
/// revision. A mismatch is an adapter error, not stale-but-passing evidence.
/// This is also what prevents a pull-request merge-commit run from being
/// recorded as proof for a different source commit: the merge SHA simply will
/// not match, and the host must translate subjects explicitly or not at all.
///
/// A dirty subject is rejected for the same reason. Forge CI checks out a
/// commit; it cannot observe an uncommitted worktree, so a run can never be
/// evidence that the current dirty bytes passed.
fn check_exact_subject(tested_sha: &str, context: &ObservationContext) -> Result<(), SpiError> {
    if context.subject.subject_kind != "git" {
        return Ok(());
    }
    if tested_sha != context.subject.revision {
        return Err(SpiError::Invalid(
            "GitHub tested commit does not match the Eggplan Git subject",
        ));
    }
    if context.subject.state != SubjectState::Clean {
        return Err(SpiError::Invalid(
            "GitHub CI evidence cannot describe a dirty Eggplan Git subject",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Normalization: execution
// ---------------------------------------------------------------------------

/// Normalize one workflow run as execution-derived evidence.
pub fn normalize_run(
    run: &GitHubActionsRunV1,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_run(run)?;
    if !actions_descriptor()
        .allowed_kinds
        .contains(&context.requested_kind)
    {
        return Err(SpiError::UnsupportedKind);
    }
    check_exact_subject(&run.head_sha, context)?;

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "github_api_version".into(),
        REVIEWED_GITHUB_API_VERSION.into(),
    );
    metadata.insert("repository_id".into(), run.repository_id.to_string());
    metadata.insert(
        "repository_full_name".into(),
        run.repository_full_name.clone(),
    );
    metadata.insert("workflow_id".into(), run.workflow_id.to_string());
    metadata.insert("workflow_path".into(), run.workflow_path.clone());
    metadata.insert("workflow_name".into(), run.workflow_name.clone());
    metadata.insert("run_id".into(), run.run_id.to_string());
    metadata.insert("run_number".into(), run.run_number.to_string());
    metadata.insert("run_attempt".into(), run.run_attempt.to_string());
    metadata.insert("event".into(), run.event.clone());
    metadata.insert("head_sha".into(), run.head_sha.clone());
    metadata.insert("native_status".into(), run_label(run.status).to_string());
    if let Some(conclusion) = run.conclusion {
        metadata.insert(
            "native_conclusion".into(),
            conclusion_label(conclusion).to_string(),
        );
    }
    metadata.insert("job_count".into(), run.jobs.len().to_string());
    metadata.insert("job_conclusions".into(), job_conclusion_summary(&run.jobs));
    metadata.insert(
        "created_at_unix_ms".into(),
        run.created_at_unix_ms.to_string(),
    );
    if let Some(started) = run.started_at_unix_ms {
        metadata.insert("started_at_unix_ms".into(), started.to_string());
    }
    if let Some(completed) = run.completed_at_unix_ms {
        metadata.insert("completed_at_unix_ms".into(), completed.to_string());
    }
    if let Some(handle) = &run.run_handle {
        metadata.insert("run_handle".into(), handle.clone());
    }

    finalize_observation(
        &actions_descriptor(),
        context,
        &NormalizedProviderResult {
            status: map_execution(run.conclusion),
            source_trust: None,
            result_metadata: metadata,
            artifacts: Vec::new(),
        },
    )
}

/// Normalize one check run as execution-derived evidence.
pub fn normalize_check_run(
    check: &GitHubCheckRunV1,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_check_run(check)?;
    if !actions_descriptor()
        .allowed_kinds
        .contains(&context.requested_kind)
    {
        return Err(SpiError::UnsupportedKind);
    }
    check_exact_subject(&check.commit_sha, context)?;

    let status = match check.conclusion {
        None => EvidenceStatus::InProgress,
        Some(CheckConclusion::Success) => EvidenceStatus::Passed,
        Some(CheckConclusion::Failure | CheckConclusion::TimedOut) => EvidenceStatus::Failed,
        Some(CheckConclusion::Cancelled | CheckConclusion::Skipped) => EvidenceStatus::Skipped,
        Some(CheckConclusion::ActionRequired) => EvidenceStatus::Blocked,
        Some(CheckConclusion::Neutral) => EvidenceStatus::Inconclusive,
    };

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "github_api_version".into(),
        REVIEWED_GITHUB_API_VERSION.into(),
    );
    metadata.insert("repository_id".into(), check.repository_id.to_string());
    metadata.insert("commit_sha".into(), check.commit_sha.clone());
    metadata.insert("check_run_id".into(), check.check_run_id.to_string());
    metadata.insert("check_name".into(), check.name.clone());
    if let Some(slug) = &check.app_slug {
        metadata.insert("check_app".into(), slug.clone());
    }
    metadata.insert(
        "native_status".into(),
        check_label(check.status).to_string(),
    );
    if let Some(conclusion) = check.conclusion {
        metadata.insert(
            "native_conclusion".into(),
            check_conclusion_label(conclusion).to_string(),
        );
    }
    if let Some(started) = check.started_at_unix_ms {
        metadata.insert("started_at_unix_ms".into(), started.to_string());
    }
    if let Some(completed) = check.completed_at_unix_ms {
        metadata.insert("completed_at_unix_ms".into(), completed.to_string());
    }

    finalize_observation(
        &actions_descriptor(),
        context,
        &NormalizedProviderResult {
            status,
            source_trust: None,
            result_metadata: metadata,
            artifacts: Vec::new(),
        },
    )
}

/// Normalize a commit's classic status list as execution-derived evidence.
pub fn normalize_commit_status(
    status: &GitHubCommitStatusV1,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_commit_status(status)?;
    if !actions_descriptor()
        .allowed_kinds
        .contains(&context.requested_kind)
    {
        return Err(SpiError::UnsupportedKind);
    }
    check_exact_subject(&status.commit_sha, context)?;

    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    for entry in &status.statuses {
        *counts.entry(entry.state_label()).or_insert(0) += 1;
    }
    // An empty status list must still carry a bounded, non-empty summary.
    let summary = if counts.is_empty() {
        "none".to_string()
    } else {
        counts
            .iter()
            .map(|(state, count)| format!("{state}:{count}"))
            .collect::<Vec<_>>()
            .join(",")
    };

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "github_api_version".into(),
        REVIEWED_GITHUB_API_VERSION.into(),
    );
    metadata.insert("repository_id".into(), status.repository_id.to_string());
    metadata.insert("commit_sha".into(), status.commit_sha.clone());
    metadata.insert("status_count".into(), status.statuses.len().to_string());
    metadata.insert("status_states".into(), summary);

    finalize_observation(
        &actions_descriptor(),
        context,
        &NormalizedProviderResult {
            // A single successful status is not a passing commit test when other
            // statuses are pending, so aggregation is over the whole list.
            status: aggregate_commit_status(&status.statuses),
            source_trust: None,
            result_metadata: metadata,
            artifacts: Vec::new(),
        },
    )
}

impl CommitStatusEntryV1 {
    fn state_label(&self) -> &'static str {
        commit_status_label(self.state)
    }
}

// ---------------------------------------------------------------------------
// Normalization: forge provenance
// ---------------------------------------------------------------------------

/// Normalize one artifact's current availability.
///
/// This is a forge availability claim. It is deliberately independent of the
/// workflow run's own terminal result: an expired artifact does not retroactively
/// turn a successful run into a failure, and a successful run does not make an
/// expired artifact available.
pub fn normalize_artifact(
    artifact: &GitHubArtifactV1,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_artifact(artifact)?;
    if context.requested_kind != EvidenceKind::Artifact {
        return Err(SpiError::UnsupportedKind);
    }
    check_exact_subject(&artifact.head_sha, context)?;

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "github_api_version".into(),
        REVIEWED_GITHUB_API_VERSION.into(),
    );
    metadata.insert("repository_id".into(), artifact.repository_id.to_string());
    metadata.insert("artifact_id".into(), artifact.artifact_id.to_string());
    metadata.insert("artifact_name".into(), artifact.name.clone());
    metadata.insert("run_id".into(), artifact.run_id.to_string());
    metadata.insert("run_attempt".into(), artifact.run_attempt.to_string());
    metadata.insert("head_sha".into(), artifact.head_sha.clone());
    metadata.insert("size_in_bytes".into(), artifact.size_in_bytes.to_string());
    metadata.insert("expired".into(), artifact.expired.to_string());
    metadata.insert(
        "created_at_unix_ms".into(),
        artifact.created_at_unix_ms.to_string(),
    );
    if let Some(expires) = artifact.expires_at_unix_ms {
        metadata.insert("expires_at_unix_ms".into(), expires.to_string());
    }
    // GitHub populates `digest` only for upload-artifact v4 or newer.
    metadata.insert(
        "digest_present".into(),
        artifact.digest.is_some().to_string(),
    );

    let status = if artifact.expired {
        EvidenceStatus::Unavailable
    } else {
        EvidenceStatus::Passed
    };
    let refs = vec![ArtifactRef {
        reference: format!(
            "github-actions:artifact:{}:{}",
            artifact.repository_id, artifact.artifact_id
        ),
        digest: artifact.digest.clone(),
        media_type: Some("application/zip".to_string()),
    }];

    finalize_observation(
        &forge_descriptor(),
        context,
        &NormalizedProviderResult {
            status,
            source_trust: None,
            result_metadata: metadata,
            artifacts: refs,
        },
    )
}

/// Normalize an exact commit identity as forge provenance.
pub fn normalize_revision(
    revision: &GitHubRevisionV1,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_revision(revision)?;
    if context.requested_kind != EvidenceKind::Revision {
        return Err(SpiError::UnsupportedKind);
    }
    check_exact_subject(&revision.commit_sha, context)?;

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "github_api_version".into(),
        REVIEWED_GITHUB_API_VERSION.into(),
    );
    metadata.insert("repository_id".into(), revision.repository_id.to_string());
    metadata.insert(
        "repository_full_name".into(),
        revision.repository_full_name.clone(),
    );
    metadata.insert("commit_sha".into(), revision.commit_sha.clone());

    finalize_observation(
        &forge_descriptor(),
        context,
        &NormalizedProviderResult {
            // Passed asserts only that this exact commit exists at this
            // repository. It asserts nothing about review, safety, merge state,
            // or release readiness.
            status: EvidenceStatus::Passed,
            source_trust: None,
            result_metadata: metadata,
            artifacts: Vec::new(),
        },
    )
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

fn job_conclusion_summary(jobs: &[JobSummary]) -> String {
    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    for job in jobs {
        if let Some(conclusion) = job.conclusion {
            *counts.entry(conclusion_label(conclusion)).or_insert(0) += 1;
        }
    }
    if counts.is_empty() {
        return "none".to_string();
    }
    counts
        .iter()
        .map(|(conclusion, count)| format!("{conclusion}:{count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn run_label(value: RunStatus) -> &'static str {
    match value {
        RunStatus::Queued => "queued",
        RunStatus::InProgress => "in_progress",
        RunStatus::Completed => "completed",
        RunStatus::Waiting => "waiting",
        RunStatus::Requested => "requested",
        RunStatus::Pending => "pending",
    }
}

fn conclusion_label(value: RunConclusion) -> &'static str {
    match value {
        RunConclusion::Success => "success",
        RunConclusion::Failure => "failure",
        RunConclusion::Neutral => "neutral",
        RunConclusion::Cancelled => "cancelled",
        RunConclusion::Skipped => "skipped",
        RunConclusion::TimedOut => "timed_out",
        RunConclusion::ActionRequired => "action_required",
        RunConclusion::Stale => "stale",
        RunConclusion::StartupFailure => "startup_failure",
    }
}

fn check_label(value: CheckStatus) -> &'static str {
    match value {
        CheckStatus::Queued => "queued",
        CheckStatus::InProgress => "in_progress",
        CheckStatus::Completed => "completed",
        CheckStatus::Waiting => "waiting",
        CheckStatus::Requested => "requested",
        CheckStatus::Pending => "pending",
    }
}

fn check_conclusion_label(value: CheckConclusion) -> &'static str {
    match value {
        CheckConclusion::Success => "success",
        CheckConclusion::Failure => "failure",
        CheckConclusion::Neutral => "neutral",
        CheckConclusion::Cancelled => "cancelled",
        CheckConclusion::Skipped => "skipped",
        CheckConclusion::TimedOut => "timed_out",
        CheckConclusion::ActionRequired => "action_required",
    }
}

fn commit_status_label(value: CommitStatusState) -> &'static str {
    match value {
        CommitStatusState::Pending => "pending",
        CommitStatusState::Success => "success",
        CommitStatusState::Failure => "failure",
        CommitStatusState::Error => "error",
    }
}
