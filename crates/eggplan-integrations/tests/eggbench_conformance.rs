//! Eggbench verified-bundle and comparison-evidence conformance.
//!
//! Every assertion here is an authority claim, not a shape check: the point is
//! that upstream Eggbench facts cannot become Eggplan evidence authority they
//! do not carry.

use eggplan_core::{
    EvidenceKind, EvidenceObservationId, EvidenceStatus, ProviderRegistry, SubjectRevision,
    SubjectState, VerificationDigest,
};
use eggplan_integrations::eggbench::{
    self, MAX_BUNDLE_ARTIFACT_HANDLES, MAX_DTO_BYTES, MAX_LABEL_CHARS, MAX_WARNING_CATEGORIES,
    REVIEWED_EGGBENCH_CI_RUN, REVIEWED_EGGBENCH_LIVE_RUN, REVIEWED_EGGBENCH_SHA,
};
use eggplan_integrations::{ObservationContext, SpiError};
use serde_json::Value;
use std::collections::BTreeMap;

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn context(kind: EvidenceKind, revision: &str) -> ObservationContext {
    ObservationContext {
        observation_id: EvidenceObservationId::new("epe_eggbench_test").unwrap(),
        subject: SubjectRevision {
            subject_kind: "git".into(),
            repository_id: "epr_eggbench_fixture".into(),
            revision: revision.into(),
            state: SubjectState::Clean,
            dirty_digest: None,
        },
        requested_kind: kind,
        observed_at_unix_ms: 1_780_000_000_000,
        invocation_ref: Some("eggbench:inspect".into()),
        verification_digest: Some(VerificationDigest::new(format!("sha256:{HEX_A}")).unwrap()),
        metadata: BTreeMap::new(),
    }
}

fn no_binding(kind: EvidenceKind) -> ObservationContext {
    let mut value = context(kind, "abc123");
    value.verification_digest = None;
    value
}

fn bundle_cases() -> Vec<(&'static str, Value)> {
    load_cases(include_bytes!("fixtures/eggbench-bundles.json"))
}

fn comparison_cases() -> Vec<(&'static str, Value)> {
    load_cases(include_bytes!("fixtures/eggbench-comparisons.json"))
}

/// Fixture files are arrays of `{ "case": <name>, ...dto }`; the label is not
/// part of the DTO and is stripped before parsing.
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

fn parse_bundle_case(case: &Value) -> eggbench::EggbenchBundleEvidenceV1 {
    eggbench::parse_bundle(&serde_json::to_vec(case).unwrap()).unwrap()
}

fn parse_comparison_case(case: &Value) -> eggbench::EggbenchComparisonEvidenceV1 {
    eggbench::parse_comparison(&serde_json::to_vec(case).unwrap()).unwrap()
}

