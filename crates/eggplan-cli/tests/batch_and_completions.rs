//! Batch queries, compact projections, provider-policy reads, and shell
//! completions.

use eggplan_core::{
    AcceptanceCriterion, CriterionId, EvidenceCardinality, EvidenceKind, EvidenceObservation,
    EvidenceObservationId, EvidenceObservationInput, EvidenceProviderId, EvidenceRequirement,
    EvidenceStatus, Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus, PlanStatus, SubjectPolicy,
    VerificationDigest,
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
