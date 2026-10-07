//! Batch queries, compact projections, provider-policy reads, and shell
//! completions.

use eggplan_core::{
    AcceptanceCriterion, CriterionId, EvidenceCardinality, EvidenceKind, EvidenceObservation,
    EvidenceObservationId, EvidenceObservationInput, EvidenceProviderId, EvidenceRequirement,
    EvidenceStatus, Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus, SubjectPolicy,
    SubjectRevision, VerificationDigest,
};
use eggplan_repo::{PlanStore, RepositoryStore};
use git2::{Repository, Signature};
use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::{TempDir, tempdir};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_eggplan")
}

fn invoke(args: &[&str]) -> std::process::Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn json_ok(output: &std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["ok"], true);
    value
}

fn json_err(output: &std::process::Output) -> Value {
    assert!(!output.status.success(), "expected failure");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    value
}

/// A clean worktree that ignores the state root, so the Git subject stays
/// stable while plans are written.
fn clean_repo(dir: &TempDir) {
    Repository::init(dir.path()).unwrap();
    fs::write(dir.path().join(".gitignore"), b".eggplan/\n").unwrap();
    commit_all(dir.path());
    let output = invoke(&["init", "--state-root", &state(dir)]);
    assert!(output.status.success());
}

fn commit_all(path: &Path) {
    let repository = Repository::open(path).unwrap();
    let mut index = repository.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    let signature = Signature::now("Eggplan Test", "test@example.invalid").unwrap();
    let parent = repository
        .head()
        .ok()
        .map(|head| head.peel_to_commit().unwrap());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "baseline",
            &tree,
            &parents,
        )
        .unwrap();
}

/// Seed `count` plans; every third one is activated so status filters have
/// something to select.
fn seed(root: &Path, count: usize) -> RepositoryStore {
    let store = RepositoryStore::open(root.join(".eggplan")).unwrap();
    let subject = store.subject_source().capture().unwrap();
    let binding = VerificationDigest::new(format!("sha256:{}", "c".repeat(64))).unwrap();
    for index in 0..count {
        let id = format!("ep_batch_{index:03}");
        let item = format!("epi_batch_{index:03}");
        let criterion = format!("epc_batch_{index:03}");
        let mut plan = Plan::new(
            PlanId::new(&id).unwrap(),
            format!("batch plan {index}"),
            vec![PlanItem {
                id: PlanItemId::new(&item).unwrap(),
                position: 0,
                parent: None,
                dependencies: vec![],
                status: if index % 5 == 0 {
                    PlanItemStatus::Blocked
                } else {
                    PlanItemStatus::Pending
                },
                description: format!("item {index}"),
                criteria: vec![AcceptanceCriterion {
                    id: CriterionId::new(&criterion).unwrap(),
                    statement: "designated test passed".into(),
                    human_judgment_allowed: false,
                    requirements: vec![EvidenceRequirement {
                        description: "designated test invocation".into(),
                        kind: EvidenceKind::Test,
                        provider: None,
                        subject_policy: SubjectPolicy::Exact,
                        cardinality: EvidenceCardinality::Any,
                        min_count: 1,
                        allow_human_judgment: false,
                        expected_verification_digest: Some(binding.clone()),
                    }],
                }],
                blocker: if index % 5 == 0 {
                    Some("blocked".into())
                } else {
                    None
                },
                next_action: None,
            }],
        )
        .unwrap();
        plan.subject = Some(subject.clone());
        store.create(&plan).unwrap();
        if index % 3 == 0 {
            let mut active = plan;
            active.revision = 1;
            active.status = PlanStatus::Active;
            let activated = store.compare_and_swap(&active.id, 0, &active).unwrap();
            let observation = EvidenceObservation::finalize(EvidenceObservationInput {
                id: EvidenceObservationId::new(format!("epe_batch_{index:03}_0")).unwrap(),
                provider_id: EvidenceProviderId::new("epp_test").unwrap(),
                kind: EvidenceKind::Test,
                status: EvidenceStatus::Passed,
                subject: subject.clone(),
                observed_at_unix_ms: 11,
                invocation_ref: Some("cargo test".into()),
                verification_digest: Some(binding.clone()),
                result_metadata: Default::default(),
                artifacts: vec![],
            })
            .unwrap();
            store
                .append_observation(&activated.id, &observation)
                .unwrap();
        }
    }
    store
}