fn find_bundle<'a>(cases: &'a [(&'static str, Value)], name: &str) -> &'a Value {
    &cases
        .iter()
        .find(|(case, _)| *case == name)
        .unwrap_or_else(|| panic!("missing bundle fixture {name}"))
        .1
}

fn find_comparison<'a>(cases: &'a [(&'static str, Value)], name: &str) -> &'a Value {
    &cases
        .iter()
        .find(|(case, _)| *case == name)
        .unwrap_or_else(|| panic!("missing comparison fixture {name}"))
        .1
}

fn mutate(case: &Value, path: &[&str], value: Value) -> Value {
    let mut cloned = case.clone();
    let mut cursor = &mut cloned;
    for key in &path[..path.len() - 1] {
        cursor = cursor.get_mut(*key).unwrap();
    }
    cursor
        .as_object_mut()
        .unwrap()
        .insert(path[path.len() - 1].to_string(), value);
    cloned
}

// ---------------------------------------------------------------------------
// Fixture provenance
// ---------------------------------------------------------------------------

#[test]
fn fixtures_are_pinned_to_an_exact_dual_green_upstream_revision() {
    assert_eq!(
        REVIEWED_EGGBENCH_SHA,
        "30a38251bccb5157beb68202ffe630f6253771e0"
    );
    assert_eq!(REVIEWED_EGGBENCH_CI_RUN, 37143714313);
    assert_eq!(REVIEWED_EGGBENCH_LIVE_RUN, 37143714261);
    for (_, case) in bundle_cases() {
        assert_eq!(
            case["reviewed_revision"].as_str().unwrap(),
            REVIEWED_EGGBENCH_SHA
        );
    }
    for (_, case) in comparison_cases() {
        assert_eq!(
            case["reviewed_revision"].as_str().unwrap(),
            REVIEWED_EGGBENCH_SHA
        );
    }
}

// ---------------------------------------------------------------------------
// Artifact integrity is not benchmark success
// ---------------------------------------------------------------------------

#[test]
fn artifact_integrity_never_becomes_benchmark_success() {
    let cases = bundle_cases();
    // A finalized, natively verified bundle is an integrity claim even when the
    // measured run failed or was cancelled.
    for (name, expected) in [
        ("finalized_success_no_comparison", EvidenceStatus::Passed),
        ("finalized_failed_execution", EvidenceStatus::Passed),
        ("finalized_cancelled_execution", EvidenceStatus::Passed),
        ("finalized_invalid_execution", EvidenceStatus::Passed),
        ("legacy_v1_ambiguous_inconclusive", EvidenceStatus::Passed),
    ] {
        let bundle = parse_bundle_case(find_bundle(&cases, name));
        let observed =
            eggbench::normalize_bundle(&bundle, &context(EvidenceKind::Artifact, "abc123"))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            observed.status(),
            expected,
            "artifact integrity status for {name}"
        );
    }

    // The same bundles carry no benchmark authority at all.
    for (name, expected) in [
        (
            "finalized_success_no_comparison",
            EvidenceStatus::Inconclusive,
        ),
        ("finalized_failed_execution", EvidenceStatus::Failed),
        ("finalized_cancelled_execution", EvidenceStatus::Skipped),
        ("finalized_invalid_execution", EvidenceStatus::Inconclusive),
        (
            "legacy_v1_ambiguous_inconclusive",
            EvidenceStatus::Inconclusive,
        ),
    ] {
        let bundle = parse_bundle_case(find_bundle(&cases, name));
        let observed =
            eggbench::normalize_bundle(&bundle, &context(EvidenceKind::Benchmark, "abc123"))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(observed.status(), expected, "benchmark status for {name}");
    }
}

#[test]
fn a_comparison_receipt_is_not_an_artifact_integrity_claim() {
    let cases = comparison_cases();
    let comparison = parse_comparison_case(find_comparison(
        &cases,
        "v4_pass_performance_and_correctness",
    ));
    assert_eq!(
        eggbench::normalize_comparison(&comparison, &context(EvidenceKind::Artifact, "abc123")),
        Err(SpiError::UnsupportedKind)
    );
}

#[test]
fn artifact_refs_are_run_scoped_handles_with_exact_digests() {
    let cases = bundle_cases();
    let bundle = parse_bundle_case(find_bundle(&cases, "finalized_success_no_comparison"));
    let observed =
        eggbench::normalize_bundle(&bundle, &context(EvidenceKind::Artifact, "abc123")).unwrap();
    let refs = observed.artifacts();
    assert_eq!(refs.len(), 3);
    assert_eq!(
        refs[0].reference,
        "eggbench:bundle:8cb4253e-e5f6-4b1e-8c55-e673d973d73a"
    );
    assert_eq!(
        refs[0].digest.as_deref(),
        Some("sha256:74c227f5672b577fb0775531314c183aaaa0985533a05e204caa6fa3ea875fdf")
    );
    assert_eq!(
        refs[2].reference,
        "eggbench:bundle:8cb4253e-e5f6-4b1e-8c55-e673d973d73a#trials/001/result.json"
    );
    // No temporary host path is retained as durable identity.
    for reference in refs {
        assert!(reference.reference.starts_with("eggbench:bundle:"));
        assert!(!reference.reference.contains("://"));
        assert!(!reference.reference.contains("/tmp"));
    }
}

#[test]
fn comparison_refs_use_the_bounded_receipt_digest_prefix() {
    let cases = comparison_cases();
    let comparison = parse_comparison_case(find_comparison(
        &cases,
        "v4_pass_performance_and_correctness",
    ));
    let observed =
        eggbench::normalize_comparison(&comparison, &context(EvidenceKind::Benchmark, "abc123"))
            .unwrap();
    let refs = observed.artifacts();
    assert_eq!(refs.len(), 1);
    assert_eq!(
        refs[0].reference,
        "eggbench:comparison:8cb4253e-e5f6-4b1e-8c55-e673d973d73a:1111111111111111"
    );
    assert_eq!(
        refs[0].digest.as_deref(),
        Some("sha256:1111111111111111111111111111111111111111111111111111111111111111")
    );
    // Without a standalone receipt digest there is nothing durable to reference.
    let no_receipt = parse_comparison_case(find_comparison(&cases, "v4_inconclusive"));
    assert!(
        eggbench::normalize_comparison(&no_receipt, &context(EvidenceKind::Benchmark, "abc123"))
            .unwrap()
            .artifacts()
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// Verdict semantics
// ---------------------------------------------------------------------------

#[test]
fn comparison_verdicts_are_normalized_without_collapsing_axes() {
    let cases = comparison_cases();
    for (name, expected, aggregate) in [
        (
            "v4_pass_performance_and_correctness",
            EvidenceStatus::Passed,
            "pass",
        ),
        (
            "v4_fail_performance_correctness_pass",
            EvidenceStatus::Failed,
            "fail",
        ),
        (
            "v4_inconclusive",
            EvidenceStatus::Inconclusive,
            "inconclusive",
        ),
        ("v4_invalid", EvidenceStatus::Inconclusive, "invalid"),
        (
            "v4_no_verdict_completed",
            EvidenceStatus::Inconclusive,
            "absent",
        ),
        (
            "v3_separated_performance_and_correctness",
            EvidenceStatus::Passed,
            "pass",
        ),
        (
            "v2_legacy_metric_only_aggregate",
            EvidenceStatus::Passed,
            "pass",
        ),
        (
            "v1_legacy_metric_only_aggregate",
            EvidenceStatus::Passed,
            "pass",
        ),
        ("v4_cancelled_execution", EvidenceStatus::Skipped, "pass"),
    ] {
        let comparison = parse_comparison_case(find_comparison(&cases, name));
        let observed = eggbench::normalize_comparison(
            &comparison,
            &context(EvidenceKind::Benchmark, "abc123"),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(observed.status(), expected, "status for {name}");
        match aggregate {
            "absent" => assert!(!observed.result_metadata().contains_key("aggregate_verdict")),
            verdict => assert_eq!(
                observed
                    .result_metadata()
                    .get("aggregate_verdict")
                    .map(String::as_str),
                Some(verdict),
                "aggregate verdict retained separately for {name}"
            ),
        }
    }
}

#[test]
fn performance_and_correctness_axes_are_retained_separately() {
    let cases = comparison_cases();
    let comparison = parse_comparison_case(find_comparison(
        &cases,
        "v4_fail_performance_correctness_pass",
    ));
    let observed =
        eggbench::normalize_comparison(&comparison, &context(EvidenceKind::Benchmark, "abc123"))
            .unwrap();
    // A performance regression with clean correctness stays a single Failed
    // claim; both axes remain individually readable and neither is merged away.
    assert_eq!(observed.status(), EvidenceStatus::Failed);
    assert_eq!(
        observed
            .result_metadata()
            .get("performance_verdict")
            .map(String::as_str),
        Some("fail")
    );
    assert_eq!(
        observed
            .result_metadata()
            .get("correctness_verdict")
            .map(String::as_str),
        Some("pass")
    );
    assert_eq!(
        observed
            .result_metadata()
            .get("correctness_check_count")
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn invalid_and_inconclusive_never_become_failed_or_passed() {
    let cases = comparison_cases();
    for name in ["v4_invalid", "v4_inconclusive", "v4_no_verdict_completed"] {
        let comparison = parse_comparison_case(find_comparison(&cases, name));
        let observed = eggbench::normalize_comparison(
            &comparison,
            &context(EvidenceKind::Benchmark, "abc123"),
        )
        .unwrap();
        assert_eq!(observed.status(), EvidenceStatus::Inconclusive, "{name}");
    }
}

#[test]
fn descriptive_comparison_cannot_become_passing_benchmark_evidence() {
    let cases = comparison_cases();
    let comparison = parse_comparison_case(find_comparison(
        &cases,
        "v4_descriptive_only_critical_mismatch",
    ));
    // Upstream reports `pass` here only because the host inverted
    // `comparability_match`; the critical mismatch makes it descriptive-only.
    assert_eq!(
        comparison.aggregate_verdict,
        Some(eggbench::AggregateVerdict::Pass)
    );
    let observed =
        eggbench::normalize_comparison(&comparison, &context(EvidenceKind::Benchmark, "abc123"))
            .unwrap();
    assert_eq!(observed.status(), EvidenceStatus::Inconclusive);
    assert_eq!(
        observed
            .result_metadata()
            .get("comparability_critical_mismatch")
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn legacy_inconclusive_status_is_never_disambiguated_by_inference() {
    let cases = bundle_cases();
    let bundle = parse_bundle_case(find_bundle(&cases, "legacy_v1_ambiguous_inconclusive"));
    let observed =
        eggbench::normalize_bundle(&bundle, &context(EvidenceKind::Benchmark, "abc123")).unwrap();
    assert_eq!(observed.status(), EvidenceStatus::Inconclusive);
    let metadata = observed.result_metadata();
    assert_eq!(
        metadata.get("legacy_status").map(String::as_str),
        Some("inconclusive")
    );
    assert_eq!(
        metadata.get("legacy_status_ambiguous").map(String::as_str),
        Some("true")
    );
    // The normalized upstream view said `completed`; Eggplan keeps the raw
    // legacy value rather than silently adopting the normalized meaning.
    assert_eq!(
        metadata.get("execution_status").map(String::as_str),
        Some("completed")
    );
}

// ---------------------------------------------------------------------------
// Fail-closed compatibility
// ---------------------------------------------------------------------------

#[test]
fn unknown_dto_and_upstream_versions_are_rejected() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");

    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["schema_version"], 2.into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["manifest_schema_version"], 3.into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["manifest_schema_version"], 9.into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["execution_status"], "unknown_state".into()))
                .unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["comparison_verdict"], "great".into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(
                base,
                &["subject"],
                serde_json::json!({"kind":"unknown"})
            ))
            .unwrap()
        )
        .is_err()
    );

    let comparisons = comparison_cases();
    let receipt = find_comparison(&comparisons, "v4_pass_performance_and_correctness");
    assert!(
        eggbench::parse_comparison(
            &serde_json::to_vec(&mutate(receipt, &["schema_version"], 2.into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_comparison(
            &serde_json::to_vec(&mutate(receipt, &["receipt_schema_version"], 5.into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_comparison(
            &serde_json::to_vec(&mutate(receipt, &["receipt_schema_version"], 0.into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_comparison(
            &serde_json::to_vec(&mutate(
                receipt,
                &["aggregate_verdict"],
                "descriptive".into()
            ))
            .unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_comparison(
            &serde_json::to_vec(&mutate(
                receipt,
                &["comparability"],
                serde_json::json!({"workload_match":true,"driver_match":true,"topology_match":true})
            ))
            .unwrap()
        )
        .is_err()
    );
}

#[test]
fn unknown_dto_fields_and_oversize_payloads_are_rejected() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");
    let extra = mutate(base, &["raw_manifest_json"], "\"{}\"".into());
    assert!(eggbench::parse_bundle(&serde_json::to_vec(&extra).unwrap()).is_err());
    assert!(eggbench::parse_bundle(&vec![b' '; MAX_DTO_BYTES + 1]).is_err());
    assert!(eggbench::parse_comparison(&vec![b' '; MAX_DTO_BYTES + 1]).is_err());
}

#[test]
fn malformed_digests_and_identities_are_rejected() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");
    for bad in [
        "",
        "abc",
        &"A".repeat(64),
        &format!("{HEX_A}f"),
        &"g".repeat(64),
    ] {
        assert!(
            eggbench::parse_bundle(
                &serde_json::to_vec(&mutate(base, &["manifest_sha256"], bad.into())).unwrap()
            )
            .is_err(),
            "manifest digest {bad:?} must be rejected"
        );
    }
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["reviewed_revision"], "not-a-sha".into())).unwrap()
        )
        .is_err()
    );
    assert!(
        eggbench::parse_bundle(&serde_json::to_vec(&mutate(base, &["run_id"], "".into())).unwrap())
            .is_err()
    );
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["run_id"], "bad id".into())).unwrap()
        )
        .is_err()
    );
}

#[test]
fn a_non_finalized_bundle_is_not_admissible_evidence() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["finalized"], false.into())).unwrap()
        )
        .is_err()
    );
}

