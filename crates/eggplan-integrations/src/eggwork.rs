//! Bounded DTO adapter for Eggwork execution snapshots.
//!
//! The host submits serialized native records. This module never calls
//! Eggwork, executes commands, or persists captured output.

use crate::{
    AdapterDescriptor, Capabilities, NormalizedProviderResult, ObservationContext, ProviderClass,
    SpiError, finalize_observation,
};
use eggplan_core::{
    ArtifactRef, EvidenceKind, EvidenceObservation, EvidenceProviderId, EvidenceStatus,
};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const REVIEWED_EGGWORK_SHA: &str = "faaa0b905fa6bc43e46825fdd98530b5533a970f";
pub const MAX_SNAPSHOT_BYTES: usize = 1_048_576;
pub const MAX_ARTIFACTS: usize = 64;
const CAPS: Capabilities = Capabilities {
    supports_in_progress: true,
    artifacts: true,
    verification_binding: true,
    research_trust_metadata: false,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ExecutionState {
    Accepted,
    Preparing,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Interrupted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ExecutionFailure {
    Spawn,
    Internal,
    OutputLimit,
    Sandbox,
    ResourceLimit,
    Interrupted,
    LeaseExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum FinalizationFailure {
    ArtifactCapture,
    Retention,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SandboxResult {
    NotRequested,
    NotApplied { reason: String },
    Applied { profile: String },
    Failed { reason: String },
}
#[derive(Debug, Deserialize)]
pub enum ResourceDimensionResult {
    NotRequested,
    NotApplied { reason: String },
    Applied { backend: String },
    LimitExceeded { backend: String },
}
#[derive(Debug, Deserialize)]
pub struct ResourceResult {
    pub memory_bytes: ResourceDimensionResult,
    pub cpu_millis: ResourceDimensionResult,
    pub pids: ResourceDimensionResult,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSnapshot {
    pub schema_version: u16,
    pub execution_id: String,
    pub generation: u64,
    pub state: ExecutionState,
    pub result: Option<ExecutionResult>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResult {
    pub state: ExecutionState,
    pub exit_code: Option<i32>,
    pub failure: Option<ExecutionFailure>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_omitted: u64,
    pub stderr_omitted: u64,
    pub cleanup_warning: Option<String>,
    #[serde(default)]
    pub finalization_failure: Option<FinalizationFailure>,
    #[serde(default)]
    pub artifact_count: u32,
    #[serde(default)]
    pub sandbox: Option<SandboxResult>,
    #[serde(default)]
    pub resources: Option<ResourceResult>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub execution_id: String,
    pub generation: u64,
    pub path: String,
    pub kind: String,
    pub digest: String,
    pub size_bytes: u64,
    pub executable: bool,
    pub created_unix_ms: u64,
    pub expires_unix_ms: u64,
}

pub fn descriptor() -> AdapterDescriptor {
    AdapterDescriptor::new(
        EvidenceProviderId::new("epp_eggwork").expect("static id"),
        ProviderClass::Execution,
        [
            EvidenceKind::Command,
            EvidenceKind::Test,
            EvidenceKind::DelegatedRun,
            EvidenceKind::Benchmark,
        ],
        "1.0",
        CAPS,
    )
    .expect("static descriptor")
}

pub fn parse_snapshot(bytes: &[u8]) -> Result<ExecutionSnapshot, SpiError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SpiError::Invalid("Eggwork snapshot exceeds byte limit"));
    }
    let value: ExecutionSnapshot = serde_json::from_slice(bytes)
        .map_err(|_| SpiError::Invalid("invalid Eggwork snapshot JSON or contract"))?;
    validate_snapshot(&value)?;
    Ok(value)
}
pub fn parse_artifacts(bytes: &[u8]) -> Result<Vec<ArtifactRecord>, SpiError> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SpiError::Invalid("Eggwork artifacts exceed byte limit"));
    }
    let artifacts: Vec<ArtifactRecord> = serde_json::from_slice(bytes)
        .map_err(|_| SpiError::Invalid("invalid Eggwork artifact JSON or contract"))?;
    if artifacts.len() > MAX_ARTIFACTS {
        return Err(SpiError::Invalid("Eggwork artifact count exceeds limit"));
    }
    Ok(artifacts)
}