fn state(dir: &TempDir) -> String {
    dir.path().join(".eggplan").to_string_lossy().to_string()
}

fn warnings_of(envelope: &Value) -> Vec<String> {
    envelope["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect()
}

/// One active plan carrying an observation bound to a superseded subject, so
/// `check` has stale evidence with no closure record behind it.
fn seed_stale_subject(dir: &TempDir) -> String {
    let store = RepositoryStore::open(dir.path().join(".eggplan")).unwrap();
    let subject = store.subject_source().capture().unwrap();
    let superseded = SubjectRevision {
        revision: "0".repeat(40),
        ..subject.clone()
    };
    let binding = VerificationDigest::new(format!("sha256:{}", "d".repeat(64))).unwrap();
    let item = PlanItemId::new("epi_stale_000").unwrap();
    let mut plan = Plan::new(
        PlanId::new("ep_stale_000").unwrap(),
        "stale subject plan",
        vec![PlanItem {
            id: item.clone(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Pending,
            description: "stale item".into(),
            criteria: vec![AcceptanceCriterion {
                id: CriterionId::new("epc_stale_000").unwrap(),
                statement: "designated test passed".into(),
                human_judgment_allowed: false,
                requirements: vec![EvidenceRequirement {
                    description: "designated test invocation".into(),
                    kind: EvidenceKind::Test,
                    provider: None,
                    subject_policy: SubjectPolicy::Exact,
                    cardinality: EvidenceCardinality::Any,
                    min_count: 1,
                    allow_human_judgment: false,
                    expected_verification_digest: Some(binding.clone()),
                }],
            }],
            blocker: None,
            next_action: None,
        }],
    )
    .unwrap();
    plan.subject = Some(subject.clone());
    let observation = EvidenceObservation::finalize(EvidenceObservationInput {
        id: EvidenceObservationId::new("epe_stale_000_0").unwrap(),
        provider_id: EvidenceProviderId::new("epp_test").unwrap(),
        kind: EvidenceKind::Test,
        status: EvidenceStatus::Passed,
        subject: superseded,
        observed_at_unix_ms: 11,
        invocation_ref: Some("cargo test".into()),
        verification_digest: Some(binding),
        result_metadata: Default::default(),
        artifacts: vec![],
    })
    .unwrap();
    store.create(&plan).unwrap();
    store.append_observation(&plan.id, &observation).unwrap();
    plan.id.to_string()
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

#[test]
fn empty_repository_lists_nothing_without_failing() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    let root = state(&dir);
    let output = invoke(&["list", "--state-root", &root, "--json"]);
    let data = json_ok(&output);
    assert_eq!(data["data"]["plans"].as_array().unwrap().len(), 0);
    assert_eq!(data["data"]["matched"], 0);
    assert_eq!(data["data"]["returned"], 0);
    assert_eq!(data["data"]["truncated"], false);
    assert_eq!(data["data"]["next_after"], Value::Null);
}

#[test]
fn list_is_ordered_by_plan_id_and_bounded() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 12);
    let root = state(&dir);
    let data = json_ok(&invoke(&["list", "--state-root", &root, "--json"]));
    let ids: Vec<&str> = data["data"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["plan_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 12);
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "ordering must be deterministic by plan ID");

    // Compact rows carry only bounded high-value fields.
    let row = &data["data"]["plans"][0];
    let keys: Vec<&str> = row
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for expected in [
        "schema_version",
        "plan_id",
        "revision",
        "status",
        "item_count",
        "ready_item_count",
        "blocked_item_count",
        "has_closure",
        "assessment_status",
        "assessment_reason_codes",
        "assessment_reason_codes_truncated",
        "objective_preview",
        "objective_truncated",
    ] {
        assert!(keys.contains(&expected), "compact row missing {expected}");
    }
    for forbidden in [
        "items",
        "evidence",
        "blocker",
        "description",
        "result_metadata",
    ] {
        assert!(
            !keys.contains(&forbidden),
            "compact row must not carry {forbidden}"
        );
    }
    assert_eq!(row["schema_version"], 1);
}

#[test]
fn list_limit_truncates_and_reports_counts_and_cursor() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 12);
    let root = state(&dir);
    let data = json_ok(&invoke(&[
        "list",
        "--state-root",
        &root,
        "--limit",
        "5",
        "--json",
    ]));
    assert_eq!(data["data"]["returned"], 5);
    // `matched` counts matching rows inside the bounded window this call could
    // see (limit + 1), not an unbounded repository-wide total.
    assert_eq!(data["data"]["matched"], 6);
    assert_eq!(data["data"]["truncated"], true);
    assert_eq!(data["data"]["next_after"], "ep_batch_004");
}