#[test]
fn legacy_manifest_requires_an_explicit_legacy_status() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "legacy_v1_ambiguous_inconclusive");
    // A v1 manifest without its overloaded status must not be reinterpreted.
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["legacy_status"], Value::Null)).unwrap()
        )
        .is_err()
    );
    // A v2 manifest must not carry one.
    let current = find_bundle(&cases, "finalized_success_no_comparison");
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(current, &["legacy_status"], "inconclusive".into()))
                .unwrap()
        )
        .is_err()
    );
}

#[test]
fn legacy_receipts_cannot_carry_performance_or_correctness_sections() {
    let cases = comparison_cases();
    let legacy = find_comparison(&cases, "v1_legacy_metric_only_aggregate");
    let with_performance = mutate(legacy, &["performance_verdict"], "pass".into());
    assert!(eggbench::parse_comparison(&serde_json::to_vec(&with_performance).unwrap()).is_err());
    let with_correctness = mutate(
        legacy,
        &["correctness"],
        serde_json::json!({"policy_id":"eggbench.security-correctness.v1","check_count":1,"aggregate_verdict":"pass"}),
    );
    assert!(eggbench::parse_comparison(&serde_json::to_vec(&with_correctness).unwrap()).is_err());
}

#[test]
fn duplicate_artifact_handles_are_rejected() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");
    let artifacts = base["artifacts"].as_array().unwrap();
    let duplicated = serde_json::json!([artifacts[0], artifacts[0]]);
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["artifacts"], duplicated)).unwrap()
        )
        .is_err()
    );
}

