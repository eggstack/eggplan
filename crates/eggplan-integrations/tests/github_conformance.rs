//! GitHub forge and Actions evidence conformance.
//!
//! These assertions are about authority, not shape. The recurring theme is that
//! a forge fact must never become an authority the forge did not assert:
//! neutral is not success, a merge commit is not a source commit, an expired
//! artifact is not an available one, and a digest is not a verification.

use eggplan_core::{
    EvidenceKind, EvidenceObservationId, EvidenceStatus, ProviderRegistry, SubjectRevision,
    SubjectState, VerificationDigest,
};
use eggplan_integrations::github::{self, MAX_DTO_BYTES, REVIEWED_GITHUB_API_VERSION};
use eggplan_integrations::{ObservationContext, SpiError};
use serde_json::Value;
use std::collections::BTreeMap;

const HEAD_SHA: &str = "674264db7dda3f063cbbedd93f6f4938c1959a11";
const PR_MERGE_SHA: &str = "1111111111111111111111111111111111111111";
const DIGEST_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn context(kind: EvidenceKind, revision: &str) -> ObservationContext {
    ObservationContext {
        observation_id: EvidenceObservationId::new("epe_github_test").unwrap(),
        subject: SubjectRevision {
            subject_kind: "git".into(),
            repository_id: "epr_github_fixture".into(),
            revision: revision.into(),
            state: SubjectState::Clean,
            dirty_digest: None,
        },
        requested_kind: kind,
        observed_at_unix_ms: 1_788_000_100_000,
        invocation_ref: Some("github:actions-runs/37350383159".into()),
        verification_digest: Some(VerificationDigest::new(format!("sha256:{DIGEST_HEX}")).unwrap()),
        metadata: BTreeMap::new(),
    }
}

/// Forge provenance observations deliberately carry no verification binding, so
/// the context must not supply one either.
fn forge_context(kind: EvidenceKind, revision: &str) -> ObservationContext {
    unbound(kind, revision)
}

fn unbound(kind: EvidenceKind, revision: &str) -> ObservationContext {
    let mut value = context(kind, revision);
    value.verification_digest = None;
    value
}

#[allow(dead_code)]
fn unbound_at_head(kind: EvidenceKind) -> ObservationContext {
    let mut value = context(kind, HEAD_SHA);
    value.verification_digest = None;
    value
}

fn load_cases(bytes: &'static [u8]) -> Vec<(&'static str, Value)> {
    let mut raw: Vec<Value> = serde_json::from_slice(bytes).unwrap();
    raw.drain(..)
        .map(|mut case| {
            let name = case["case"].as_str().unwrap().to_string();
            case.as_object_mut().unwrap().remove("case");
            let name: &'static str = Box::leak(name.into_boxed_str());
            (name, case)
        })
        .collect()
}

fn run_cases() -> Vec<(&'static str, Value)> {
    load_cases(include_bytes!("fixtures/github-runs.json"))
}
fn check_cases() -> Vec<(&'static str, Value)> {
    load_cases(include_bytes!("fixtures/github-check-runs.json"))
}
fn commit_status_cases() -> Vec<(&'static str, Value)> {
    load_cases(include_bytes!("fixtures/github-commit-statuses.json"))
}
fn artifact_cases() -> Vec<(&'static str, Value)> {
    load_cases(include_bytes!("fixtures/github-artifacts.json"))
}
fn revision_cases() -> Vec<(&'static str, Value)> {
    load_cases(include_bytes!("fixtures/github-revisions.json"))
}