#[test]
fn keyset_pagination_neither_duplicates_nor_omits() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 11);
    let root = state(&dir);
    let mut seen: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..5 {
        let mut args = vec!["list", "--state-root", &root, "--limit", "4", "--json"];
        if let Some(value) = &cursor {
            args.push("--after");
            args.push(value);
        }
        let data = json_ok(&invoke(&args));
        let page: Vec<String> = data["data"]["plans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["plan_id"].as_str().unwrap().to_string())
            .collect();
        if let Value::String(next) = data["data"]["next_after"].clone() {
            let next = next.as_str().to_string();
            // `--after X` returns strictly-greater IDs, so the next cursor is
            // the last ID handed out and no row is ever repeated.
            assert_eq!(
                page.last().map(String::as_str),
                Some(next.as_str()),
                "the cursor must be the last row returned, so the next page starts after it"
            );
            if let Some(previous) = &cursor {
                assert!(
                    page.first()
                        .is_none_or(|first| first.as_str() > previous.as_str()),
                    "a later page must start after the previous cursor"
                );
            }
            cursor = Some(next);
        }
        seen.extend(page);
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 11, "every plan must appear exactly once");
}

#[test]
fn status_filter_uses_canonical_values_only() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 9);
    let root = state(&dir);
    let active = json_ok(&invoke(&[
        "list",
        "--state-root",
        &root,
        "--status",
        "active",
        "--json",
    ]));
    let rows = active["data"]["plans"].as_array().unwrap();
    assert!(!rows.is_empty());
    for row in rows {
        assert_eq!(row["status"], "active");
    }
    assert_eq!(active["data"]["filter_status"], "active");

    for bad in ["Active", "ACTIVE", "pending", "closed!", "Draft"] {
        let failure = json_err(&invoke(&[
            "list",
            "--state-root",
            &root,
            "--status",
            bad,
            "--json",
        ]));
        assert_eq!(failure["error"]["code"], "usage");
    }
}

#[test]
fn list_bounds_are_enforced() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 2);
    let root = state(&dir);
    for bad in ["0", "101", "-1", "abc", ""] {
        let failure = json_err(&invoke(&[
            "list",
            "--state-root",
            &root,
            "--limit",
            bad,
            "--json",
        ]));
        assert_eq!(failure["error"]["code"], "usage", "--limit {bad:?}");
    }
    // A malformed cursor Plan ID fails rather than silently returning rows.
    let failure = json_err(&invoke(&[
        "list",
        "--state-root",
        &root,
        "--after",
        "not-a-plan",
        "--json",
    ]));
    assert_eq!(failure["error"]["code"], "invalid_id");
    // Positional arguments are not part of the list surface.
    let failure = json_err(&invoke(&[
        "list",
        "--state-root",
        &root,
        "ep_batch_000",
        "--json",
    ]));
    assert_eq!(failure["error"]["code"], "usage");
}

#[test]
fn list_uses_the_inspection_snapshot_rather_than_per_plan_reads() {
    // The repository-wide path must go through the M003a snapshot. Proved by
    // observing the snapshot's own drift failure on a single-plan selection,
    // which is only reachable through `inspection_snapshot`.
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 4);
    let root = state(&dir);
    let data = json_ok(&invoke(&["list", "--state-root", &root, "--json"]));
    assert!(
        data["data"]["repository_id"]
            .as_str()
            .unwrap()
            .starts_with("epr_"),
        "list reports the repository identity"
    );
    assert!(data["data"]["plans"].as_array().unwrap().len() >= 4);
}

// ---------------------------------------------------------------------------
// multi-ID status
// ---------------------------------------------------------------------------