#[test]
fn duplicate_drivers_and_warning_categories_are_rejected() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");
    let duplicate_drivers =
        serde_json::json!([{"name":"http","version":"0.1.0"},{"name":"http","version":null}]);
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["drivers"], duplicate_drivers)).unwrap()
        )
        .is_err()
    );
    let comparisons = comparison_cases();
    let receipt = find_comparison(&comparisons, "v4_fail_performance_correctness_pass");
    let duplicate_warnings = serde_json::json!(["slow_runner", "slow_runner"]);
    assert!(
        eggbench::parse_comparison(
            &serde_json::to_vec(&mutate(
                receipt,
                &["warning_categories"],
                duplicate_warnings
            ))
            .unwrap()
        )
        .is_err()
    );
}

#[test]
fn selection_and_count_bounds_are_enforced() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");
    let handle = base["artifacts"][0].clone();
    let mut many = Vec::new();
    for index in 0..=MAX_BUNDLE_ARTIFACT_HANDLES {
        let mut handle = handle.clone();
        handle["path"] = Value::String(format!("extra/{index}.json"));
        many.push(handle);
    }
    assert!(
        eggbench::parse_bundle(
            &serde_json::to_vec(&mutate(base, &["artifacts"], Value::Array(many.clone()))).unwrap()
        )
        .is_err()
    );
    // Declared count must cover the selection.
    let many = Value::Array(many);
    assert!(
        eggbench::parse_bundle(&serde_json::to_vec(&mutate(base, &["artifacts"], many)).unwrap())
            .is_err()
    );
    let comparisons = comparison_cases();
    let receipt = find_comparison(&comparisons, "v4_fail_performance_correctness_pass");
    let too_many: Vec<Value> = (0..=MAX_WARNING_CATEGORIES)
        .map(|index| Value::String(format!("warning_{index}")))
        .collect();
    assert!(
        eggbench::parse_comparison(
            &serde_json::to_vec(&mutate(
                receipt,
                &["warning_categories"],
                Value::Array(too_many)
            ))
            .unwrap()
        )
        .is_err()
    );
}