pub fn normalize(
    snapshot: &ExecutionSnapshot,
    artifacts: &[ArtifactRecord],
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate_snapshot(snapshot)?;
    if snapshot.generation == 0
        || snapshot.execution_id.is_empty()
        || snapshot.execution_id.len() > 128
        || !snapshot
            .execution_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
        || artifacts.len() > MAX_ARTIFACTS
    {
        return Err(SpiError::Invalid(
            "invalid Eggwork identity or artifact count",
        ));
    }
    if context.verification_digest.is_none() {
        return Err(SpiError::MissingVerificationBinding);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut refs = Vec::with_capacity(artifacts.len());
    for a in artifacts {
        if a.execution_id != snapshot.execution_id
            || a.generation != snapshot.generation
            || !seen.insert(a.artifact_id.as_str())
            || a.artifact_id.is_empty()
            || a.artifact_id.len() > 128
            || !a
                .artifact_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
            || a.expires_unix_ms < a.created_unix_ms
            || a.kind != "File"
            || a.digest.len() != 64
            || !a
                .digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || a.path.is_empty()
            || a.path.len() > 1024
            || a.path.starts_with('/')
            || a.path.contains('\\')
            || a.path.contains('\0')
            || a.path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
        {
            return Err(SpiError::Invalid("invalid or mismatched Eggwork artifact"));
        }
        refs.push(ArtifactRef {
            reference: format!("eggwork:artifact:{}", a.artifact_id),
            digest: Some(format!("sha256:{}", a.digest)),
            media_type: Some("application/octet-stream".into()),
        });
    }
    if snapshot
        .result
        .as_ref()
        .is_some_and(|r| r.artifact_count as usize != artifacts.len())
    {
        return Err(SpiError::Invalid("Eggwork artifact count mismatch"));
    }
    let result = match &snapshot.result {
        Some(r) if r.state == snapshot.state => Some(r),
        Some(_) => return Err(SpiError::Invalid("Eggwork snapshot/result state mismatch")),
        None => None,
    };
    let status = match (&result, snapshot.state) {
        (
            Some(_),
            ExecutionState::Accepted
            | ExecutionState::Preparing
            | ExecutionState::Running
            | ExecutionState::Cancelling,
        )
        | (
            None,
            ExecutionState::Accepted
            | ExecutionState::Preparing
            | ExecutionState::Running
            | ExecutionState::Cancelling,
        ) => EvidenceStatus::InProgress,
        (None, _) => EvidenceStatus::Inconclusive,
        (Some(_), ExecutionState::Succeeded) => EvidenceStatus::Passed,
        (Some(_), ExecutionState::Failed | ExecutionState::TimedOut) => EvidenceStatus::Failed,
        (Some(_), ExecutionState::Cancelled) => EvidenceStatus::Skipped,
        (Some(_), ExecutionState::Interrupted) => EvidenceStatus::Inconclusive,
    };
    let mut metadata = BTreeMap::new();
    metadata.insert("execution_id".into(), snapshot.execution_id.clone());
    metadata.insert("generation".into(), snapshot.generation.to_string());
    metadata.insert(
        "native_state".into(),
        format!("{:?}", snapshot.state).to_ascii_lowercase(),
    );
    if let Some(r) = result {
        metadata.insert("stdout_bytes".into(), r.stdout_bytes.to_string());
        metadata.insert("stderr_bytes".into(), r.stderr_bytes.to_string());
        metadata.insert("stdout_omitted".into(), r.stdout_omitted.to_string());
        metadata.insert("stderr_omitted".into(), r.stderr_omitted.to_string());
        metadata.insert("artifact_count".into(), artifacts.len().to_string());
        if let Some(code) = r.exit_code {
            metadata.insert("exit_code".into(), code.to_string());
        }
        if let Some(failure) = r.failure {
            metadata.insert(
                "failure_class".into(),
                format!("{:?}", failure).to_ascii_lowercase(),
            );
        }
        if let Some(failure) = r.finalization_failure {
            metadata.insert(
                "finalization_failure".into(),
                format!("{failure:?}").to_ascii_lowercase(),
            );
        }
        if r.cleanup_warning.is_some() {
            metadata.insert("cleanup_warning".into(), "present".into());
        }
        if let Some(sandbox) = &r.sandbox {
            metadata.insert(
                "sandbox_outcome".into(),
                match sandbox {
                    SandboxResult::NotRequested => "not_requested",
                    SandboxResult::NotApplied { .. } => "not_applied",
                    SandboxResult::Applied { .. } => "applied",
                    SandboxResult::Failed { .. } => "failed",
                }
                .into(),
            );
        }
        if let Some(resources) = &r.resources {
            for (dimension, outcome) in [
                ("memory", &resources.memory_bytes),
                ("cpu", &resources.cpu_millis),
                ("pids", &resources.pids),
            ] {
                let label = match outcome {
                    ResourceDimensionResult::NotRequested => "not_requested",
                    ResourceDimensionResult::NotApplied { .. } => "not_applied",
                    ResourceDimensionResult::Applied { .. } => "applied",
                    ResourceDimensionResult::LimitExceeded { .. } => "limit_exceeded",
                };
                metadata.insert(format!("resource_{dimension}"), label.into());
            }
        }
    }
    let sandbox_failed = result.is_some_and(|r| {
        r.sandbox
            .as_ref()
            .is_some_and(|s| matches!(s, SandboxResult::Failed { .. }))
    });
    let resource_failed = result.is_some_and(|r| {
        r.resources.as_ref().is_some_and(|v| {
            matches!(
                v.memory_bytes,
                ResourceDimensionResult::LimitExceeded { .. }
            ) || matches!(v.cpu_millis, ResourceDimensionResult::LimitExceeded { .. })
                || matches!(v.pids, ResourceDimensionResult::LimitExceeded { .. })
        })
    });
    let status = if result.is_some_and(|r| r.finalization_failure.is_some()) {
        EvidenceStatus::Inconclusive
    } else if sandbox_failed || resource_failed {
        EvidenceStatus::Failed
    } else {
        status
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

fn validate_snapshot(value: &ExecutionSnapshot) -> Result<(), SpiError> {
    if value.schema_version != 1 {
        return Err(SpiError::Invalid("unsupported Eggwork schema version"));
    }
    if value
        .result
        .as_ref()
        .is_some_and(|r| r.cleanup_warning.as_ref().is_some_and(|s| s.len() > 128))
    {
        return Err(SpiError::Invalid("Eggwork warning exceeds bound"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggplan_core::{EvidenceObservationId, SubjectRevision, SubjectState, VerificationDigest};
    fn context() -> ObservationContext {
        ObservationContext {
            observation_id: EvidenceObservationId::new("epe_eggwork_test").unwrap(),
            subject: SubjectRevision {
                subject_kind: "git".into(),
                repository_id: "epr_test".into(),
                revision: "abc".into(),
                state: SubjectState::Clean,
                dirty_digest: None,
            },
            requested_kind: EvidenceKind::Test,
            observed_at_unix_ms: 1_700_000_000_000,
            invocation_ref: None,
            verification_digest: Some(
                VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
            ),
            metadata: BTreeMap::new(),
        }
    }
    fn snapshot(state: &str, terminal: bool) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"execution_id":"exe-test","generation":1,"state":state,
        "result":if terminal {serde_json::json!({"state":state,"exit_code":if state == "Succeeded" {Some(0)} else {None},"failure":null,"stdout_bytes":11,"stderr_bytes":3,"stdout_omitted":0,"stderr_omitted":0,"cleanup_warning":null,"finalization_failure":null,"artifact_count":0,"sandbox":null,"resources":null})} else {serde_json::Value::Null}
    })).unwrap()
    }
    #[test]
    fn states_map_and_failures_remain_distinct() {
        for (state, terminal, expected) in [
            ("Accepted", false, EvidenceStatus::InProgress),
            ("Preparing", false, EvidenceStatus::InProgress),
            ("Running", false, EvidenceStatus::InProgress),
            ("Cancelling", false, EvidenceStatus::InProgress),
            ("Succeeded", true, EvidenceStatus::Passed),
            ("Failed", true, EvidenceStatus::Failed),
            ("TimedOut", true, EvidenceStatus::Failed),
            ("Cancelled", true, EvidenceStatus::Skipped),
            ("Interrupted", true, EvidenceStatus::Inconclusive),
            ("Succeeded", false, EvidenceStatus::Inconclusive),
        ] {
            let input = parse_snapshot(&snapshot(state, terminal)).unwrap();
            assert_eq!(
                normalize(&input, &[], &context()).unwrap().status(),
                expected,
                "{state}"
            );
        }
    }
    #[test]
    fn binding_artifact_identity_and_version_are_validated() {
        let mut input = parse_snapshot(&snapshot("Succeeded", true)).unwrap();
        input.result.as_mut().unwrap().artifact_count = 1;
        let mut no_binding = context();
        no_binding.verification_digest = None;
        assert_eq!(
            normalize(&input, &[], &no_binding),
            Err(SpiError::MissingVerificationBinding)
        );
        let artifact = ArtifactRecord {
            artifact_id: "art1".into(),
            execution_id: "wrong".into(),
            generation: 1,
            path: "out/a".into(),
            kind: "File".into(),
            digest: "b".repeat(64),
            size_bytes: 3,
            executable: false,
            created_unix_ms: 1,
            expires_unix_ms: 2,
        };
        assert!(normalize(&input, &[artifact], &context()).is_err());
        assert!(parse_snapshot(&snapshot("FutureState", false)).is_err());
        assert!(parse_snapshot(&vec![b' '; MAX_SNAPSHOT_BYTES + 1]).is_err());
    }
    #[test]
    fn artifact_refs_are_digest_bound_and_output_content_is_omitted() {
        let mut input = parse_snapshot(&snapshot("Succeeded", true)).unwrap();
        input.result.as_mut().unwrap().artifact_count = 1;
        let artifact = ArtifactRecord {
            artifact_id: "art1".into(),
            execution_id: "exe-test".into(),
            generation: 1,
            path: "out/a".into(),
            kind: "File".into(),
            digest: "b".repeat(64),
            size_bytes: 3,
            executable: false,
            created_unix_ms: 1,
            expires_unix_ms: 2,
        };
        let first = normalize(&input, std::slice::from_ref(&artifact), &context()).unwrap();
        let second = normalize(&input, &[artifact], &context()).unwrap();
        assert_eq!(first.content_digest(), second.content_digest());
        assert_eq!(
            first.artifacts()[0].digest.as_deref(),
            Some(format!("sha256:{}", "b".repeat(64)).as_str())
        );
        assert!(!first.result_metadata().contains_key("stdout"));
        assert_eq!(
            first
                .result_metadata()
                .get("stdout_bytes")
                .map(String::as_str),
            Some("11")
        );
    }

    #[test]
    fn reviewed_sibling_shape_fixtures_cover_terminal_and_artifact_cases() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_slice(include_bytes!("../tests/fixtures/eggwork-snapshots.json"))
                .unwrap();
        let artifacts =
            parse_artifacts(include_bytes!("../tests/fixtures/eggwork-artifacts.json")).unwrap();
        assert_eq!(cases.len(), 7);
        for mut case in cases {
            let name = case["case"].as_str().unwrap().to_owned();
            case.as_object_mut().unwrap().remove("case");
            let input = parse_snapshot(&serde_json::to_vec(&case).unwrap()).unwrap();
            let selected = if name == "success" {
                artifacts.as_slice()
            } else {
                &[]
            };
            let observation = normalize(&input, selected, &context()).unwrap();
            if name == "artifact_finalization_failure" {
                assert_eq!(observation.status(), EvidenceStatus::Inconclusive);
            }
        }
    }
}
