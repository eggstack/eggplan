use eggplan_core::{
    AcceptanceCriterion, ArtifactRef, AssessmentStatus, EvidenceCardinality, EvidenceKind,
    EvidenceObservationId, EvidenceProviderId, EvidenceRequirement, EvidenceStatus, Plan, PlanId,
    PlanItem, PlanItemId, PlanItemStatus, PlanStatus, ProviderRegistry, SubjectPolicy,
    SubjectRevision, SubjectState, VerificationDigest, assess_plan,
};
use eggplan_integrations::{
    AdapterDescriptor, Capabilities, NormalizedProviderResult, ObservationContext, ProviderClass,
    SourceTrust, SpiError, finalize_observation, verification_digest,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const RUNNING_CAPS: Capabilities = Capabilities {
    supports_in_progress: true,
    artifacts: true,
    verification_binding: true,
    research_trust_metadata: true,
};

fn descriptor(provider: &str, class: ProviderClass, kinds: &[EvidenceKind]) -> AdapterDescriptor {
    AdapterDescriptor::new(
        EvidenceProviderId::new(provider).unwrap(),
        class,
        kinds.iter().copied(),
        "1.0",
        RUNNING_CAPS,
    )
    .unwrap()
}

fn context(kind: EvidenceKind, binding: Option<VerificationDigest>) -> ObservationContext {
    ObservationContext {
        observation_id: EvidenceObservationId::new("epe_spi_test").unwrap(),
        subject: SubjectRevision {
            subject_kind: "git".into(),
            repository_id: "epr_fixture_repo".into(),
            revision: "abc123".into(),
            state: SubjectState::Clean,
            dirty_digest: None,
        },
        requested_kind: kind,
        observed_at_unix_ms: 1_700_000_000_000,
        invocation_ref: Some("native:fixture/1".into()),
        verification_digest: binding,
        metadata: BTreeMap::new(),
    }
}

fn result(status: EvidenceStatus) -> NormalizedProviderResult {
    NormalizedProviderResult {
        status,
        source_trust: None,
        result_metadata: BTreeMap::new(),
        artifacts: vec![],
    }
}

fn binding() -> VerificationDigest {
    verification_digest("eggwork", 1, &json!({"target":"fixed", "args":["check"]})).unwrap()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCorpus {
    schema_version: u32,
    fixture_only: bool,
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    provider: String,
    native_state: String,
    status: EvidenceStatus,
    #[serde(default)]
    source_trust: Option<SourceTrust>,
    #[serde(default)]
    gap_count: Option<u32>,
    #[serde(default)]
    comparison_verdict: Option<String>,
}

#[test]
fn descriptor_is_strict_bounded_and_does_not_self_register() {
    let d = descriptor(
        "epp_eggwork",
        ProviderClass::Execution,
        &[EvidenceKind::Test],
    );
    assert_eq!(d.allowed_kinds.len(), 1);
    let host_registry = ProviderRegistry::default();
    assert!(
        host_registry
            .get(&EvidenceProviderId::new("epp_eggwork").unwrap())
            .is_none()
    );
    let mut host_registry = host_registry;
    host_registry
        .register_trusted(d.provider_descriptor().unwrap())
        .unwrap();
    assert!(
        host_registry
            .get(&EvidenceProviderId::new("epp_eggwork").unwrap())
            .is_some()
    );

    assert!(
        AdapterDescriptor::new(
            EvidenceProviderId::new("epp_empty").unwrap(),
            ProviderClass::Other,
            [],
            "1.0",
            RUNNING_CAPS,
        )
        .is_err()
    );
    assert!(
        AdapterDescriptor::new(
            EvidenceProviderId::new("epp_dupe").unwrap(),
            ProviderClass::Other,
            [EvidenceKind::Test, EvidenceKind::Test],
            "1.0",
            RUNNING_CAPS,
        )
        .is_err()
    );
    assert!(serde_json::from_str::<AdapterDescriptor>(
        r#"{"schema_version":1,"provider_id":"epp_test","provider_class":"execution","allowed_kinds":["test"],"adapter_version":"1","capabilities":{"supports_in_progress":false,"artifacts":false,"verification_binding":false,"research_trust_metadata":false},"trusted":true}"#
    ).is_err());
    assert!(serde_json::from_str::<AdapterDescriptor>(
        r#"{"schema_version":1,"provider_id":"epp_test","provider_class":"execution","allowed_kinds":["test","test"],"adapter_version":"1","capabilities":{"supports_in_progress":false,"artifacts":false,"verification_binding":false,"research_trust_metadata":false}}"#
    ).is_err());
    assert!(serde_json::from_str::<AdapterDescriptor>(
        r#"{"schema_version":1,"provider_id":"epp_test","provider_class":"execution","allowed_kinds":["test","future_kind"],"adapter_version":"1","capabilities":{"supports_in_progress":false,"artifacts":false,"verification_binding":false,"research_trust_metadata":false}}"#
    ).is_err());
}

#[test]
fn verification_digest_is_canonical_versioned_and_namespaced() {
    let first = verification_digest("eggwork", 1, &json!({"b":2,"a":{"z":1,"y":0}})).unwrap();
    let reordered = verification_digest("eggwork", 1, &json!({"a":{"y":0,"z":1},"b":2})).unwrap();
    let golden: Value =
        serde_json::from_slice(include_bytes!("fixtures/verification-digest.json")).unwrap();
    assert_eq!(first.as_str(), golden["digest"]);
    assert_eq!(first, reordered);
    assert_ne!(
        first,
        verification_digest("eggsearch", 1, &json!({"b":2,"a":{"z":1,"y":0}})).unwrap()
    );
    assert_ne!(
        first,
        verification_digest("eggwork", 2, &json!({"b":2,"a":{"z":1,"y":0}})).unwrap()
    );
    assert_ne!(
        first,
        verification_digest("eggwork", 1, &json!({"b":3,"a":{"z":1,"y":0}})).unwrap()
    );
    assert!(verification_digest("", 1, &json!({})).is_err());
    assert!(verification_digest("eggwork", 0, &json!({})).is_err());
    assert!(verification_digest("eggwork", 1, &json!({"large": "x".repeat(65_000)})).is_err());
    let mut nested = json!(null);
    for _ in 0..40 {
        nested = json!([nested]);
    }
    assert!(verification_digest("eggwork", 1, &nested).is_err());
}

#[test]
fn finalization_uses_descriptor_authority_and_requires_execution_binding() {
    let descriptor = descriptor(
        "epp_eggwork",
        ProviderClass::Execution,
        &[EvidenceKind::Test],
    );
    let observation = finalize_observation(
        &descriptor,
        &context(EvidenceKind::Test, Some(binding())),
        &result(EvidenceStatus::Passed),
    )
    .unwrap();
    assert_eq!(observation.provider_id().as_str(), "epp_eggwork");
    assert_eq!(observation.status(), EvidenceStatus::Passed);
    assert_eq!(observation.kind(), EvidenceKind::Test);
    assert_eq!(observation.content_digest().len(), 71);
    assert_eq!(observation.verification_digest(), Some(&binding()));
    assert_eq!(observation.subject().revision, "abc123");
    assert_eq!(
        finalize_observation(
            &descriptor,
            &context(EvidenceKind::Test, None),
            &result(EvidenceStatus::Passed)
        ),
        Err(SpiError::MissingVerificationBinding)
    );
    assert_eq!(
        finalize_observation(
            &descriptor,
            &context(EvidenceKind::Research, None),
            &result(EvidenceStatus::Passed)
        ),
        Err(SpiError::UnsupportedKind)
    );
}

#[test]
fn passing_synthetic_execution_is_usable_only_after_host_trust_registration() {
    let descriptor = descriptor(
        "epp_eggwork",
        ProviderClass::Execution,
        &[EvidenceKind::Test],
    );
    let observation = finalize_observation(
        &descriptor,
        &context(EvidenceKind::Test, Some(binding())),
        &result(EvidenceStatus::Passed),
    )
    .unwrap();
    let mut plan = Plan {
        schema_version: eggplan_core::SCHEMA_VERSION,
        id: PlanId::new("ep_spi_trust").unwrap(),
        revision: 0,
        objective: "qualify host registration".into(),
        status: PlanStatus::Closed,
        provenance: BTreeMap::new(),
        items: vec![PlanItem {
            id: PlanItemId::new("epi_spi_trust").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Completed,
            description: "test provider trust".into(),
            criteria: vec![AcceptanceCriterion {
                id: eggplan_core::CriterionId::new("epc_spi_trust").unwrap(),
                statement: "host explicitly trusts the provider".into(),
                human_judgment_allowed: false,
                requirements: vec![EvidenceRequirement {
                    description: "passing host test".into(),
                    kind: EvidenceKind::Test,
                    provider: Some(EvidenceProviderId::new("epp_eggwork").unwrap()),
                    subject_policy: SubjectPolicy::Exact,
                    cardinality: EvidenceCardinality::Any,
                    min_count: 1,
                    allow_human_judgment: false,
                    expected_verification_digest: Some(binding()),
                }],
            }],
            blocker: None,
            next_action: None,
        }],
        subject: None,
    };
    let current_subject = observation.subject().clone();
    let untrusted = ProviderRegistry::default();
    assert_ne!(
        assess_plan(
            &plan,
            &current_subject,
            std::slice::from_ref(&observation),
            &untrusted
        )
        .status,
        AssessmentStatus::Complete
    );
    let mut host_policy = ProviderRegistry::default();
    host_policy
        .register_trusted(descriptor.provider_descriptor().unwrap())
        .unwrap();
    plan.items[0].status = PlanItemStatus::Completed;
    assert_eq!(
        assess_plan(&plan, &current_subject, &[observation], &host_policy).status,
        AssessmentStatus::Complete
    );
}

#[test]
fn status_normalization_preserves_native_outcomes_and_content_digest() {
    let descriptor = descriptor(
        "epp_native",
        ProviderClass::Other,
        &[EvidenceKind::Artifact],
    );
    for status in [
        EvidenceStatus::InProgress,
        EvidenceStatus::Failed,
        EvidenceStatus::NotRun,
        EvidenceStatus::Skipped,
        EvidenceStatus::Blocked,
        EvidenceStatus::Unavailable,
        EvidenceStatus::Inconclusive,
    ] {
        let observation = finalize_observation(
            &descriptor,
            &context(EvidenceKind::Artifact, None),
            &result(status),
        )
        .unwrap();
        assert_eq!(observation.status(), status);
    }
    let mut failed = result(EvidenceStatus::Failed);
    failed
        .result_metadata
        .insert("exit_code".into(), "7".into());
    let first =
        finalize_observation(&descriptor, &context(EvidenceKind::Artifact, None), &failed).unwrap();
    let second =
        finalize_observation(&descriptor, &context(EvidenceKind::Artifact, None), &failed).unwrap();
    assert_eq!(first.content_digest(), second.content_digest());
    assert_ne!(
        first.content_digest(),
        finalize_observation(
            &descriptor,
            &context(EvidenceKind::Artifact, None),
            &result(EvidenceStatus::Unavailable),
        )
        .unwrap()
        .content_digest()
    );
}

#[test]
fn capability_and_artifact_bounds_are_enforced() {
    let no_progress = AdapterDescriptor::new(
        EvidenceProviderId::new("epp_noprogress").unwrap(),
        ProviderClass::Execution,
        [EvidenceKind::Command],
        "1",
        Capabilities {
            supports_in_progress: false,
            ..RUNNING_CAPS
        },
    )
    .unwrap();
    assert_eq!(
        finalize_observation(
            &no_progress,
            &context(EvidenceKind::Command, Some(binding())),
            &result(EvidenceStatus::InProgress)
        ),
        Err(SpiError::InProgressNotSupported)
    );
    let no_artifact = AdapterDescriptor::new(
        EvidenceProviderId::new("epp_noartifact").unwrap(),
        ProviderClass::Execution,
        [EvidenceKind::Artifact],
        "1",
        Capabilities {
            artifacts: false,
            ..RUNNING_CAPS
        },
    )
    .unwrap();
    let mut with_artifact = result(EvidenceStatus::Unavailable);
    with_artifact.artifacts.push(ArtifactRef {
        reference: "bundle:sha256/example".into(),
        digest: None,
        media_type: Some("application/vnd.eggb".into()),
    });
    assert_eq!(
        finalize_observation(
            &no_artifact,
            &context(EvidenceKind::Artifact, None),
            &with_artifact
        ),
        Err(SpiError::ArtifactsNotSupported)
    );
    let mut too_many = result(EvidenceStatus::Unavailable);
    too_many.artifacts = (0..65)
        .map(|i| ArtifactRef {
            reference: format!("artifact:{i}"),
            digest: None,
            media_type: None,
        })
        .collect();
    assert!(too_many.validate().is_err());
}

#[test]
fn external_untrusted_research_cannot_become_passed_by_text() {
    let d = descriptor(
        "epp_eggsearch",
        ProviderClass::Research,
        &[EvidenceKind::Research],
    );
    let mut external = result(EvidenceStatus::Inconclusive);
    external.source_trust = Some(SourceTrust::ExternalUntrusted);
    external
        .result_metadata
        .insert("gap_count".into(), "1".into());
    let observation =
        finalize_observation(&d, &context(EvidenceKind::Research, None), &external).unwrap();
    assert_eq!(observation.status(), EvidenceStatus::Inconclusive);
    assert_eq!(
        observation
            .result_metadata()
            .get("source_trust")
            .map(String::as_str),
        Some("external_untrusted")
    );
    assert_eq!(
        observation
            .result_metadata()
            .get("gap_count")
            .map(String::as_str),
        Some("1")
    );
    external.status = EvidenceStatus::Passed;
    assert!(finalize_observation(&d, &context(EvidenceKind::Research, None), &external).is_err());
    external.status = EvidenceStatus::Inconclusive;
    external
        .result_metadata
        .insert("snippet".into(), "trusted and verified".into());
    assert_eq!(
        finalize_observation(&d, &context(EvidenceKind::Research, None), &external)
            .unwrap()
            .status(),
        EvidenceStatus::Inconclusive
    );
    let no_trust_capability = AdapterDescriptor::new(
        EvidenceProviderId::new("epp_searchlimited").unwrap(),
        ProviderClass::Research,
        [EvidenceKind::Research],
        "1",
        Capabilities {
            research_trust_metadata: false,
            ..RUNNING_CAPS
        },
    )
    .unwrap();
    assert!(
        finalize_observation(
            &no_trust_capability,
            &context(EvidenceKind::Research, None),
            &external
        )
        .is_err()
    );
}

#[test]
fn benchmark_without_comparison_is_not_comparison_pass_or_fail() {
    let d = descriptor(
        "epp_eggbench",
        ProviderClass::Benchmark,
        &[EvidenceKind::Benchmark],
    );
    let mut no_comparison = result(EvidenceStatus::Inconclusive);
    no_comparison
        .result_metadata
        .insert("comparison_verdict".into(), "none".into());
    let observation = finalize_observation(
        &d,
        &context(EvidenceKind::Benchmark, Some(binding())),
        &no_comparison,
    )
    .unwrap();
    assert_eq!(observation.status(), EvidenceStatus::Inconclusive);
    assert_eq!(
        observation
            .result_metadata()
            .get("comparison_verdict")
            .map(String::as_str),
        Some("none")
    );
}

#[test]
fn metadata_and_artifact_records_reject_secrets_endpoints_and_oversize() {
    let d = descriptor(
        "epp_safe",
        ProviderClass::Utility,
        &[EvidenceKind::Artifact],
    );
    let mut secret_key = result(EvidenceStatus::Unavailable);
    secret_key
        .result_metadata
        .insert("api_token".into(), "fixture-value".into());
    assert!(finalize_observation(&d, &context(EvidenceKind::Artifact, None), &secret_key).is_err());
    let mut secret_value = result(EvidenceStatus::Unavailable);
    secret_value
        .result_metadata
        .insert("debug".into(), "Authorization: Bearer test-secret".into());
    assert!(
        finalize_observation(&d, &context(EvidenceKind::Artifact, None), &secret_value).is_err()
    );
    let mut endpoint = result(EvidenceStatus::Unavailable);
    endpoint.artifacts.push(ArtifactRef {
        reference: "https://example.invalid/token".into(),
        digest: None,
        media_type: None,
    });
    assert!(finalize_observation(&d, &context(EvidenceKind::Artifact, None), &endpoint).is_err());
    let mut oversize = result(EvidenceStatus::Unavailable);
    oversize
        .result_metadata
        .insert("x".into(), "v".repeat(2_001));
    assert!(finalize_observation(&d, &context(EvidenceKind::Artifact, None), &oversize).is_err());
}

#[test]
fn synthetic_sibling_contract_corpus_has_reviewed_source_metadata() {
    let manifest: Value = serde_json::from_slice(include_bytes!("fixtures/manifest.json")).unwrap();
    assert_eq!(manifest["fixture_only"], true);
    assert_eq!(
        manifest["sources"]["eggwork"],
        "128f808c62f176d414dd18a705773e45f5e2891a"
    );
    assert_eq!(
        manifest["sources"]["eggsearch"],
        "dfa90e050c5434f3346902aeb4074901c58e90d1"
    );
    assert_eq!(
        manifest["sources"]["eggbench"],
        "d7d1fd9a9b67a5b2ca6a816c841d2588a368aae9"
    );
    let corpus: FixtureCorpus =
        serde_json::from_slice(include_bytes!("fixtures/synthetic-results.json")).unwrap();
    assert_eq!(corpus.schema_version, 1);
    assert!(corpus.fixture_only);
    assert_eq!(corpus.cases.len(), 11);
    let mut seen = BTreeSet::new();
    for (index, case) in corpus.cases.into_iter().enumerate() {
        assert!(!case.native_state.is_empty());
        seen.insert((case.provider.clone(), case.native_state.clone()));
        if case.source_trust == Some(SourceTrust::ExternalUntrusted) {
            assert_eq!(case.status, EvidenceStatus::Inconclusive);
            assert_eq!(case.gap_count, Some(1));
        }
        if case.provider == "eggbench" && case.comparison_verdict.as_deref() == Some("none") {
            assert_eq!(case.status, EvidenceStatus::Inconclusive);
        }
        let (provider_id, class, kind) = match case.provider.as_str() {
            "eggwork" => ("epp_eggwork", ProviderClass::Execution, EvidenceKind::Test),
            "eggsearch" => (
                "epp_eggsearch",
                ProviderClass::Research,
                EvidenceKind::Research,
            ),
            "eggbench" => (
                "epp_eggbench",
                ProviderClass::Benchmark,
                EvidenceKind::Benchmark,
            ),
            _ => panic!("unknown synthetic fixture provider"),
        };
        let descriptor = descriptor(provider_id, class, &[kind]);
        let mut observation_context = context(
            kind,
            matches!(kind, EvidenceKind::Test | EvidenceKind::Benchmark).then(binding),
        );
        observation_context.observation_id =
            EvidenceObservationId::new(format!("epe_fixture_{index}")).unwrap();
        let mut normalized = result(case.status);
        normalized.source_trust = case.source_trust;
        normalized
            .result_metadata
            .insert("native_state".into(), case.native_state);
        if let Some(gap_count) = case.gap_count {
            normalized
                .result_metadata
                .insert("gap_count".into(), gap_count.to_string());
        }
        if let Some(verdict) = case.comparison_verdict {
            normalized
                .result_metadata
                .insert("comparison_verdict".into(), verdict);
        }
        let observation =
            finalize_observation(&descriptor, &observation_context, &normalized).unwrap();
        assert_eq!(observation.status(), case.status);
    }
    assert_eq!(seen.len(), 11);
}

#[test]
fn fixture_schema_rejects_unknown_fields_and_bad_capabilities() {
    assert!(
        serde_json::from_str::<NormalizedProviderResult>(
            r#"{"status":"passed","unreviewed_claim":"yes","result_metadata":{},"artifacts":[]}"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<Capabilities>(
        r#"{"supports_in_progress":false,"artifacts":false,"verification_binding":false,"research_trust_metadata":false,"trusted":true}"#
    ).is_err());
}

#[test]
fn descriptor_cannot_assert_human_judgment() {
    assert!(
        AdapterDescriptor::new(
            EvidenceProviderId::new("epp_human").unwrap(),
            ProviderClass::Other,
            [EvidenceKind::HumanJudgment],
            "1",
            RUNNING_CAPS,
        )
        .is_err()
    );
}