#[test]
fn escaping_artifact_paths_are_rejected() {
    let cases = bundle_cases();
    let base = find_bundle(&cases, "finalized_success_no_comparison");
    for bad in [
        "/etc/passwd",
        "../escape.json",
        "nested/../../escape.json",
        "back\\slash.json",
        "double//slash.json",
    ] {
        let broken = mutate(base, &["artifacts"], base["artifacts"].clone());
        let mut broken = broken;
        broken["artifacts"][0]["path"] = Value::String(bad.into());
        assert!(
            eggbench::parse_bundle(&serde_json::to_vec(&broken).unwrap()).is_err(),
            "artifact path {bad:?} must be rejected"
        );
    }
}

// ---------------------------------------------------------------------------
// Subject and identity agreement
// ---------------------------------------------------------------------------

#[test]
fn subject_revision_disagreement_is_rejected_for_git_subjects() {
    let cases = bundle_cases();
    let agreeing = parse_bundle_case(find_bundle(&cases, "finalized_failed_execution"));
    assert!(
        eggbench::normalize_bundle(&agreeing, &context(EvidenceKind::Benchmark, "abc123")).is_ok()
    );
    assert_eq!(
        eggbench::normalize_bundle(&agreeing, &context(EvidenceKind::Benchmark, "def456")),
        Err(SpiError::Invalid(
            "Eggbench subject revision disagrees with the Eggplan Git subject"
        ))
    );

    let comparisons = comparison_cases();
    let receipt = parse_comparison_case(find_comparison(&comparisons, "v4_cancelled_execution"));
    assert!(
        eggbench::normalize_comparison(&receipt, &context(EvidenceKind::Benchmark, "abc123"))
            .is_ok()
    );
    assert!(
        eggbench::normalize_comparison(&receipt, &context(EvidenceKind::Benchmark, "other"))
            .is_err()
    );
}