#[test]
fn single_id_status_semantics_are_unchanged() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 3);
    let root = state(&dir);
    let data = json_ok(&invoke(&[
        "status",
        "ep_batch_000",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(data["data"]["total_plans"], 1);
    assert_eq!(data["data"]["truncated"], false);
    assert_eq!(data["data"]["plans"][0]["plan_id"], "ep_batch_000");
}

#[test]
fn status_accepts_a_bounded_explicit_id_set() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 6);
    let root = state(&dir);
    let data = json_ok(&invoke(&[
        "status",
        "ep_batch_005",
        "ep_batch_001",
        "--state-root",
        &root,
        "--json",
    ]));
    let ids: Vec<&str> = data["data"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["plan_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["ep_batch_001", "ep_batch_005"],
        "explicit set is deterministic"
    );
    assert_eq!(data["data"]["total_plans"], 2);
    assert_eq!(data["data"]["truncated"], false);
}

#[test]
fn status_rejects_duplicates_and_unknown_ids_deterministically() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 3);
    let root = state(&dir);
    let duplicate = json_err(&invoke(&[
        "status",
        "ep_batch_000",
        "ep_batch_000",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(duplicate["error"]["code"], "usage");
    assert!(
        duplicate["error"]["message"]
            .as_str()
            .unwrap()
            .contains("duplicate")
    );

    let missing = json_err(&invoke(&[
        "status",
        "ep_batch_000",
        "ep_absent",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(missing["error"]["code"], "plan_not_found");
    assert!(
        missing["error"]["message"]
            .as_str()
            .unwrap()
            .contains("ep_absent")
    );
}

#[test]
fn explicit_id_set_is_bounded() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 1);
    let root = state(&dir);
    let ids: Vec<String> = (0..101)
        .map(|index| format!("ep_batch_{index:03}"))
        .collect();
    let mut args = ["status", "--state-root", &root, "--json"].to_vec();
    args.extend(ids.iter().map(String::as_str));
    let failure = json_err(&invoke(&args));
    assert_eq!(failure["error"]["code"], "usage");
    assert!(
        failure["error"]["message"]
            .as_str()
            .unwrap()
            .contains("explicit plan IDs")
    );
}

// ---------------------------------------------------------------------------
// provider policy on reads
// ---------------------------------------------------------------------------

fn policy_file(dir: &TempDir) -> String {
    let path = dir.path().join("policy.json");
    fs::write(
        &path,
        br#"{"schema_version":1,"providers":[{"provider_id":"epp_test","class":"host","allowed_kinds":["test"]}]}"#,
    )
    .unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn no_policy_reports_untrusted_while_a_policy_changes_only_interpretation() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 3);
    let root = state(&dir);

    let without = json_ok(&invoke(&["list", "--state-root", &root, "--json"]));
    let with = json_ok(&invoke(&[
        "list",
        "--state-root",
        &root,
        "--provider-policy",
        &policy_file(&dir),
        "--json",
    ]));

    // Evidence is identical either way; only the assessment reading changes.
    let ids = |value: &Value| -> Vec<String> {
        value["data"]["plans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["plan_id"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(ids(&without), ids(&with));
    for (a, b) in without["data"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .zip(with["data"]["plans"].as_array().unwrap())
    {
        assert_eq!(a["item_count"], b["item_count"]);
        assert_eq!(a["revision"], b["revision"]);
        assert_eq!(a["objective_preview"], b["objective_preview"]);
    }

    // The no-policy run is explicit about having no policy rather than silent.
    let warnings: Vec<&str> = without["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert!(
        warnings
            .iter()
            .any(|value| value.contains("assessment_uses_empty_provider_registry")),
        "no-policy reads must say so, got {warnings:?}"
    );
    assert!(
        with["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|value| !value.as_str().unwrap().contains("empty_provider_registry")),
        "a supplied policy must not also warn about having none"
    );
}

#[test]
fn malformed_provider_policy_preserves_typed_diagnostics() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 2);
    let root = state(&dir);

    let bad = dir.path().join("bad.json");
    fs::write(&bad, b"{\"schema_version\":2,\"providers\":[]}").unwrap();
    let failure = json_err(&invoke(&[
        "list",
        "--state-root",
        &root,
        "--provider-policy",
        bad.to_str().unwrap(),
        "--json",
    ]));
    assert_eq!(failure["error"]["code"], "unknown_schema");

    let missing = json_err(&invoke(&[
        "list",
        "--state-root",
        &root,
        "--provider-policy",
        dir.path().join("absent.json").to_str().unwrap(),
        "--json",
    ]));
    assert_eq!(missing["error"]["code"], "input_unavailable");
}

#[test]
fn show_and_registry_render_apply_a_supplied_policy_instead_of_discarding_it() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 3);
    let root = state(&dir);

    // A declared-but-discarded `--provider-policy` reported success even for a
    // file that does not exist; both reads must now resolve it like `list`.
    let missing_policy = dir.path().join("absent.json").to_string_lossy().to_string();
    let missing = json_err(&invoke(&[
        "show",
        "ep_batch_000",
        "--state-root",
        &root,
        "--provider-policy",
        &missing_policy,
        "--json",
    ]));
    assert_eq!(
        missing["error"]["code"], "input_unavailable",
        "show must resolve a supplied policy file"
    );
    let missing = json_err(&invoke(&[
        "registry",
        "render",
        "--state-root",
        &root,
        "--provider-policy",
        &missing_policy,
        "--json",
    ]));
    assert_eq!(
        missing["error"]["code"], "input_unavailable",
        "registry render must resolve a supplied policy file"
    );

    // The registry warning must describe the invocation that actually happened:
    // a supplied policy changes the projected assessment and must not be
    // reported as "no provider policy was supplied".
    let without = json_ok(&invoke(&[
        "registry",
        "render",
        "--state-root",
        &root,
        "--json",
    ]));
    let with = json_ok(&invoke(&[
        "registry",
        "render",
        "--state-root",
        &root,
        "--provider-policy",
        &policy_file(&dir),
        "--json",
    ]));
    assert!(warnings_of(&without).contains(
        &"assessment_uses_empty_provider_registry:no provider policy was supplied".to_string()
    ));
    assert!(
        !warnings_of(&with)
            .iter()
            .any(|warning| warning.contains("empty_provider_registry")),
        "a supplied policy must not also warn about having none: {:?}",
        warnings_of(&with)
    );

    // `show` is the same read with the same policy applied.
    let shown = json_ok(&invoke(&[
        "show",
        "ep_batch_000",
        "--state-root",
        &root,
        "--provider-policy",
        &policy_file(&dir),
        "--json",
    ]));
    assert!(
        !warnings_of(&shown)
            .iter()
            .any(|warning| warning.contains("empty_provider_registry")),
        "a supplied policy must not also warn about having none"
    );
    let shown_without = json_ok(&invoke(&[
        "show",
        "ep_batch_000",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(
        warnings_of(&shown_without),
        vec!["assessment_uses_empty_provider_registry:no provider policy was supplied".to_string()],
        "show must warn when it assessed evidence without a policy"
    );
}

#[test]
fn status_warns_about_an_empty_registry_for_the_repository_wide_read() {
    // The repository-wide read is the one most likely to contain evidence, so it
    // must not be the one read that suppresses the warning.
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 3);
    let root = state(&dir);

    let wide = json_ok(&invoke(&["status", "--state-root", &root, "--json"]));
    assert!(warnings_of(&wide).contains(
        &"assessment_uses_empty_provider_registry:no provider policy was supplied".to_string()
    ));

    let one = json_ok(&invoke(&[
        "status",
        "ep_batch_000",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(
        warnings_of(&one),
        warnings_of(&wide),
        "both read paths must agree about the same repository state"
    );

    let with = json_ok(&invoke(&[
        "status",
        "--state-root",
        &root,
        "--provider-policy",
        &policy_file(&dir),
        "--json",
    ]));
    assert!(
        !warnings_of(&with)
            .iter()
            .any(|warning| warning.contains("empty_provider_registry"))
    );
}

#[test]
fn a_filtered_list_window_that_matches_nothing_still_reports_a_usable_cursor() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 12);
    let root = state(&dir);

    // No plan is `closed`, so the retained window matches nothing at all.
    let data = json_ok(&invoke(&[
        "list",
        "--status",
        "closed",
        "--limit",
        "5",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(data["data"]["matched"], 0);
    assert_eq!(data["data"]["returned"], 0);
    assert_eq!(data["data"]["truncated"], true);
    let cursor = data["data"]["next_after"]
        .as_str()
        .expect("a truncated window must hand back a cursor it can advance past");
    assert!(!cursor.is_empty());

    // Following the cursor must actually advance, or the caller is stranded.
    let next = json_ok(&invoke(&[
        "list",
        "--status",
        "closed",
        "--limit",
        "5",
        "--state-root",
        &root,
        "--after",
        cursor,
        "--json",
    ]));
    assert_eq!(next["data"]["matched"], 0);
    assert!(
        next["data"]["next_after"].as_str() != Some(cursor),
        "paging must advance past the previous cursor"
    );

    // Paging with a filter still reaches every matching row, never skipping one.
    let mut cursor: Option<String> = None;
    let mut seen: Vec<String> = Vec::new();
    loop {
        let mut args = vec![
            "list",
            "--status",
            "draft",
            "--limit",
            "2",
            "--state-root",
            &root,
            "--json",
        ];
        if let Some(value) = &cursor {
            args.push("--after");
            args.push(value);
        }
        let page = json_ok(&invoke(&args));
        for row in page["data"]["plans"].as_array().unwrap() {
            seen.push(row["plan_id"].as_str().unwrap().to_string());
        }
        if page["data"]["truncated"] != true {
            break;
        }
        cursor = Some(
            page["data"]["next_after"]
                .as_str()
                .expect("truncated pages must carry a cursor")
                .to_string(),
        );
    }
    let mut expected = seen.clone();
    expected.sort();
    expected.dedup();
    assert_eq!(seen, expected, "paging must not duplicate or omit a row");
}

#[test]
fn item_update_accepts_exactly_the_status_spellings_the_metadata_declares() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 1);
    let root = state(&dir);

    let help = invoke(&["help", "item", "--json"]);
    let advertised = json_ok(&help)["data"]["command"]
        .as_str()
        .unwrap()
        .to_string();
    let values = advertised
        .rsplit_once('[')
        .and_then(|(_, tail)| tail.split_once(']'))
        .map(|(values, _)| values.to_string())
        .expect("`item` help must advertise the closed `--status` value set");
    assert_eq!(
        values, "pending|actionable|in_progress|blocked|completed|cancelled",
        "help must advertise exactly the values the parser accepts"
    );

    // Every advertised spelling must parse, and the PascalCase spellings the
    // metadata used to advertise must be rejected rather than suggested.
    let accepted = json_ok(&invoke(&[
        "item",
        "update",
        "ep_batch_000",
        "epi_batch_000",
        "--status",
        "blocked",
        "--expected-revision",
        "1",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(accepted["data"]["revision"], 2);
    let shown = json_ok(&invoke(&[
        "show",
        "ep_batch_000",
        "--state-root",
        &root,
        "--json",
    ]));
    let item = &shown["data"]["items"][0];
    assert_eq!(item["item_id"], "epi_batch_000");
    assert_eq!(item["status"], "blocked");

    let rejected = json_err(&invoke(&[
        "item",
        "update",
        "ep_batch_000",
        "epi_batch_000",
        "--status",
        "Blocked",
        "--expected-revision",
        "2",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(rejected["error"]["code"], "invalid_status");
}

#[test]
fn help_flag_is_rendered_for_every_command() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    let root = state(&dir);

    for command in [
        "init",
        "new",
        "list",
        "show",
        "status",
        "ready",
        "graph",
        "check",
        "activate",
        "item",
        "evidence",
        "assess",
        "close",
        "closure",
        "registry",
        "markdown",
        "completions",
    ] {
        let output = invoke(&[command, "--help", "--state-root", &root, "--json"]);
        assert!(
            output.status.success(),
            "`{command} --help` must render help, got {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let data = json_ok(&output);
        let rendered = data["data"]["command"].as_str().unwrap();
        assert!(
            rendered.starts_with(&format!("eggplan {command} ")),
            "`{command} --help` must render that command's own metadata: {rendered}"
        );

        // `help COMMAND` and `COMMAND --help` are the same read.
        let via_help = json_ok(&invoke(&["help", command, "--json"]));
        assert_eq!(via_help["data"]["command"], data["data"]["command"]);
    }

    // A bare `help --help` names no command, so it renders the full usage
    // rather than inventing one.
    let bare = json_ok(&invoke(&["help", "--help", "--json"]));
    assert_eq!(bare["data"]["command"], Value::Null);
    assert!(bare["data"]["usage"].as_str().unwrap().contains("COMMAND"));
}

#[test]
fn generated_help_does_not_advertise_positional_shapes_the_cli_rejects() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 1);
    let root = state(&dir);

    // `new` accepts no positional, `check` accepts at most one.
    assert!(
        json_err(&invoke(&[
            "new",
            "ep_batch_000",
            "--state-root",
            &root,
            "--json"
        ]))["error"]["code"]
            .eq("usage")
    );
    assert!(
        json_err(&invoke(&[
            "check",
            "ep_batch_000",
            "ep_batch_001",
            "--state-root",
            &root,
            "--json"
        ]))["error"]["code"]
            .eq("usage")
    );

    let usage = json_ok(&invoke(&["help", "--json"]))["data"]["usage"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        usage.contains("new - create a plan from a definition"),
        "`new` must be documented as taking no positional: {usage}"
    );
    assert!(
        usage.contains("check [ARG] - read-only integrity and readiness check"),
        "`check` must be documented as taking at most one positional: {usage}"
    );
}

#[test]
fn check_reports_stale_only_for_a_stale_closure_record() {
    // Evidence bound to a superseded subject is an assessment classification
    // (`invalid_or_stale`); `stale` is reserved for a closure record whose
    // subject no longer matches. All four reads must agree about one state.
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    let root = state(&dir);
    let plan_id = seed_stale_subject(&dir);
    // A trusted provider is required to reach the stale branch at all, so all
    // four reads are given the same policy.
    let policy = policy_file(&dir);

    let checked = json_ok(&invoke(&[
        "check",
        &plan_id,
        "--state-root",
        &root,
        "--provider-policy",
        &policy,
        "--json",
    ]));
    let plan = &checked["data"]["plans"][0];
    assert_eq!(
        plan["state"], "invalid_or_stale",
        "evidence on a superseded subject is not a stale closure record"
    );
    assert_eq!(plan["assessment_status"], "invalid_or_stale");
    let codes: Vec<&str> = plan["reason_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert!(
        codes.contains(&"stale_subject"),
        "the reason code still carries the detail: {codes:?}"
    );

    for command in [
        vec![
            "list",
            "--state-root",
            &root,
            "--limit",
            "100",
            "--provider-policy",
            &policy,
            "--json",
        ],
        vec![
            "show",
            &plan_id,
            "--state-root",
            &root,
            "--provider-policy",
            &policy,
            "--json",
        ],
    ] {
        let data = json_ok(&invoke(&command));
        let status = if command[0] == "list" {
            data["data"]["plans"][0]["assessment_status"].clone()
        } else {
            data["data"]["plan"]["assessment_status"].clone()
        };
        assert_eq!(status, "invalid_or_stale");
    }
    let status = json_ok(&invoke(&[
        "status",
        &plan_id,
        "--state-root",
        &root,
        "--provider-policy",
        &policy,
        "--json",
    ]));
    assert_eq!(
        status["data"]["plans"][0]["assessment_status"],
        "invalid_or_stale"
    );
}

#[test]
fn non_utf8_arguments_produce_a_diagnostic_envelope_not_a_panic() {
    // `--state-root` is user-supplied, so an undecodable argument is reachable
    // input rather than an impossible one.
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let output = Command::new(bin())
            .arg("show")
            .arg(std::ffi::OsStr::from_bytes(b"ep_\xff\xfe"))
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "stable error exit code");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("panicked"),
            "must not panic: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["error"]["code"], "invalid_argument");
    }
}

#[test]
fn provider_ids_in_observations_never_self_authorize() {
    // Seeding used provider `epp_test` with class "host". Without a policy that
    // label must buy nothing, so the assessment stays untrusted.
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    seed(dir.path(), 3);
    let root = state(&dir);
    let data = json_ok(&invoke(&["list", "--state-root", &root, "--json"]));
    let assessed: Vec<&Value> = data["data"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["assessment_status"].is_string())
        .collect();
    for row in assessed {
        let status = row["assessment_status"].as_str().unwrap();
        assert_ne!(
            status, "complete",
            "an untrusted provider must not complete"
        );
        let codes: Vec<&str> = row["assessment_reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();
        assert!(
            codes
                .iter()
                .any(|code| code.contains("trust") || code.contains("missing")),
            "untrusted assessment must carry a reason code, got {codes:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// completions
// ---------------------------------------------------------------------------

#[test]
fn completion_scripts_are_generated_for_all_four_shells() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let data = json_ok(&invoke(&[
            "completions",
            shell,
            "--state-root",
            &state(&dir),
            "--json",
        ]));
        assert_eq!(data["data"]["shell"], shell);
        let script = data["data"]["script"].as_str().unwrap();
        assert!(!script.is_empty());
        assert_eq!(data["data"]["bytes"], script.len());
        for command in ["status", "list", "check", "completions", "registry"] {
            assert!(
                script.contains(command),
                "{shell} completion must mention {command}"
            );
        }
        assert!(!script.contains("definitely-not-a-command"));
        // fish spells long options without the leading dashes.
        assert!(
            script.contains("--provider-policy") || script.contains("provider-policy"),
            "{shell} completion must list options"
        );
    }
}

#[test]
fn completion_generation_is_deterministic_and_has_no_side_effects() {
    let dir = tempdir().unwrap();
    clean_repo(&dir);
    let root = state(&dir);

    // No repository scan is required to generate a script.
    let without_state = json_ok(&invoke(&["completions", "bash", "--json"]));
    let with_state = json_ok(&invoke(&[
        "completions",
        "bash",
        "--state-root",
        &root,
        "--json",
    ]));
    assert_eq!(
        without_state["data"]["script"], with_state["data"]["script"],
        "generation must not depend on repository state"
    );
    let again = json_ok(&invoke(&["completions", "bash", "--json"]));
    assert_eq!(with_state["data"]["script"], again["data"]["script"]);

    // No repository mutation.
    let fingerprint = |dir: &Path| -> Vec<(String, u64)> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            for entry in fs::read_dir(&current).unwrap() {
                let entry = entry.unwrap();
                if entry.metadata().unwrap().is_dir() {
                    stack.push(entry.path());
                } else {
                    out.push((
                        entry.path().to_string_lossy().to_string(),
                        entry.metadata().unwrap().len(),
                    ));
                }
            }
        }
        out.sort();
        out
    };
    let before = fingerprint(&dir.path().join(".eggplan"));
    let _ = invoke(&["completions", "fish", "--state-root", &root]);
    assert_eq!(fingerprint(&dir.path().join(".eggplan")), before);
}

#[test]
fn unknown_completion_shell_is_a_usage_error() {
    let failure = json_err(&invoke(&["completions", "tcsh", "--json"]));
    assert_eq!(failure["error"]["code"], "usage");
    assert!(
        failure["error"]["message"]
            .as_str()
            .unwrap()
            .contains("tcsh")
    );
    let missing = json_err(&invoke(&["completions", "--json"]));
    assert_eq!(missing["error"]["code"], "usage");
}

#[test]
fn metadata_drives_help_validation_and_completions_together() {
    let metadata = eggplan_cli::command_metadata();
    assert!(metadata.iter().any(|spec| spec.name == "list"));
    assert!(metadata.iter().any(|spec| spec.name == "completions"));

    let dir = tempdir().unwrap();
    clean_repo(&dir);
    let root = state(&dir);
    let _ = &root;

    // Every declared option is accepted by its command.
    for spec in &metadata {
        if spec.name != "list" {
            continue;
        }
        for option in &spec.options {
            // `--state-root` is already supplied by the harness, and the
            // remaining two need real file or enum arguments.
            if matches!(*option, "--status" | "--provider-policy" | "--state-root") {
                continue;
            }
            let value = if *option == "--limit" {
                "3"
            } else {
                "ep_batch_000"
            };
            let output = invoke(&["list", "--state-root", &root, option, value, "--json"]);
            assert!(
                output.status.success(),
                "declared option {option} must be accepted: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }

    // An option that is not declared for the command is still rejected.
    let rejected = json_err(&invoke(&[
        "list",
        "--state-root",
        &root,
        "--expected-revision",
        "1",
        "--json",
    ]));
    assert_eq!(rejected["error"]["code"], "usage");
}

#[test]
fn completion_generation_uses_no_process_filesystem_or_network_api() {
    // Static proof that the generator cannot shell out, walk the filesystem,
    // or open a socket.
    let source = include_str!("../src/commands.rs");
    for forbidden in [
        "std::process",
        "Command::new",
        "std::fs",
        "std::net",
        "reqwest",
        "tokio",
        "include_dir",
        "dirs::",
        "home_dir",
    ] {
        assert!(
            !source.contains(forbidden),
            "completion generator must not contain {forbidden}"
        );
    }
}
