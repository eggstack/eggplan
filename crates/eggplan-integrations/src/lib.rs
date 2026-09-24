#![forbid(unsafe_code)]

//! Synchronous normalization contracts for external evidence producers.
//!
//! This crate acquires no evidence and grants no provider trust. Hosts own
//! transports, native clients, authentication, and registry membership.

use eggplan_core::{
    ArtifactRef, EvidenceKind, EvidenceObservation, EvidenceObservationInput, EvidenceProviderId,
    EvidenceStatus, ProviderDescriptor, SubjectRevision, VerificationDigest,
    bounds::{
        ARTIFACT_REF_CHARS, ID_CHARS, INVOCATION_REF_CHARS, MAX_ARTIFACT_REFS,
        MAX_OBSERVATION_METADATA, OBSERVATION_METADATA_VALUE_CHARS,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const ADAPTER_SCHEMA_VERSION: u32 = 1;
pub const VERIFICATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_CONTEXT_METADATA: usize = 16;
pub const MAX_VERIFICATION_SPEC_BYTES: usize = 65_536;
pub const MAX_VERIFICATION_SPEC_NODES: usize = 4_096;
pub const MAX_VERIFICATION_SPEC_DEPTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderClass {
    Execution,
    Research,
    Benchmark,
    Utility,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub supports_in_progress: bool,
    pub artifacts: bool,
    pub verification_binding: bool,
    pub research_trust_metadata: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterDescriptor {
    pub schema_version: u32,
    pub provider_id: EvidenceProviderId,
    pub provider_class: ProviderClass,
    #[serde(deserialize_with = "deserialize_kinds")]
    pub allowed_kinds: BTreeSet<EvidenceKind>,
    pub adapter_version: String,
    pub capabilities: Capabilities,
}

impl AdapterDescriptor {
    pub fn new(
        provider_id: EvidenceProviderId,
        provider_class: ProviderClass,
        allowed_kinds: impl IntoIterator<Item = EvidenceKind>,
        adapter_version: impl Into<String>,
        capabilities: Capabilities,
    ) -> Result<Self, SpiError> {
        let kinds: Vec<_> = allowed_kinds.into_iter().collect();
        let allowed_kinds: BTreeSet<_> = kinds.iter().copied().collect();
        if kinds.len() != allowed_kinds.len() {
            return Err(SpiError::Invalid("duplicate evidence kind"));
        }
        let value = Self {
            schema_version: ADAPTER_SCHEMA_VERSION,
            provider_id,
            provider_class,
            allowed_kinds: allowed_kinds.into_iter().collect(),
            adapter_version: adapter_version.into(),
            capabilities,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), SpiError> {
        if self.schema_version != ADAPTER_SCHEMA_VERSION {
            return Err(SpiError::Invalid("unknown adapter descriptor schema"));
        }
        bounded_text(&self.adapter_version, "adapter version", ID_CHARS)?;
        if self.allowed_kinds.is_empty() {
            return Err(SpiError::Invalid(
                "adapter must allow at least one evidence kind",
            ));
        }
        if self.allowed_kinds.contains(&EvidenceKind::HumanJudgment) {
            return Err(SpiError::Invalid("adapters cannot assert human judgment"));
        }
        Ok(())
    }

    /// A host may pass this descriptor to its explicit trusted-provider
    /// registry. Calling this method does not mutate or enroll that registry.
    pub fn provider_descriptor(&self) -> Result<ProviderDescriptor, SpiError> {
        self.validate()?;
        ProviderDescriptor::new(
            self.provider_id.clone(),
            format!("{:?}", self.provider_class).to_ascii_lowercase(),
            self.allowed_kinds.iter().copied(),
        )
        .map_err(|_| SpiError::Invalid("core provider descriptor rejected"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationContext {
    pub observation_id: eggplan_core::EvidenceObservationId,
    pub subject: SubjectRevision,
    pub requested_kind: EvidenceKind,
    pub observed_at_unix_ms: u64,
    #[serde(default)]
    pub invocation_ref: Option<String>,
    #[serde(default)]
    pub verification_digest: Option<VerificationDigest>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl ObservationContext {
    pub fn validate(&self) -> Result<(), SpiError> {
        self.subject
            .validate()
            .map_err(|_| SpiError::Invalid("invalid subject"))?;
        if let Some(value) = &self.invocation_ref {
            bounded_text(value, "invocation reference", INVOCATION_REF_CHARS)?;
            if looks_like_secret_or_endpoint(value) {
                return Err(SpiError::Invalid(
                    "secret or endpoint cannot be persisted as invocation reference",
                ));
            }
        }
        validate_map(&self.metadata, MAX_CONTEXT_METADATA)?;
        Ok(())
    }
}

/// Native facts that an adapter has normalized. It deliberately has no
/// provider ID: finalization takes authority from AdapterDescriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedProviderResult {
    pub status: EvidenceStatus,
    #[serde(default)]
    pub source_trust: Option<SourceTrust>,
    #[serde(default)]
    pub result_metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTrust {
    ProviderTrusted,
    ExternalUntrusted,
}

impl NormalizedProviderResult {
    pub fn validate(&self) -> Result<(), SpiError> {
        validate_map(&self.result_metadata, MAX_OBSERVATION_METADATA)?;
        if self.artifacts.len() > MAX_ARTIFACT_REFS {
            return Err(SpiError::Invalid("artifact reference limit exceeded"));
        }
        for artifact in &self.artifacts {
            if artifact.reference.chars().count() > ARTIFACT_REF_CHARS {
                return Err(SpiError::Invalid("artifact reference exceeds bound"));
            }
            if looks_like_secret_or_endpoint(&artifact.reference) {
                return Err(SpiError::Invalid(
                    "secret or endpoint cannot be persisted as artifact reference",
                ));
            }
            artifact
                .validate()
                .map_err(|_| SpiError::Invalid("invalid artifact reference"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SpiError {
    #[error("invalid provider SPI input: {0}")]
    Invalid(&'static str),
    #[error("unsupported evidence kind for provider")]
    UnsupportedKind,
    #[error("execution-derived evidence requires verification binding")]
    MissingVerificationBinding,
    #[error("verification binding capability is not declared")]
    VerificationBindingNotSupported,
    #[error("artifact capability is not declared")]
    ArtifactsNotSupported,
    #[error("in-progress capability is not declared")]
    InProgressNotSupported,
    #[error("native result cannot assert human judgment")]
    HumanJudgmentUnsupported,
    #[error("invalid canonical verification specification: {0}")]
    InvalidVerificationSpec(String),
}

/// Derive a deterministic, provider-namespaced digest from a bounded,
/// versioned JSON specification. JSON object key ordering is canonicalized
/// recursively before domain-separated SHA-256 hashing.
pub fn verification_digest(
    provider_namespace: &str,
    schema_version: u32,
    payload: &serde_json::Value,
) -> Result<VerificationDigest, SpiError> {
    bounded_text(provider_namespace, "provider namespace", ID_CHARS)?;
    if schema_version == 0 {
        return Err(SpiError::Invalid(
            "verification schema version must be nonzero",
        ));
    }
    let mut node_count = 0;
    validate_verification_value(payload, 0, &mut node_count)?;
    eggplan_core::verification_digest(provider_namespace, schema_version, payload)
        .map_err(SpiError::InvalidVerificationSpec)
}

fn deserialize_kinds<'de, D>(deserializer: D) -> Result<BTreeSet<EvidenceKind>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let kinds = Vec::<EvidenceKind>::deserialize(deserializer)?;
    let set: BTreeSet<_> = kinds.iter().copied().collect();
    if kinds.len() != set.len() {
        return Err(serde::de::Error::custom("duplicate evidence kind"));
    }
    Ok(set)
}

fn validate_verification_value(
    value: &serde_json::Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), SpiError> {
    *nodes += 1;
    if depth > MAX_VERIFICATION_SPEC_DEPTH || *nodes > MAX_VERIFICATION_SPEC_NODES {
        return Err(SpiError::Invalid(
            "verification specification structure exceeds bound",
        ));
    }
    match value {
        serde_json::Value::String(text) => {
            if text.contains('\0') || text.chars().count() > 4_000 {
                return Err(SpiError::Invalid(
                    "verification string contains NUL or exceeds bound",
                ));
            }
        }
        serde_json::Value::Array(values) => {
            if values.len() > 512 {
                return Err(SpiError::Invalid("verification array exceeds bound"));
            }
            for value in values {
                validate_verification_value(value, depth + 1, nodes)?;
            }
        }
        serde_json::Value::Object(values) => {
            if values.len() > 512 {
                return Err(SpiError::Invalid("verification object exceeds bound"));
            }
            for (key, value) in values {
                bounded_text(key, "verification key", ID_CHARS)?;
                validate_verification_value(value, depth + 1, nodes)?;
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
    Ok(())
}

pub fn finalize_observation(
    descriptor: &AdapterDescriptor,
    context: &ObservationContext,
    result: &NormalizedProviderResult,
) -> Result<EvidenceObservation, SpiError> {
    descriptor.validate()?;
    context.validate()?;
    result.validate()?;
    if !descriptor.allowed_kinds.contains(&context.requested_kind) {
        return Err(SpiError::UnsupportedKind);
    }
    if context.requested_kind == EvidenceKind::HumanJudgment {
        return Err(SpiError::HumanJudgmentUnsupported);
    }
    if descriptor.provider_class == ProviderClass::Research && result.source_trust.is_none() {
        return Err(SpiError::Invalid(
            "research results must preserve a source trust marker",
        ));
    }
    if result.source_trust.is_some() && !descriptor.capabilities.research_trust_metadata {
        return Err(SpiError::Invalid(
            "provider does not declare research trust metadata capability",
        ));
    }
    if result.source_trust == Some(SourceTrust::ExternalUntrusted)
        && result.status == EvidenceStatus::Passed
    {
        return Err(SpiError::Invalid(
            "external-untrusted source cannot normalize as passed evidence",
        ));
    }
    if result.status == EvidenceStatus::InProgress && !descriptor.capabilities.supports_in_progress
    {
        return Err(SpiError::InProgressNotSupported);
    }
    if !result.artifacts.is_empty() && !descriptor.capabilities.artifacts {
        return Err(SpiError::ArtifactsNotSupported);
    }
    let execution_derived = matches!(
        context.requested_kind,
        EvidenceKind::Command
            | EvidenceKind::Test
            | EvidenceKind::StaticAnalysis
            | EvidenceKind::DelegatedRun
            | EvidenceKind::Benchmark
    );
    if execution_derived && context.verification_digest.is_none() {
        return Err(SpiError::MissingVerificationBinding);
    }
    if context.verification_digest.is_some() && !descriptor.capabilities.verification_binding {
        return Err(SpiError::VerificationBindingNotSupported);
    }
    let mut metadata = context.metadata.clone();
    if let Some(trust) = result.source_trust
        && metadata
            .insert(
                "source_trust".into(),
                match trust {
                    SourceTrust::ProviderTrusted => "provider_trusted",
                    SourceTrust::ExternalUntrusted => "external_untrusted",
                }
                .into(),
            )
            .is_some()
    {
        return Err(SpiError::Invalid(
            "source trust metadata is reserved for the normalized trust marker",
        ));
    }
    for (key, value) in &result.result_metadata {
        if metadata.insert(key.clone(), value.clone()).is_some() {
            return Err(SpiError::Invalid("duplicate context/result metadata key"));
        }
    }
    validate_map(&metadata, MAX_OBSERVATION_METADATA)?;
    EvidenceObservation::finalize(EvidenceObservationInput {
        id: context.observation_id.clone(),
        provider_id: descriptor.provider_id.clone(),
        kind: context.requested_kind,
        status: result.status,
        subject: context.subject.clone(),
        observed_at_unix_ms: context.observed_at_unix_ms,
        invocation_ref: context.invocation_ref.clone(),
        verification_digest: context.verification_digest.clone(),
        result_metadata: metadata,
        artifacts: result.artifacts.clone(),
    })
    .map_err(|_| SpiError::Invalid("core rejected normalized observation"))
}

fn validate_map(map: &BTreeMap<String, String>, max_entries: usize) -> Result<(), SpiError> {
    if map.len() > max_entries {
        return Err(SpiError::Invalid("metadata entry limit exceeded"));
    }
    for (key, value) in map {
        bounded_text(key, "metadata key", ID_CHARS)?;
        bounded_text(value, "metadata value", OBSERVATION_METADATA_VALUE_CHARS)?;
        if is_sensitive_key(key) || looks_like_secret_or_endpoint(value) {
            return Err(SpiError::Invalid(
                "sensitive metadata cannot be persisted by the generic SPI",
            ));
        }
    }
    Ok(())
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "token",
        "password",
        "secret",
        "credential",
        "authorization",
        "endpoint",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn looks_like_secret_or_endpoint(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("://")
        || value.starts_with("sk-")
        || [
            "bearer ",
            "token=",
            "password=",
            "secret=",
            "api_key=",
            "api-key=",
        ]
        .iter()
        .any(|needle| value.contains(needle))
}

fn bounded_text(value: &str, _label: &'static str, max: usize) -> Result<(), SpiError> {
    let count = value.chars().count();
    if value.is_empty() || value.contains('\0') || count > max {
        Err(SpiError::Invalid(
            "text is empty, contains NUL, or exceeds bound",
        ))
    } else {
        Ok(())
    }
}