#[test]
fn an_eggbench_hint_is_never_translated_into_a_eggplan_subject() {
    let cases = bundle_cases();
    let bundle = parse_bundle_case(find_bundle(&cases, "finalized_failed_execution"));
    // A non-Git Eggplan subject is not compared against a Git revision hint,
    // and the hint stays provenance metadata rather than identity.
    let mut non_git = context(EvidenceKind::Artifact, "irrelevant");
    non_git.subject.subject_kind = "path".into();
    non_git.subject.repository_id = "epr_not_git".into();
    let observed = eggbench::normalize_bundle(&bundle, &non_git).unwrap();
    assert_eq!(
        observed
            .result_metadata()
            .get("upstream_subject_revision_hint")
            .map(String::as_str),
        Some("abc123")
    );
    assert_eq!(
        observed
            .result_metadata()
            .get("subject_kind")
            .map(String::as_str),
        Some("managed_command")
    );
}

#[test]
fn candidate_and_baseline_identities_must_differ() {
    let cases = comparison_cases();
    let base = find_comparison(&cases, "v4_pass_performance_and_correctness");
    let same_run = mutate(
        base,
        &["baseline", "run_id"],
        base["candidate"]["run_id"].clone(),
    );
    assert!(eggbench::parse_comparison(&serde_json::to_vec(&same_run).unwrap()).is_err());
    let same_digest = mutate(
        base,
        &["baseline", "manifest_sha256"],
        base["candidate"]["manifest_sha256"].clone(),
    );
    assert!(eggbench::parse_comparison(&serde_json::to_vec(&same_digest).unwrap()).is_err());
}

