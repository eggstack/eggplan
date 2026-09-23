use crate::{
    ArtifactRef, EvidenceKind, EvidenceObservationId, EvidenceProviderId, SubjectRevision,
    ValidationError, VerificationDigest, bounds::*, canonical_json, digest_json,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const EVIDENCE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Passed,
    Failed,
    InProgress,
    NotRun,
    Skipped,
    Blocked,
    Unavailable,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceObservation {
    schema_version: u32,
    id: EvidenceObservationId,
    provider_id: EvidenceProviderId,
    kind: EvidenceKind,
    status: EvidenceStatus,
    subject: SubjectRevision,
    observed_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    invocation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_digest: Option<VerificationDigest>,
    result_metadata: BTreeMap<String, String>,
    artifacts: Vec<ArtifactRef>,
    content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceObservationInput {
    pub id: EvidenceObservationId,
    pub provider_id: EvidenceProviderId,
    pub kind: EvidenceKind,
    pub status: EvidenceStatus,
    pub subject: SubjectRevision,
    pub observed_at_unix_ms: u64,
    pub invocation_ref: Option<String>,
    pub verification_digest: Option<VerificationDigest>,
    pub result_metadata: BTreeMap<String, String>,
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationContent {
    schema_version: u32,
    id: EvidenceObservationId,
    provider_id: EvidenceProviderId,
    kind: EvidenceKind,
    status: EvidenceStatus,
    subject: SubjectRevision,
    observed_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    invocation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_digest: Option<VerificationDigest>,
    result_metadata: BTreeMap<String, String>,
    artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvidenceError {
    #[error("invalid evidence observation: {0}")]
    Invalid(String),
    #[error("unknown evidence schema version {0}")]
    UnknownSchema(u32),
    #[error("evidence content digest mismatch")]
    DigestMismatch,
    #[error("provider {0} is already registered")]
    DuplicateProvider(EvidenceProviderId),
}

/// A descriptor is trusted because the host/provider adapter adds it to the
/// explicit registry supplied to assessment. Persisted observation text alone
/// cannot enroll its own provider or widen this descriptor's allowed kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub(crate) id: EvidenceProviderId,
    pub(crate) class: String,
    pub(crate) allowed_kinds: BTreeSet<EvidenceKind>,
}

impl ProviderDescriptor {
    pub fn new(
        id: EvidenceProviderId,
        class: impl Into<String>,
        allowed_kinds: impl IntoIterator<Item = EvidenceKind>,
    ) -> Result<Self, EvidenceError> {
        let class = class.into();
        validate_text(&class, "provider class", 64)?;
        let allowed_kinds: BTreeSet<_> = allowed_kinds.into_iter().collect();
        if allowed_kinds.is_empty() {
            return Err(EvidenceError::Invalid(
                "provider must allow at least one kind".into(),
            ));
        }
        Ok(Self {
            id,
            class,
            allowed_kinds,
        })
    }

    pub fn id(&self) -> &EvidenceProviderId {
        &self.id
    }
    pub fn class(&self) -> &str {
        &self.class
    }
    pub fn allowed_kinds(&self) -> &BTreeSet<EvidenceKind> {
        &self.allowed_kinds
    }

    fn validate(&self) -> Result<(), EvidenceError> {
        validate_text(&self.class, "provider class", 64)?;
        if self.allowed_kinds.is_empty() {
            return Err(EvidenceError::Invalid(
                "provider must allow at least one kind".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    providers: BTreeMap<EvidenceProviderId, ProviderDescriptor>,
}

impl ProviderRegistry {
    pub fn register_trusted(
        &mut self,
        descriptor: ProviderDescriptor,
    ) -> Result<(), EvidenceError> {
        descriptor.validate()?;
        let id = descriptor.id.clone();
        if self.providers.contains_key(&id) {
            return Err(EvidenceError::DuplicateProvider(id));
        }
        self.providers.insert(id, descriptor);
        Ok(())
    }
    pub fn get(&self, id: &EvidenceProviderId) -> Option<&ProviderDescriptor> {
        self.providers.get(id)
    }
}

impl EvidenceObservation {
    pub fn finalize(input: EvidenceObservationInput) -> Result<Self, EvidenceError> {
        let mut observation = Self {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            id: input.id,
            provider_id: input.provider_id,
            kind: input.kind,
            status: input.status,
            subject: input.subject,
            observed_at_unix_ms: input.observed_at_unix_ms,
            invocation_ref: input.invocation_ref,
            verification_digest: input.verification_digest,
            result_metadata: input.result_metadata,
            artifacts: input.artifacts,
            content_digest: String::new(),
        };
        observation.validate_fields()?;
        observation.content_digest = digest_json(&observation.content())
            .map_err(|e| EvidenceError::Invalid(e.to_string()))?;
        Ok(observation)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let observation: Self = serde_json::from_slice(bytes)?;
        observation.validate()?;
        Ok(observation)
    }

    pub fn validate(&self) -> Result<(), EvidenceError> {
        self.validate_fields()?;
        let digest =
            digest_json(&self.content()).map_err(|e| EvidenceError::Invalid(e.to_string()))?;
        if digest != self.content_digest {
            return Err(EvidenceError::DigestMismatch);
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        canonical_json(self)
    }
    pub fn id(&self) -> &EvidenceObservationId {
        &self.id
    }
    pub fn provider_id(&self) -> &EvidenceProviderId {
        &self.provider_id
    }
    pub fn kind(&self) -> EvidenceKind {
        self.kind
    }
    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn subject(&self) -> &SubjectRevision {
        &self.subject
    }
    pub fn observed_at_unix_ms(&self) -> u64 {
        self.observed_at_unix_ms
    }
    pub fn invocation_ref(&self) -> Option<&str> {
        self.invocation_ref.as_deref()
    }
    pub fn verification_digest(&self) -> Option<&VerificationDigest> {
        self.verification_digest.as_ref()
    }
    pub fn result_metadata(&self) -> &BTreeMap<String, String> {
        &self.result_metadata
    }
    pub fn artifacts(&self) -> &[ArtifactRef] {
        &self.artifacts
    }
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    fn content(&self) -> ObservationContent {
        ObservationContent {
            schema_version: self.schema_version,
            id: self.id.clone(),
            provider_id: self.provider_id.clone(),
            kind: self.kind,
            status: self.status,
            subject: self.subject.clone(),
            observed_at_unix_ms: self.observed_at_unix_ms,
            invocation_ref: self.invocation_ref.clone(),
            verification_digest: self.verification_digest.clone(),
            result_metadata: self.result_metadata.clone(),
            artifacts: self.artifacts.clone(),
        }
    }

    fn validate_fields(&self) -> Result<(), EvidenceError> {
        if !matches!(self.schema_version, 1 | EVIDENCE_SCHEMA_VERSION) {
            return Err(EvidenceError::UnknownSchema(self.schema_version));
        }
        if self.schema_version == 1 && self.verification_digest.is_some() {
            return Err(EvidenceError::Invalid(
                "schema-v1 observation cannot carry verification binding".into(),
            ));
        }
        if self.schema_version == EVIDENCE_SCHEMA_VERSION
            && crate::is_execution_evidence(self.kind)
            && self.verification_digest.is_none()
        {
            return Err(EvidenceError::Invalid(
                "schema-v2 execution observation requires verification binding".into(),
            ));
        }
        self.subject
            .validate()
            .map_err(|e| EvidenceError::Invalid(e.to_string()))?;
        if let Some(invocation) = &self.invocation_ref {
            validate_text(invocation, "invocation_ref", INVOCATION_REF_CHARS)?;
        }
        if self.result_metadata.len() > MAX_OBSERVATION_METADATA {
            return Err(EvidenceError::Invalid(
                "result metadata entry limit exceeded".into(),
            ));
        }
        for (key, value) in &self.result_metadata {
            validate_text(key, "result metadata key", ID_CHARS)?;
            validate_text(
                value,
                "result metadata value",
                OBSERVATION_METADATA_VALUE_CHARS,
            )?;
        }
        if self.artifacts.len() > MAX_ARTIFACT_REFS {
            return Err(EvidenceError::Invalid(
                "artifact reference limit exceeded".into(),
            ));
        }
        for artifact in &self.artifacts {
            artifact
                .validate()
                .map_err(|e| EvidenceError::Invalid(e.to_string()))?;
        }
        Ok(())
    }
}

fn validate_text(value: &str, label: &str, max: usize) -> Result<(), EvidenceError> {
    let count = value.chars().count();
    if value.is_empty() || value.contains('\0') || count > max {
        Err(EvidenceError::Invalid(format!(
            "{label} is empty, contains NUL, or exceeds {max} Unicode scalar values"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn finalize_v1_for_test(mut input: EvidenceObservationInput) -> EvidenceObservation {
    input.verification_digest = None;
    let mut observation = EvidenceObservation {
        schema_version: 1,
        id: input.id,
        provider_id: input.provider_id,
        kind: input.kind,
        status: input.status,
        subject: input.subject,
        observed_at_unix_ms: input.observed_at_unix_ms,
        invocation_ref: input.invocation_ref,
        verification_digest: None,
        result_metadata: input.result_metadata,
        artifacts: input.artifacts,
        content_digest: String::new(),
    };
    observation.content_digest = digest_json(&observation.content()).unwrap();
    observation.validate().unwrap();
    observation
}

impl From<ValidationError> for EvidenceError {
    fn from(value: ValidationError) -> Self {
        Self::Invalid(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SubjectState;

    fn input(status: EvidenceStatus) -> EvidenceObservationInput {
        EvidenceObservationInput {
            id: EvidenceObservationId::new("epe_test").unwrap(),
            provider_id: EvidenceProviderId::new("epp_test").unwrap(),
            kind: EvidenceKind::Test,
            status,
            subject: SubjectRevision {
                subject_kind: "git".into(),
                repository_id: "epr_test".into(),
                revision: "abc123".into(),
                state: SubjectState::Clean,
                dirty_digest: None,
            },
            observed_at_unix_ms: 1_700_000_000_000,
            invocation_ref: None,
            verification_digest: Some(
                VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
            ),
            result_metadata: BTreeMap::new(),
            artifacts: vec![],
        }
    }

    #[test]
    fn observation_digest_roundtrip_and_status_vocabulary() {
        let statuses = [
            EvidenceStatus::Passed,
            EvidenceStatus::Failed,
            EvidenceStatus::InProgress,
            EvidenceStatus::NotRun,
            EvidenceStatus::Skipped,
            EvidenceStatus::Blocked,
            EvidenceStatus::Unavailable,
            EvidenceStatus::Inconclusive,
        ];
        let fixtures: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../tests/fixtures/evidence-v1-digests.json"))
                .unwrap();
        for status in statuses {
            let observation = finalize_v1_for_test(input(status));
            assert_eq!(
                observation.content_digest(),
                fixtures[&format!("status:{}", status_name(status))]
            );
            let parsed =
                EvidenceObservation::parse(&observation.canonical_json().unwrap()).unwrap();
            assert_eq!(parsed, observation);
            assert_eq!(parsed.status(), status);
        }
        for kind in [
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
        ] {
            let mut observation_input = input(EvidenceStatus::Passed);
            observation_input.kind = kind;
            observation_input.id = EvidenceObservationId::new(format!(
                "epe_kind_{}",
                format!("{kind:?}").to_lowercase()
            ))
            .unwrap();
            let observation = finalize_v1_for_test(observation_input);
            assert_eq!(
                observation.content_digest(),
                fixtures[&format!("kind:{}", kind_name(kind))]
            );
        }
    }

    #[test]
    fn schema_v2_evidence_golden_bytes_and_digest() {
        let observation = EvidenceObservation::finalize(input(EvidenceStatus::Passed)).unwrap();
        let bytes = observation.canonical_json().unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            include_str!("../tests/fixtures/evidence-v2-observation.json").trim()
        );
        assert_eq!(
            observation.content_digest(),
            include_str!("../tests/fixtures/evidence-v2-observation.sha256").trim()
        );
    }

    #[test]
    fn legacy_evidence_rejects_unknown_nested_subject_and_artifact_fields() {
        let observation = finalize_v1_for_test(input(EvidenceStatus::Passed));
        let mut value: serde_json::Value =
            serde_json::from_slice(&observation.canonical_json().unwrap()).unwrap();
        value["subject"]["future"] = serde_json::json!(true);
        assert!(EvidenceObservation::parse(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value =
            serde_json::from_slice(&observation.canonical_json().unwrap()).unwrap();
        value["artifacts"] = serde_json::json!([{"reference":"artifact.bin","future":true}]);
        assert!(EvidenceObservation::parse(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value =
            serde_json::from_slice(&observation.canonical_json().unwrap()).unwrap();
        value["future"] = serde_json::json!("unknown");
        assert!(EvidenceObservation::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn schema_v2_execution_observations_require_bindings() {
        for kind in [
            EvidenceKind::Command,
            EvidenceKind::Test,
            EvidenceKind::StaticAnalysis,
            EvidenceKind::DelegatedRun,
            EvidenceKind::Benchmark,
        ] {
            let mut input = input(EvidenceStatus::Passed);
            input.kind = kind;
            input.verification_digest = None;
            assert!(
                EvidenceObservation::finalize(input).is_err(),
                "{kind:?} accepted without a binding"
            );
        }
    }

    #[test]
    fn content_change_changes_digest_and_tampering_fails() {
        let observation = EvidenceObservation::finalize(input(EvidenceStatus::Passed)).unwrap();
        let mut changed = input(EvidenceStatus::Passed);
        changed
            .result_metadata
            .insert("summary".into(), "different".into());
        let changed = EvidenceObservation::finalize(changed).unwrap();
        assert_ne!(observation.content_digest(), changed.content_digest());
        let tampered = String::from_utf8(observation.canonical_json().unwrap())
            .unwrap()
            .replace("abc123", "def456");
        assert!(
            matches!(EvidenceObservation::parse(tampered.as_bytes()), Err(e) if e.to_string().contains("digest mismatch"))
        );
        let unknown_schema = String::from_utf8(observation.canonical_json().unwrap())
            .unwrap()
            .replace("\"schema_version\":2", "\"schema_version\":3");
        assert!(matches!(
            EvidenceObservation::parse(unknown_schema.as_bytes()),
            Err(error) if error.to_string().contains("unknown evidence schema version 3")
        ));
    }

    #[test]
    fn provider_registry_rejects_duplicate_identity() {
        let provider = EvidenceProviderId::new("epp_host").unwrap();
        let descriptor =
            ProviderDescriptor::new(provider.clone(), "host", [EvidenceKind::Test]).unwrap();
        let mut registry = ProviderRegistry::default();
        registry.register_trusted(descriptor.clone()).unwrap();
        assert!(matches!(
            registry.register_trusted(descriptor),
            Err(EvidenceError::DuplicateProvider(_))
        ));
    }

    #[test]
    fn observation_bounds_reject_oversize_metadata_and_invocation() {
        let mut data = input(EvidenceStatus::Passed);
        for index in 0..=MAX_OBSERVATION_METADATA {
            data.result_metadata
                .insert(format!("key_{index}"), "v".into());
        }
        assert!(EvidenceObservation::finalize(data).is_err());
        let mut data = input(EvidenceStatus::Passed);
        data.invocation_ref = Some("x".repeat(INVOCATION_REF_CHARS + 1));
        assert!(EvidenceObservation::finalize(data).is_err());
    }

    fn status_name(status: EvidenceStatus) -> &'static str {
        match status {
            EvidenceStatus::Passed => "passed",
            EvidenceStatus::Failed => "failed",
            EvidenceStatus::InProgress => "in_progress",
            EvidenceStatus::NotRun => "not_run",
            EvidenceStatus::Skipped => "skipped",
            EvidenceStatus::Blocked => "blocked",
            EvidenceStatus::Unavailable => "unavailable",
            EvidenceStatus::Inconclusive => "inconclusive",
        }
    }

    fn kind_name(kind: EvidenceKind) -> &'static str {
        match kind {
            EvidenceKind::Command => "command",
            EvidenceKind::Test => "test",
            EvidenceKind::StaticAnalysis => "static_analysis",
            EvidenceKind::Revision => "revision",
            EvidenceKind::Artifact => "artifact",
            EvidenceKind::DelegatedRun => "delegated_run",
            EvidenceKind::Benchmark => "benchmark",
            EvidenceKind::Research => "research",
            EvidenceKind::HumanJudgment => "human_judgment",
            EvidenceKind::Attestation => "attestation",
        }
    }
}