fn pick<'a>(cases: &'a [(&'static str, Value)], name: &str) -> &'a Value {
    &cases
        .iter()
        .find(|(case, _)| *case == name)
        .unwrap_or_else(|| panic!("missing fixture {name}"))
        .1
}

fn parse_run(case: &Value) -> github::GitHubActionsRunV1 {
    github::parse_run(&serde_json::to_vec(case).unwrap()).unwrap()
}
fn parse_check(case: &Value) -> github::GitHubCheckRunV1 {
    github::parse_check_run(&serde_json::to_vec(case).unwrap()).unwrap()
}
fn parse_commit_status(case: &Value) -> github::GitHubCommitStatusV1 {
    github::parse_commit_status(&serde_json::to_vec(case).unwrap()).unwrap()
}
fn parse_artifact(case: &Value) -> github::GitHubArtifactV1 {
    github::parse_artifact(&serde_json::to_vec(case).unwrap()).unwrap()
}
fn parse_revision(case: &Value) -> github::GitHubRevisionV1 {
    github::parse_revision(&serde_json::to_vec(case).unwrap()).unwrap()
}

fn mutate(case: &Value, key: &str, value: Value) -> Value {
    let mut cloned = case.clone();
    cloned[key] = value;
    cloned
}

// ---------------------------------------------------------------------------
// Provenance of the frozen contract
// ---------------------------------------------------------------------------

#[test]
fn fixtures_are_pinned_to_the_reviewed_api_version() {
    assert_eq!(REVIEWED_GITHUB_API_VERSION, "2022-11-28");
    let cases = run_cases();
    let base = pick(&cases, "successful_workflow");
    // The fixture set reproduces a real run from this repository.
    assert_eq!(
        base["repository_full_name"].as_str().unwrap(),
        "eggstack/eggplan"
    );
    assert_eq!(
        base["workflow_path"].as_str().unwrap(),
        ".github/workflows/ci.yml"
    );
    assert_eq!(base["run_attempt"].as_u64().unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Execution outcome mapping
// ---------------------------------------------------------------------------

#[test]
fn workflow_run_conclusions_map_without_inventing_authority() {
    let cases = run_cases();
    for (name, expected) in [
        ("successful_workflow", EvidenceStatus::Passed),
        ("failed_workflow", EvidenceStatus::Failed),
        ("in_progress_workflow", EvidenceStatus::InProgress),
        ("cancelled_workflow", EvidenceStatus::Skipped),
        ("neutral_workflow", EvidenceStatus::Inconclusive),
        ("action_required_workflow", EvidenceStatus::Blocked),
        ("timed_out_workflow", EvidenceStatus::Failed),
        ("skipped_workflow", EvidenceStatus::Skipped),
        ("stale_workflow", EvidenceStatus::Inconclusive),
        ("startup_failure_workflow", EvidenceStatus::Failed),
        ("queued_workflow", EvidenceStatus::InProgress),
        ("successful_rerun", EvidenceStatus::Passed),
    ] {
        let run = parse_run(pick(&cases, name));
        let observed = github::normalize_run(&run, &context(EvidenceKind::Test, HEAD_SHA))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(observed.status(), expected, "status for {name}");
    }
}

#[test]
fn neutral_and_stale_are_never_success() {
    let cases = run_cases();
    for name in ["neutral_workflow", "stale_workflow"] {
        let run = parse_run(pick(&cases, name));
        assert_eq!(
            run.conclusion,
            Some(if name == "neutral_workflow" {
                github::RunConclusion::Neutral
            } else {
                github::RunConclusion::Stale
            })
        );
        let observed = github::normalize_run(&run, &context(EvidenceKind::Test, HEAD_SHA)).unwrap();
        assert_ne!(observed.status(), EvidenceStatus::Passed);
    }
}

#[test]
fn run_attempt_is_part_of_execution_identity() {
    let cases = run_cases();
    let first = parse_run(pick(&cases, "successful_workflow"));
    let rerun = parse_run(pick(&cases, "successful_rerun"));
    // Same run_id, different attempt: distinct executions, never collapsed.
    assert_eq!(first.run_id, rerun.run_id);
    assert_eq!(first.run_attempt, 1);
    assert_eq!(rerun.run_attempt, 2);
    let observed = github::normalize_run(&rerun, &context(EvidenceKind::Test, HEAD_SHA)).unwrap();
    assert_eq!(
        observed
            .result_metadata()
            .get("run_attempt")
            .map(String::as_str),
        Some("2")
    );
    // Job conclusions stay countable metadata and never override the run verdict.
    assert_eq!(
        observed
            .result_metadata()
            .get("job_conclusions")
            .map(String::as_str),
        Some("skipped:1,success:1")
    );
}

#[test]
fn status_and_conclusion_must_agree_about_terminality() {
    let cases = run_cases();
    let base = pick(&cases, "successful_workflow");
    // Terminal status without a conclusion is a contradiction.
    let missing = mutate(base, "conclusion", Value::Null);
    assert!(github::parse_run(&serde_json::to_vec(&missing).unwrap()).is_err());
    // Non-terminal status with a conclusion is a contradiction.
    let running = pick(&cases, "in_progress_workflow").clone();
    let contradictory = mutate(&running, "conclusion", "success".into());
    assert!(github::parse_run(&serde_json::to_vec(&contradictory).unwrap()).is_err());
    // And the same rule applies per job.
    let job_contradiction = mutate(
        base,
        "jobs",
        serde_json::json!([{
            "job_id": 1, "name": "build", "head_sha": null,
            "status": "completed", "conclusion": null, "step_conclusions": {}
        }]),
    );
    assert!(github::parse_run(&serde_json::to_vec(&job_contradiction).unwrap()).is_err());
}

#[test]
fn check_run_conclusions_map_without_inventing_authority() {
    let cases = check_cases();
    for (name, expected) in [
        ("check_success", EvidenceStatus::Passed),
        ("check_failure", EvidenceStatus::Failed),
        ("check_in_progress", EvidenceStatus::InProgress),
        ("check_cancelled", EvidenceStatus::Skipped),
        ("check_neutral", EvidenceStatus::Inconclusive),
        ("check_action_required", EvidenceStatus::Blocked),
    ] {
        let check = parse_check(pick(&cases, name));
        let observed =
            github::normalize_check_run(&check, &context(EvidenceKind::Command, HEAD_SHA))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(observed.status(), expected, "status for {name}");
    }
}

#[test]
fn classic_commit_status_aggregates_conservatively() {
    let cases = commit_status_cases();
    for (name, expected) in [
        ("classic_all_success", EvidenceStatus::Passed),
        ("classic_with_failure", EvidenceStatus::Failed),
        ("classic_with_error", EvidenceStatus::Failed),
        ("classic_with_pending", EvidenceStatus::InProgress),
        ("classic_empty", EvidenceStatus::Inconclusive),
    ] {
        let status = parse_commit_status(pick(&cases, name));
        let observed =
            github::normalize_commit_status(&status, &context(EvidenceKind::Test, HEAD_SHA))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(observed.status(), expected, "status for {name}");
    }
    // A single success must not pass a commit that also has a pending status.
    let cases = commit_status_cases();
    let mixed = parse_commit_status(pick(&cases, "classic_with_pending"));
    assert!(
        mixed
            .statuses
            .iter()
            .any(|entry| { github::map_commit_status(entry.state) == EvidenceStatus::Passed })
    );
    assert_eq!(
        github::normalize_commit_status(&mixed, &context(EvidenceKind::Test, HEAD_SHA))
            .unwrap()
            .status(),
        EvidenceStatus::InProgress
    );
}

// ---------------------------------------------------------------------------
// Revision exactness
// ---------------------------------------------------------------------------

#[test]
fn ci_evidence_for_a_different_commit_is_rejected() {
    let cases = run_cases();
    // A run whose tested SHA is not the Eggplan subject SHA cannot be evidence
    // about that subject. This is also what keeps a PR merge-commit run from
    // standing in for a source commit.
    let pr_run = parse_run(pick(&cases, "pr_merge_sha_workflow"));
    assert_eq!(
        github::normalize_run(&pr_run, &context(EvidenceKind::Test, HEAD_SHA)),
        Err(SpiError::Invalid(
            "GitHub tested commit does not match the Eggplan Git subject"
        ))
    );
    let checks = check_cases();
    let pr_check = parse_check(pick(&checks, "check_pr_merge_sha"));
    assert!(
        github::normalize_check_run(&pr_check, &context(EvidenceKind::Command, HEAD_SHA)).is_err()
    );
    let commits = commit_status_cases();
    let wrong_sha = mutate(
        pick(&commits, "classic_all_success"),
        "commit_sha",
        PR_MERGE_SHA.into(),
    );
    let wrong_sha = github::parse_commit_status(&serde_json::to_vec(&wrong_sha).unwrap()).unwrap();
    assert!(
        github::normalize_commit_status(&wrong_sha, &context(EvidenceKind::Test, HEAD_SHA))
            .is_err()
    );

    // A dirty subject can never carry a clean GitHub run as evidence: forge CI
    // checks out a commit and can never observe uncommitted bytes.
    let cases = run_cases();
    let clean = parse_run(pick(&cases, "successful_workflow"));
    let mut dirty = context(EvidenceKind::Test, HEAD_SHA);
    dirty.subject.state = SubjectState::Dirty;
    dirty.subject.dirty_digest = Some(format!("sha256:{DIGEST_HEX}"));
    assert_eq!(
        github::normalize_run(&clean, &dirty),
        Err(SpiError::Invalid(
            "GitHub CI evidence cannot describe a dirty Eggplan Git subject"
        ))
    );

    // A non-Git subject is not compared against a commit SHA at all.
    let mut non_git = context(EvidenceKind::Test, HEAD_SHA);
    non_git.subject.subject_kind = "path".into();
    non_git.subject.repository_id = "epr_path_fixture".into();
    assert!(github::normalize_run(&clean, &non_git).is_ok());
}

#[test]
fn a_job_cannot_disagree_with_its_run_about_the_tested_commit() {
    let cases = run_cases();
    let mismatched = pick(&cases, "job_head_sha_disagreement").clone();
    assert!(github::parse_run(&serde_json::to_vec(&mismatched).unwrap()).is_err());
}

// ---------------------------------------------------------------------------
// Artifact availability is independent of execution outcome
// ---------------------------------------------------------------------------

#[test]
fn artifact_availability_is_a_separate_claim_from_execution() {
    let cases = artifact_cases();
    let available = parse_artifact(pick(&cases, "artifact_with_digest"));
    let observed =
        github::normalize_artifact(&available, &forge_context(EvidenceKind::Artifact, HEAD_SHA))
            .unwrap();
    assert_eq!(observed.status(), EvidenceStatus::Passed);
    let refs = observed.artifacts();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].reference, "github-actions:artifact:1382494389:5");
    assert_eq!(
        refs[0].digest.as_deref(),
        Some("sha256:cfc3236bdad15b5898bca8408945c9e19e1917da8704adc20eaa618444290a8c")
    );

    // An expired artifact is unavailable, not failed, and not passed.
    let expired = parse_artifact(pick(&cases, "artifact_expired"));
    let observed =
        github::normalize_artifact(&expired, &forge_context(EvidenceKind::Artifact, HEAD_SHA))
            .unwrap();
    assert_eq!(observed.status(), EvidenceStatus::Unavailable);
    assert_eq!(
        observed
            .result_metadata()
            .get("expired")
            .map(String::as_str),
        Some("true")
    );

    // A failed run does not make its artifact unavailable, and an expired
    // artifact does not retroactively fail the run that produced it.
    let runs = run_cases();
    let failed = parse_run(pick(&runs, "failed_workflow"));
    assert_eq!(
        github::normalize_run(&failed, &context(EvidenceKind::Test, HEAD_SHA))
            .unwrap()
            .status(),
        EvidenceStatus::Failed
    );
    assert_eq!(
        github::normalize_artifact(&available, &forge_context(EvidenceKind::Artifact, HEAD_SHA))
            .unwrap()
            .status(),
        EvidenceStatus::Passed
    );
}

#[test]
fn an_absent_artifact_digest_is_recorded_as_absent_never_inferred() {
    let cases = artifact_cases();
    let legacy = parse_artifact(pick(&cases, "artifact_without_digest"));
    assert!(legacy.digest.is_none());
    let observed =
        github::normalize_artifact(&legacy, &forge_context(EvidenceKind::Artifact, HEAD_SHA))
            .unwrap();
    assert_eq!(
        observed
            .result_metadata()
            .get("digest_present")
            .map(String::as_str),
        Some("false")
    );
    // No digest means no invented digest on the reference either.
    assert!(observed.artifacts()[0].digest.is_none());

    // A malformed digest is rejected, not tolerated.
    for bad in [
        "",
        "abc",
        "sha256:",
        "sha512:abc",
        &format!("sha256:{}", "A".repeat(64)),
    ] {
        let broken = mutate(
            pick(&cases, "artifact_with_digest"),
            "digest",
            Value::String(bad.into()),
        );
        assert!(
            github::parse_artifact(&serde_json::to_vec(&broken).unwrap()).is_err(),
            "digest {bad:?} must be rejected"
        );
    }
}

#[test]
fn an_artifact_for_a_different_commit_is_rejected() {
    let cases = artifact_cases();
    let pr_artifact = parse_artifact(pick(&cases, "artifact_pr_merge_sha"));
    assert!(
        github::normalize_artifact(
            &pr_artifact,
            &forge_context(EvidenceKind::Artifact, HEAD_SHA)
        )
        .is_err()
    );
}

// ---------------------------------------------------------------------------
// Revision provenance
// ---------------------------------------------------------------------------

#[test]
fn revision_provenance_is_narrow_and_not_a_verdict() {
    let cases = revision_cases();
    let revision = parse_revision(pick(&cases, "exact_commit"));
    let observed =
        github::normalize_revision(&revision, &forge_context(EvidenceKind::Revision, HEAD_SHA))
            .unwrap();
    // Passed means only "this exact commit exists at this repository".
    assert_eq!(observed.status(), EvidenceStatus::Passed);
    let metadata = observed.result_metadata();
    assert_eq!(
        metadata.get("commit_sha").map(String::as_str),
        Some(HEAD_SHA)
    );
    assert_eq!(
        metadata.get("repository_full_name").map(String::as_str),
        Some("eggstack/eggplan")
    );
    // The serialized claim carries no review, merge, safety, or tag assertion.
    let serialized = serde_json::to_string(&observed).unwrap();
    for forbidden in [
        "reviewed",
        "merged",
        "safe",
        "release",
        "protected",
        "attestation",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "{forbidden} must not appear"
        );
    }
}

#[test]
fn revision_identity_rejects_mutable_branch_or_tag_shapes() {
    let cases = revision_cases();
    let base = pick(&cases, "exact_commit");
    for bad in [
        "main",
        "v1.0.0",
        "refs/heads/main",
        &"a".repeat(39),
        &"a".repeat(41),
    ] {
        let broken = mutate(base, "commit_sha", Value::String(bad.into()));
        assert!(
            github::parse_revision(&serde_json::to_vec(&broken).unwrap()).is_err(),
            "commit {bad:?} must be rejected"
        );
    }
}

// ---------------------------------------------------------------------------
// Provider identity, verification binding, trust
// ---------------------------------------------------------------------------

#[test]
fn provider_identities_and_allowed_kinds_are_fixed() {
    let actions = github::actions_descriptor();
    assert_eq!(actions.provider_id.as_str(), "epp_github_actions");
    let core = actions.provider_descriptor().unwrap();
    assert_eq!(core.class(), "execution");
    for kind in [
        EvidenceKind::Test,
        EvidenceKind::Command,
        EvidenceKind::DelegatedRun,
        EvidenceKind::Benchmark,
    ] {
        assert!(
            core.allowed_kinds().contains(&kind),
            "{kind:?} must be allowed"
        );
    }
    assert_eq!(actions.allowed_kinds.len(), 4);
    // Forge kinds are never produced by the execution provider.
    for kind in [
        EvidenceKind::Revision,
        EvidenceKind::Artifact,
        EvidenceKind::Attestation,
    ] {
        assert!(!core.allowed_kinds().contains(&kind));
    }

    let forge = github::forge_descriptor();
    assert_eq!(forge.provider_id.as_str(), "epp_github_forge");
    let core = forge.provider_descriptor().unwrap();
    assert_eq!(core.class(), "utility");
    assert!(core.allowed_kinds().contains(&EvidenceKind::Revision));
    assert!(core.allowed_kinds().contains(&EvidenceKind::Artifact));
    assert_eq!(forge.allowed_kinds.len(), 2);
    // Attestation verification remains deferred to Interoperability M001.
    assert!(!core.allowed_kinds().contains(&EvidenceKind::Attestation));

    // The forge provider cannot carry verification binding at all, so an
    // artifact digest can never be replayed as a verification digest.
    assert!(!forge.capabilities.supports_in_progress);
    let runs = run_cases();
    let success = parse_run(pick(&runs, "successful_workflow"));
    assert!(success.head_sha.len() == 40);
    assert!(
        github::normalize_artifact(
            &parse_artifact(pick(
                &load_cases(include_bytes!("fixtures/github-artifacts.json")),
                "artifact_with_digest"
            )),
            &forge_context(EvidenceKind::Artifact, HEAD_SHA)
        )
        .unwrap()
        .verification_digest()
        .is_none()
    );
}

#[test]
fn execution_evidence_requires_host_verification_binding() {
    let cases = run_cases();
    let run = parse_run(pick(&cases, "successful_workflow"));
    assert_eq!(
        github::normalize_run(&run, &unbound(EvidenceKind::Test, HEAD_SHA)),
        Err(SpiError::MissingVerificationBinding)
    );
    let checks = check_cases();
    let check = parse_check(pick(&checks, "check_success"));
    assert_eq!(
        github::normalize_check_run(&check, &unbound(EvidenceKind::Command, HEAD_SHA)),
        Err(SpiError::MissingVerificationBinding)
    );
    let observed = github::normalize_run(&run, &context(EvidenceKind::Test, HEAD_SHA)).unwrap();
    assert_eq!(
        observed.verification_digest().map(|value| value.as_str()),
        Some(format!("sha256:{DIGEST_HEX}").as_str())
    );
}

#[test]
fn forge_provenance_rejects_a_verification_binding_instead_of_ignoring_it() {
    let cases = artifact_cases();
    let artifact = parse_artifact(pick(&cases, "artifact_with_digest"));
    let error = github::normalize_artifact(&artifact, &context(EvidenceKind::Artifact, HEAD_SHA))
        .unwrap_err();
    assert_eq!(error, SpiError::VerificationBindingNotSupported);
}

#[test]
fn unsupported_kinds_are_rejected_in_both_directions() {
    let cases = run_cases();
    let run = parse_run(pick(&cases, "successful_workflow"));
    for kind in [
        EvidenceKind::Artifact,
        EvidenceKind::Revision,
        EvidenceKind::Attestation,
        EvidenceKind::Research,
        EvidenceKind::HumanJudgment,
        EvidenceKind::StaticAnalysis,
    ] {
        assert_eq!(
            github::normalize_run(&run, &context(kind, HEAD_SHA)),
            Err(SpiError::UnsupportedKind),
            "{kind:?} must not be produced by the execution adapter"
        );
    }
    let artifacts = artifact_cases();
    let artifact = parse_artifact(pick(&artifacts, "artifact_with_digest"));
    assert_eq!(
        github::normalize_artifact(&artifact, &context(EvidenceKind::Test, HEAD_SHA)),
        Err(SpiError::UnsupportedKind)
    );
    let revisions = revision_cases();
    let revision = parse_revision(pick(&revisions, "exact_commit"));
    assert_eq!(
        github::normalize_revision(&revision, &forge_context(EvidenceKind::Artifact, HEAD_SHA)),
        Err(SpiError::UnsupportedKind)
    );
}

#[test]
fn the_adapter_never_enrolls_trust_or_acquires_evidence() {
    let source = include_str!("../src/github.rs");
    for forbidden in [
        "register_trusted",
        "ProviderRegistry",
        "std::process",
        "Command::new",
        "std::fs",
        "std::net",
        "reqwest",
        "tokio",
        "octocrab",
        "http://",
        "https://",
        "api.github.com",
        "archive_download_url",
        "Authorization",
        "Bearer",
    ] {
        assert!(
            !source.contains(forbidden),
            "GitHub adapter must not contain {forbidden}"
        );
    }
    // A default registry does not know either provider until a host enrolls it.
    let registry = ProviderRegistry::default();
    assert!(
        registry
            .get(&github::actions_descriptor().provider_id)
            .is_none()
    );
    assert!(
        registry
            .get(&github::forge_descriptor().provider_id)
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// Fail-closed schema and identity handling
// ---------------------------------------------------------------------------

#[test]
fn unknown_schemas_versions_and_fields_fail_closed() {
    let cases = run_cases();
    let base = pick(&cases, "successful_workflow");
    assert!(
        github::parse_run(&serde_json::to_vec(&mutate(base, "schema_version", 2.into())).unwrap())
            .is_err()
    );
    assert!(
        github::parse_run(
            &serde_json::to_vec(&mutate(base, "conclusion", "exploded".into())).unwrap()
        )
        .is_err()
    );
    assert!(
        github::parse_run(
            &serde_json::to_vec(&mutate(base, "status", "sorta_running".into())).unwrap()
        )
        .is_err()
    );
    let extra = mutate(base, "logs_url", Value::String("https://x/y".into()));
    assert!(github::parse_run(&serde_json::to_vec(&extra).unwrap()).is_err());

    let checks = check_cases();
    assert!(
        github::parse_check_run(
            &serde_json::to_vec(&mutate(
                pick(&checks, "check_success"),
                "schema_version",
                2.into()
            ))
            .unwrap()
        )
        .is_err()
    );
    let statuses = commit_status_cases();
    assert!(
        github::parse_commit_status(
            &serde_json::to_vec(&mutate(
                pick(&statuses, "classic_all_success"),
                "schema_version",
                2.into()
            ))
            .unwrap()
        )
        .is_err()
    );
    let artifacts = artifact_cases();
    assert!(
        github::parse_artifact(
            &serde_json::to_vec(&mutate(
                pick(&artifacts, "artifact_with_digest"),
                "schema_version",
                2.into()
            ))
            .unwrap()
        )
        .is_err()
    );
    let revisions = revision_cases();
    assert!(
        github::parse_revision(
            &serde_json::to_vec(&mutate(
                pick(&revisions, "exact_commit"),
                "schema_version",
                2.into()
            ))
            .unwrap()
        )
        .is_err()
    );

    let oversize = vec![b' '; MAX_DTO_BYTES + 1];
    assert!(github::parse_run(&oversize).is_err());
    assert!(github::parse_check_run(&oversize).is_err());
    assert!(github::parse_commit_status(&oversize).is_err());
    assert!(github::parse_artifact(&oversize).is_err());
    assert!(github::parse_revision(&oversize).is_err());
}

#[test]
fn identity_fields_are_validated() {
    let cases = run_cases();
    let base = pick(&cases, "successful_workflow");
    for key in ["repository_id", "workflow_id", "run_id", "run_number"] {
        assert!(
            github::parse_run(&serde_json::to_vec(&mutate(base, key, Value::from(0))).unwrap())
                .is_err(),
            "{key} must be positive"
        );
    }
    assert!(
        github::parse_run(
            &serde_json::to_vec(&mutate(base, "run_attempt", Value::from(0))).unwrap()
        )
        .is_err()
    );
    assert!(
        github::parse_run(
            &serde_json::to_vec(&mutate(base, "repository_full_name", "eggstack".into())).unwrap()
        )
        .is_err()
    );
    assert!(
        github::parse_run(
            &serde_json::to_vec(&mutate(base, "repository_full_name", "eggstack/".into())).unwrap()
        )
        .is_err()
    );
    assert!(
        github::parse_run(&serde_json::to_vec(&mutate(base, "head_sha", "abc123".into())).unwrap())
            .is_err()
    );
    assert!(
        github::parse_run(
            &serde_json::to_vec(&mutate(base, "head_sha", HEAD_SHA.to_uppercase().into())).unwrap()
        )
        .is_err()
    );
    assert!(
        github::parse_run(
            &serde_json::to_vec(&mutate(base, "completed_at_unix_ms", Value::from(1))).unwrap()
        )
        .is_err()
    );
}

#[test]
fn workflow_paths_and_navigation_handles_are_confined() {
    let cases = run_cases();
    let base = pick(&cases, "successful_workflow");
    for bad in [
        "/.github/workflows/ci.yml",
        ".github/../../etc/ci.yml",
        ".github//ci.yml",
        ".github\\ci.yml",
    ] {
        assert!(
            github::parse_run(
                &serde_json::to_vec(&mutate(base, "workflow_path", bad.into())).unwrap()
            )
            .is_err(),
            "workflow path {bad:?} must be rejected"
        );
    }
    for bad in [
        "https://github.com/eggstack/eggplan/actions/runs/1",
        "eggstack/eggplan?token=abc",
        "user@host/path",
        "run#fragment",
        "has space",
    ] {
        assert!(
            github::parse_run(
                &serde_json::to_vec(&mutate(base, "run_handle", bad.into())).unwrap()
            )
            .is_err(),
            "navigation handle {bad:?} must be rejected"
        );
    }
}

#[test]
fn duplicate_identities_are_rejected() {
    let cases = run_cases();
    let job = serde_json::json!({
        "job_id": 7, "name": "build", "head_sha": null,
        "status": "completed", "conclusion": "success", "step_conclusions": {}
    });
    let base = pick(&cases, "successful_workflow");
    let duplicated = mutate(base, "jobs", serde_json::json!([job.clone(), job]));
    assert!(github::parse_run(&serde_json::to_vec(&duplicated).unwrap()).is_err());

    let statuses = commit_status_cases();
    let entry = serde_json::json!({"status_id": 1, "context": "ci/x", "state": "success"});
    let dup_ids = mutate(
        pick(&statuses, "classic_all_success"),
        "statuses",
        serde_json::json!([entry.clone(), entry]),
    );
    assert!(github::parse_commit_status(&serde_json::to_vec(&dup_ids).unwrap()).is_err());
    let dup_contexts = mutate(
        pick(&statuses, "classic_all_success"),
        "statuses",
        serde_json::json!([
            {"status_id": 1, "context": "ci/x", "state": "success"},
            {"status_id": 2, "context": "ci/x", "state": "failure"}
        ]),
    );
    assert!(github::parse_commit_status(&serde_json::to_vec(&dup_contexts).unwrap()).is_err());
}

#[test]
fn a_github_attestation_payload_is_rejected_not_accepted_as_provenance() {
    // A Sigstore/in-toto shaped payload is a plausible thing for a host to hand
    // this adapter. Every parser must refuse it rather than silently recording
    // an unverified provenance claim.
    let attestation = serde_json::json!({
        "schema_version": 1,
        "mediaType": "application/vnd.in-toto+json",
        "predicateType": "https://slsa.dev/provenance/v1",
        "bundle": {
            "mediaType": "application/vnd.dev.sigstore.bundle.v0.3+json",
            "dsseEnvelope": {
                "payload": "BASE64",
                "payloadType": "application/vnd.in-toto+json",
                "signatures": [{"sig": "BASE64", "keyid": ""}]
            }
        }
    });
    let bytes = serde_json::to_vec(&attestation).unwrap();
    assert!(github::parse_artifact(&bytes).is_err());
    assert!(github::parse_revision(&bytes).is_err());
    assert!(github::parse_run(&bytes).is_err());
    assert!(github::parse_check_run(&bytes).is_err());
    assert!(github::parse_commit_status(&bytes).is_err());
}

#[test]
fn arbitrary_native_prose_is_absent_from_serialized_observations() {
    let cases = run_cases();
    let run = parse_run(pick(&cases, "successful_rerun"));
    let observed = github::normalize_run(&run, &context(EvidenceKind::Test, HEAD_SHA)).unwrap();
    let serialized = serde_json::to_string(&observed).unwrap();
    for forbidden in [
        "://",
        "logs",
        "annotation",
        "step:",
        "runner_name",
        "Set up job",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "{forbidden} must not be serialized"
        );
    }
    // Step detail survives only as bounded per-conclusion counts.
    let stepped = mutate(
        pick(&cases, "successful_rerun"),
        "jobs",
        serde_json::json!([{
            "job_id": 1, "name": "native", "head_sha": null,
            "status": "completed", "conclusion": "failure",
            "step_conclusions": {"success": 6, "failure": 1}
        }]),
    );
    let run = parse_run(&stepped);
    let observed = github::normalize_run(&run, &context(EvidenceKind::Test, HEAD_SHA)).unwrap();
    assert_eq!(
        observed
            .result_metadata()
            .get("job_conclusions")
            .map(String::as_str),
        Some("failure:1")
    );
}