#[test]
fn a_comparison_candidate_must_match_the_verified_bundle() {
    let bundles = bundle_cases();
    let comparisons = comparison_cases();
    let bundle = parse_bundle_case(find_bundle(&bundles, "finalized_success_no_comparison"));
    let agreeing = parse_comparison_case(find_comparison(
        &comparisons,
        "v4_pass_performance_and_correctness",
    ));
    assert!(eggbench::assert_same_bundle(&agreeing, &bundle).is_ok());
    let other = parse_comparison_case(find_comparison(&comparisons, "v4_invalid"));
    assert!(eggbench::assert_same_bundle(&other, &bundle).is_err());
}

// ---------------------------------------------------------------------------
// Verification binding and trust
// ---------------------------------------------------------------------------

#[test]
fn benchmark_evidence_requires_host_verification_binding() {
    let cases = bundle_cases();
    let bundle = parse_bundle_case(find_bundle(&cases, "embedded_comparison_pass"));
    assert_eq!(
        eggbench::normalize_bundle(&bundle, &no_binding(EvidenceKind::Benchmark)),
        Err(SpiError::MissingVerificationBinding)
    );
    let comparisons = comparison_cases();
    let receipt = parse_comparison_case(find_comparison(
        &comparisons,
        "v4_pass_performance_and_correctness",
    ));
    assert_eq!(
        eggbench::normalize_comparison(&receipt, &no_binding(EvidenceKind::Benchmark)),
        Err(SpiError::MissingVerificationBinding)
    );
    // The digest is never derived from run id, digest, or policy by the adapter.
    let observed =
        eggbench::normalize_comparison(&receipt, &context(EvidenceKind::Benchmark, "abc123"))
            .unwrap();
    assert_eq!(
        observed.verification_digest().map(|value| value.as_str()),
        Some(format!("sha256:{HEX_A}").as_str())
    );
}

#[test]
fn provider_identity_is_fixed_and_trust_is_host_controlled() {
    let descriptor = eggbench::descriptor();
    assert_eq!(descriptor.provider_id.as_str(), "epp_eggbench");
    let core_descriptor = descriptor.provider_descriptor().unwrap();
    assert_eq!(core_descriptor.class(), "benchmark");
    let kinds = core_descriptor.allowed_kinds();
    assert!(kinds.contains(&EvidenceKind::Artifact));
    assert!(kinds.contains(&EvidenceKind::Benchmark));
    // Attestation verification is explicitly deferred to Interoperability M001.
    assert!(!kinds.contains(&EvidenceKind::Attestation));
    assert!(descriptor.allowed_kinds.len() == 2);

    // The adapter hands back a descriptor; it never enrolls it.
    let registry = ProviderRegistry::default();
    assert!(registry.get(&descriptor.provider_id).is_none());
}

#[test]
fn the_adapter_cannot_enroll_trust_or_acquire_evidence() {
    let source = include_str!("../src/eggbench.rs");
    for forbidden in [
        "register_trusted",
        "ProviderRegistry",
        "std::process",
        "Command::new",
        "std::fs",
        "reqwest",
        "tokio",
        "http://",
        "https://",
    ] {
        assert!(
            !source.contains(forbidden),
            "Eggbench adapter must not contain {forbidden}"
        );
    }
}

#[test]
fn arbitrary_native_prose_is_absent_from_serialized_observations() {
    let cases = comparison_cases();
    let comparison = parse_comparison_case(find_comparison(
        &cases,
        "v4_fail_performance_correctness_pass",
    ));
    let observed =
        eggbench::normalize_comparison(&comparison, &context(EvidenceKind::Benchmark, "abc123"))
            .unwrap();
    let serialized = serde_json::to_string(&observed).unwrap();
    // Warning `detail` prose is never retained; only stable categories are.
    assert!(serialized.contains("slow_runner"));
    assert!(!serialized.contains("detail"));
    for (_, case) in comparison_cases() {
        let observation = eggbench::normalize_comparison(
            &parse_comparison_case(&case),
            &context(EvidenceKind::Benchmark, "abc123"),
        )
        .unwrap();
        let json = serde_json::to_string(&observation).unwrap();
        assert!(!json.contains("://"), "no endpoint may be serialized");
        assert!(!json.contains("Bearer"), "no credential may be serialized");
    }
}

#[test]
fn warning_categories_must_be_stable_snake_case_labels() {
    let cases = comparison_cases();
    let base = find_comparison(&cases, "v4_fail_performance_correctness_pass");
    for bad in [
        serde_json::json!(["Slow Runner"]),
        serde_json::json!(["slow-runner"]),
        serde_json::json!(["https://example.com/x"]),
        serde_json::json!([""]),
    ] {
        let rendered = bad.to_string();
        assert!(
            eggbench::parse_comparison(
                &serde_json::to_vec(&mutate(base, &["warning_categories"], bad)).unwrap()
            )
            .is_err(),
            "warning category {rendered} must be rejected"
        );
    }
}

#[test]
fn unsupported_kinds_are_rejected_in_both_directions() {
    let cases = bundle_cases();
    let bundle = parse_bundle_case(find_bundle(&cases, "finalized_success_no_comparison"));
    for kind in [
        EvidenceKind::Test,
        EvidenceKind::Command,
        EvidenceKind::Research,
        EvidenceKind::Attestation,
        EvidenceKind::HumanJudgment,
        EvidenceKind::Revision,
    ] {
        assert_eq!(
            eggbench::normalize_bundle(&bundle, &context(kind, "abc123")),
            Err(SpiError::UnsupportedKind),
            "{kind:?} must not be produced by the Eggbench adapter"
        );
    }
}

#[test]
fn a_custom_artifact_role_label_is_bounded_not_dropped() {
    let cases = bundle_cases();
    let bundle = parse_bundle_case(find_bundle(&cases, "custom_role_label_artifact"));
    let handle = &bundle.artifacts[0];
    assert_eq!(eggbench::artifact_role_label(&handle.role), "other");
    assert!(
        eggbench::normalize_bundle(&bundle, &context(EvidenceKind::Artifact, "abc123")).is_ok()
    );

    // A bounded custom role label is retained, not silently dropped.
    let cases = bundle_cases();
    let base = find_bundle(&cases, "custom_role_label_artifact");
    let digest = base["artifacts"][0]["sha256"].clone();
    for bad in [
        "a".repeat(MAX_LABEL_CHARS + 1),
        "role://host/path".to_string(),
        String::from("with\u{0}nul"),
    ] {
        let broken = mutate(
            base,
            &["artifacts"],
            serde_json::json!([{
                "path": "extra/custom.bin",
                "role": {"kind": "other", "label": bad},
                "media_type": null,
                "byte_size": 10,
                "sha256": digest,
            }]),
        );
        assert!(
            eggbench::parse_bundle(&serde_json::to_vec(&broken).unwrap()).is_err(),
            "role label {bad:?} must be rejected"
        );
    }
}
